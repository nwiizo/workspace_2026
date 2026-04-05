use crate::analysis::parser::{module_path_from_file, normalize_type_name};
use crate::types::{ArgUsage, CallArg, CallSite, SpanInfo};
use std::path::{Path, PathBuf};
use syn::visit::Visit;
use syn::{Expr, ExprCall, ExprMethodCall, File, ImplItem, Item, ItemFn, ItemImpl};

/// Extract call sites from a parsed file.
pub fn extract_call_sites(path: &Path, ast: &File) -> Vec<CallSite> {
    let mut visitor = CallVisitor {
        file_path: path.to_path_buf(),
        module_path: module_path_from_file(path),
        current_impl: None,
        current_fn: String::new(),
        call_sites: Vec::new(),
    };
    visitor.visit_file(ast);
    visitor.call_sites
}

struct CallVisitor {
    file_path: PathBuf,
    module_path: String,
    current_impl: Option<String>,
    current_fn: String,
    call_sites: Vec<CallSite>,
}

impl CallVisitor {
    fn build_qualified_name(&self, ident: &str) -> String {
        if self.module_path.is_empty() {
            ident.to_string()
        } else if let Some(ref impl_target) = self.current_impl {
            format!("{}::{}::{}", self.module_path, impl_target, ident)
        } else {
            format!("{}::{}", self.module_path, ident)
        }
    }

    fn analyze_arg_usage(&self, expr: &Expr) -> (String, ArgUsage) {
        let text = quote::quote!(#expr).to_string();
        match expr {
            // `&x` or `&mut x`
            Expr::Reference(r) => {
                if r.mutability.is_some() {
                    (text, ArgUsage::BorrowMut)
                } else {
                    (text, ArgUsage::Borrow)
                }
            }
            // `x.clone()`
            Expr::MethodCall(mc) if mc.method == "clone" && mc.args.is_empty() => {
                (text, ArgUsage::Clone)
            }
            _ => (text, ArgUsage::Move),
        }
    }

    fn callee_name_from_expr(&self, func: &Expr) -> Option<String> {
        match func {
            Expr::Path(p) => {
                let segments: Vec<_> = p
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect();
                Some(segments.join("::"))
            }
            _ => None,
        }
    }

    fn record_call(&mut self, callee: String, args: Vec<CallArg>, line: usize, col: usize) {
        self.call_sites.push(CallSite {
            caller: self.current_fn.clone(),
            callee,
            args,
            span: SpanInfo {
                file: self.file_path.clone(),
                line,
                col,
            },
        });
    }
}

impl<'ast> Visit<'ast> for CallVisitor {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        let prev = self.current_fn.clone();
        self.current_fn = self.build_qualified_name(&node.sig.ident.to_string());
        syn::visit::visit_item_fn(self, node);
        self.current_fn = prev;
    }

    fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
        let self_ty = &node.self_ty;
        let impl_name = normalize_type_name(&quote::quote!(#self_ty).to_string());
        let prev_impl = self.current_impl.take();
        self.current_impl = Some(impl_name);

        for item in &node.items {
            if let ImplItem::Fn(method) = item {
                let prev_fn = self.current_fn.clone();
                self.current_fn = self.build_qualified_name(&method.sig.ident.to_string());
                syn::visit::visit_impl_item_fn(self, method);
                self.current_fn = prev_fn;
            }
        }

        self.current_impl = prev_impl;
    }

    fn visit_item(&mut self, node: &'ast Item) {
        // Impl blocks are handled by visit_item_impl.
        if let Item::Impl(imp) = node {
            self.visit_item_impl(imp);
        } else {
            syn::visit::visit_item(self, node);
        }
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Some(callee) = self.callee_name_from_expr(&node.func) {
            let args: Vec<CallArg> = node
                .args
                .iter()
                .map(|a| {
                    let (expr, usage) = self.analyze_arg_usage(a);
                    CallArg { expr, usage }
                })
                .collect();

            self.record_call(
                callee,
                args,
                node.paren_token.span.open().start().line,
                node.paren_token.span.open().start().column,
            );
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let callee = node.method.to_string();
        let mut args: Vec<CallArg> = Vec::new();

        // The receiver is the first implicit argument.
        let (recv_text, recv_usage) = self.analyze_arg_usage(&node.receiver);
        args.push(CallArg {
            expr: recv_text,
            usage: recv_usage,
        });

        for a in &node.args {
            let (expr, usage) = self.analyze_arg_usage(a);
            args.push(CallArg { expr, usage });
        }

        self.record_call(
            callee,
            args,
            node.method.span().start().line,
            node.method.span().start().column,
        );
        syn::visit::visit_expr_method_call(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_calls(source: &str) -> Vec<CallSite> {
        let ast: File = syn::parse_str(source).unwrap();
        let path = PathBuf::from("test.rs");
        extract_call_sites(&path, &ast)
    }

    #[test]
    fn function_call_with_borrow() {
        let calls = parse_calls(
            r#"
            fn caller() {
                process(&config);
            }
            "#,
        );
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].callee, "process");
        assert_eq!(calls[0].args[0].usage, ArgUsage::Borrow);
    }

    #[test]
    fn function_call_with_clone() {
        let calls = parse_calls(
            r#"
            fn caller() {
                process(data.clone());
            }
            "#,
        );
        // 2 calls: data.clone() (method call) + process(...) (function call)
        assert_eq!(calls.len(), 2);
        let process_call = calls.iter().find(|c| c.callee == "process").unwrap();
        assert_eq!(process_call.args[0].usage, ArgUsage::Clone);
    }

    #[test]
    fn method_call() {
        let calls = parse_calls(
            r#"
            fn caller() {
                obj.process(&mut data);
            }
            "#,
        );
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].callee, "process");
        assert_eq!(calls[0].args.len(), 2);
        assert_eq!(calls[0].args[1].usage, ArgUsage::BorrowMut);
    }

    #[test]
    fn move_by_default() {
        let calls = parse_calls(
            r#"
            fn caller() {
                consume(value);
            }
            "#,
        );
        assert_eq!(calls[0].args[0].usage, ArgUsage::Move);
    }

    #[test]
    fn caller_is_qualified() {
        let calls = parse_calls(
            r#"
            fn my_func() {
                process(&data);
            }
            "#,
        );
        // file is "test.rs" → module_path = "test"
        assert_eq!(calls[0].caller, "test::my_func");
    }

    #[test]
    fn method_caller_is_qualified() {
        let calls = parse_calls(
            r#"
            struct Svc;
            impl Svc {
                fn handle(&self) {
                    process(&self.data);
                }
            }
            "#,
        );
        let process_call = calls.iter().find(|c| c.callee == "process").unwrap();
        assert_eq!(process_call.caller, "test::Svc::handle");
    }
}
