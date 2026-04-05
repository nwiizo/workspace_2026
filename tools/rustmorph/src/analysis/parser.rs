use crate::analysis::ownership::analyze_type;
use crate::types::{FunctionSignature, OwnershipKind, ParamInfo, SpanInfo, TypeInfo};
use std::path::Path;
use syn::visit::Visit;
use syn::{File, FnArg, ImplItem, Item, ItemFn, ItemImpl, Pat, Signature};

/// Result of parsing a single file.
#[derive(Debug, Default)]
pub struct FileParseResult {
    pub functions: Vec<FunctionSignature>,
}

/// Parse an already-parsed AST and extract function signatures.
/// Avoids double file read when the caller already has the AST.
pub(crate) fn parse_ast(path: &Path, module_path: &str, ast: &File) -> FileParseResult {
    let mut visitor = FnVisitor {
        file_path: path.to_path_buf(),
        module_path: module_path.to_string(),
        current_impl: None,
        result: FileParseResult::default(),
    };
    visitor.visit_file(ast);
    visitor.result
}

/// Normalize type names produced by `quote!` — remove extra spaces around `<`, `>`, `'`.
pub(crate) fn normalize_type_name(name: &str) -> String {
    name.replace(" < ", "<")
        .replace("< ", "<")
        .replace(" >", ">")
        .replace(" ,", ",")
        .replace(", ", ",")
}

/// Derive a module path from a file path (heuristic).
pub(crate) fn module_path_from_file(path: &Path) -> String {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    if stem == "mod" || stem == "lib" || stem == "main" {
        path.parent()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default()
    } else {
        stem.to_string()
    }
}

struct FnVisitor {
    file_path: std::path::PathBuf,
    module_path: String,
    current_impl: Option<String>,
    result: FileParseResult,
}

impl FnVisitor {
    fn extract_signature(&self, sig: &Signature) -> FunctionSignature {
        let short_name = sig.ident.to_string();
        let name = if self.module_path.is_empty() {
            short_name.clone()
        } else if let Some(ref impl_target) = self.current_impl {
            format!("{}::{}::{}", self.module_path, impl_target, short_name)
        } else {
            format!("{}::{}", self.module_path, short_name)
        };

        let params: Vec<ParamInfo> = sig
            .inputs
            .iter()
            .map(|arg| self.extract_param(arg))
            .collect();

        let return_type = match &sig.output {
            syn::ReturnType::Default => None,
            syn::ReturnType::Type(_, ty) => Some(analyze_type(ty)),
        };

        FunctionSignature {
            name,
            short_name,
            impl_target: self.current_impl.clone(),
            params,
            return_type,
            span: SpanInfo {
                file: self.file_path.clone(),
                line: sig.ident.span().start().line,
                col: sig.ident.span().start().column,
            },
        }
    }

    fn extract_param(&self, arg: &FnArg) -> ParamInfo {
        match arg {
            FnArg::Receiver(recv) => {
                let (ownership, raw) = if recv.reference.is_some() {
                    if recv.mutability.is_some() {
                        (OwnershipKind::MutRef, "&mut Self".to_string())
                    } else {
                        (OwnershipKind::Ref, "&Self".to_string())
                    }
                } else {
                    (OwnershipKind::Owned, "Self".to_string())
                };
                ParamInfo {
                    name: "self".to_string(),
                    type_info: TypeInfo {
                        ownership,
                        raw: raw.clone(),
                        inner: "Self".to_string(),
                        lifetime: recv
                            .reference
                            .as_ref()
                            .and_then(|r| r.1.as_ref())
                            .map(|lt| lt.to_string()),
                        is_generic: false,
                    },
                    span: SpanInfo {
                        file: self.file_path.clone(),
                        line: recv.self_token.span.start().line,
                        col: recv.self_token.span.start().column,
                    },
                }
            }
            FnArg::Typed(pat_type) => {
                let name = match pat_type.pat.as_ref() {
                    Pat::Ident(pi) => pi.ident.to_string(),
                    _ => "_".to_string(),
                };
                let type_info = analyze_type(&pat_type.ty);
                ParamInfo {
                    name,
                    type_info,
                    span: SpanInfo {
                        file: self.file_path.clone(),
                        line: pat_type.pat.span().start().line,
                        col: pat_type.pat.span().start().column,
                    },
                }
            }
        }
    }
}

impl<'ast> Visit<'ast> for FnVisitor {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        let sig = self.extract_signature(&node.sig);
        self.result.functions.push(sig);
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
        let self_ty = &node.self_ty;
        let impl_name = normalize_type_name(&quote::quote!(#self_ty).to_string());
        self.current_impl = Some(impl_name);
        for item in &node.items {
            if let ImplItem::Fn(method) = item {
                let sig = self.extract_signature(&method.sig);
                self.result.functions.push(sig);
            }
        }
        self.current_impl = None;
    }

    fn visit_item(&mut self, node: &'ast Item) {
        if let Item::Impl(imp) = node {
            self.visit_item_impl(imp);
        } else {
            syn::visit::visit_item(self, node);
        }
    }
}

/// Extract a `use syn::Pat` span for convenience.
trait SpanExt {
    fn span(&self) -> proc_macro2::Span;
}

impl SpanExt for Pat {
    fn span(&self) -> proc_macro2::Span {
        match self {
            Pat::Ident(pi) => pi.ident.span(),
            _ => proc_macro2::Span::call_site(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn parse_source(source: &str) -> FileParseResult {
        let mut f = NamedTempFile::with_suffix(".rs").unwrap();
        f.write_all(source.as_bytes()).unwrap();
        let ast = syn::parse_file(source).unwrap();
        let module_path = module_path_from_file(f.path());
        parse_ast(f.path(), &module_path, &ast)
    }

    #[test]
    fn free_function() {
        let result = parse_source("fn process(data: &Config, count: usize) -> String { todo!() }");
        assert_eq!(result.functions.len(), 1);

        let sig = &result.functions[0];
        assert_eq!(sig.short_name, "process");
        assert_eq!(sig.params.len(), 2);
        assert_eq!(
            sig.params[0].type_info.ownership,
            crate::types::OwnershipKind::Ref
        );
        assert_eq!(
            sig.params[1].type_info.ownership,
            crate::types::OwnershipKind::Owned
        );
        assert!(sig.return_type.is_some());
    }

    #[test]
    fn method_with_self() {
        let result = parse_source(
            r#"
            struct Foo;
            impl Foo {
                fn bar(&self, x: &mut Vec<u8>) {}
                fn consume(self) {}
            }
            "#,
        );
        assert_eq!(result.functions.len(), 2);
        assert_eq!(result.functions[0].short_name, "bar");
        assert!(result.functions[0].impl_target.is_some());
        assert_eq!(
            result.functions[0].params[0].type_info.ownership,
            crate::types::OwnershipKind::Ref
        );
        assert_eq!(
            result.functions[0].params[1].type_info.ownership,
            crate::types::OwnershipKind::MutRef
        );
        assert_eq!(
            result.functions[1].params[0].type_info.ownership,
            crate::types::OwnershipKind::Owned
        );
    }
}
