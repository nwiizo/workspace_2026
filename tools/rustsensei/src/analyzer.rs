use crate::model::{
    AnalysisResult, Lang, MemoryLocation, MemoryRegion, OwnershipEvent, Step, VariableState,
    VariableStatus,
};
use std::collections::HashMap;
use syn::{Expr, Item, Stmt};

/// Pick text by language. `ja` first, `en` second.
macro_rules! t {
    ($lang:expr, $ja:expr, $en:expr) => {
        match $lang {
            Lang::Ja => $ja,
            Lang::En => $en,
        }
    };
}

/// Types that implement Copy and don't trigger move semantics.
const COPY_TYPES: &[&str] = &[
    "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize", "f32",
    "f64", "bool", "char",
];

/// Internal state of a tracked variable.
#[derive(Debug, Clone)]
struct VarInfo {
    type_name: String,
    status: VariableStatus,
    memory: MemoryLocation,
    value_hint: String,
    is_copy: bool,
    borrowed_by: Vec<String>,
    borrows_from: Option<String>,
}

/// Analyzer that walks through statements and produces ownership steps.
struct OwnershipAnalyzer {
    steps: Vec<Step>,
    variables: HashMap<String, VarInfo>,
    /// Stack of scope variable names (for drop ordering).
    scopes: Vec<Vec<String>>,
    lang: Lang,
}

impl OwnershipAnalyzer {
    fn new(lang: Lang) -> Self {
        Self {
            steps: Vec::new(),
            variables: HashMap::new(),
            lang,
            scopes: vec![Vec::new()],
        }
    }

    fn current_scope_mut(&mut self) -> &mut Vec<String> {
        self.scopes.last_mut().expect("at least one scope")
    }

    fn snapshot_variables(&self) -> Vec<VariableState> {
        self.variables
            .iter()
            .filter(|(_, info)| info.status != VariableStatus::Dropped)
            .map(|(name, info)| VariableState {
                name: name.clone(),
                type_name: info.type_name.clone(),
                status: info.status,
                memory: info.memory,
                value_hint: info.value_hint.clone(),
                borrowed_by: info.borrowed_by.clone(),
                borrows_from: info.borrows_from.clone(),
            })
            .collect()
    }

    fn snapshot_memory(&self) -> Vec<MemoryRegion> {
        self.variables
            .iter()
            .filter(|(_, info)| {
                info.memory == MemoryLocation::Heap
                    && info.status != VariableStatus::Moved
                    && info.status != VariableStatus::Dropped
            })
            .map(|(name, info)| MemoryRegion {
                address: self.alloc_addr_for(name),
                content: info.value_hint.clone(),
                owner: name.clone(),
                refs: info.borrowed_by.clone(),
            })
            .collect()
    }

    fn alloc_addr_for(&self, _name: &str) -> String {
        // Simplified: deterministic address based on variable count
        format!("0x{:04x}", 0x2000 + self.variables.len() * 0x100)
    }

    fn emit_step(&mut self, line: usize, description: String, event: OwnershipEvent) {
        let index = self.steps.len();
        let variables = self.snapshot_variables();
        let memory = self.snapshot_memory();
        self.steps.push(Step {
            index,
            source_line: line,
            description,
            event,
            variables,
            memory,
        });
    }

    fn is_copy_type(type_name: &str) -> bool {
        COPY_TYPES.contains(&type_name)
    }

    fn is_heap_type(type_name: &str) -> bool {
        matches!(
            type_name,
            "String" | "Vec" | "Box" | "HashMap" | "HashSet" | "BTreeMap" | "BTreeSet"
        )
    }

    fn infer_type_from_expr(&self, expr: &Expr) -> (String, String, bool) {
        // Returns (type_name, value_hint, is_copy)
        match expr {
            Expr::Lit(lit) => match &lit.lit {
                syn::Lit::Int(i) => ("i32".to_string(), i.to_string(), true),
                syn::Lit::Float(f) => ("f64".to_string(), f.to_string(), true),
                syn::Lit::Str(s) => ("&str".to_string(), format!("\"{}\"", s.value()), true),
                syn::Lit::Bool(b) => ("bool".to_string(), b.value.to_string(), true),
                syn::Lit::Char(c) => ("char".to_string(), format!("'{}'", c.value()), true),
                _ => ("unknown".to_string(), "?".to_string(), false),
            },
            Expr::Call(call) => self.infer_from_call(call),
            Expr::MethodCall(mc) => self.infer_from_method_call(mc),
            Expr::Reference(r) => {
                let (inner_type, hint, _) = self.infer_type_from_expr(&r.expr);
                if r.mutability.is_some() {
                    (format!("&mut {inner_type}"), hint, false)
                } else {
                    (format!("&{inner_type}"), hint, true)
                }
            }
            Expr::Path(p) => {
                // Variable reference — look up existing type
                if let Some(ident) = p.path.get_ident() {
                    let name = ident.to_string();
                    if let Some(info) = self.variables.get(&name) {
                        return (
                            info.type_name.clone(),
                            info.value_hint.clone(),
                            info.is_copy,
                        );
                    }
                    // Boolean literals
                    if name == "true" || name == "false" {
                        return ("bool".to_string(), name, true);
                    }
                }
                ("unknown".to_string(), "?".to_string(), false)
            }
            Expr::Macro(m) => {
                let macro_name = m
                    .mac
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident.to_string())
                    .unwrap_or_default();
                match macro_name.as_str() {
                    "vec" => ("Vec".to_string(), "vec![...]".to_string(), false),
                    "format" => ("String".to_string(), "format!(...)".to_string(), false),
                    "string" => ("String".to_string(), "String".to_string(), false),
                    _ => ("unknown".to_string(), format!("{macro_name}!(...)"), false),
                }
            }
            _ => ("unknown".to_string(), "?".to_string(), false),
        }
    }

    fn infer_from_call(&self, call: &syn::ExprCall) -> (String, String, bool) {
        if let Expr::Path(p) = &*call.func {
            let segments: Vec<_> = p
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect();
            let full_path = segments.join("::");

            if full_path == "String::from" {
                let hint = call
                    .args
                    .first()
                    .map(|a| quote::quote!(#a).to_string())
                    .unwrap_or_else(|| "\"\"".to_string());
                return ("String".to_string(), hint, false);
            }
            if full_path == "String::new" {
                return ("String".to_string(), "\"\"".to_string(), false);
            }
            if full_path == "Vec::new" || full_path == "Vec::with_capacity" {
                return ("Vec".to_string(), "[]".to_string(), false);
            }
            if full_path == "Box::new" {
                let hint = call
                    .args
                    .first()
                    .map(|a| quote::quote!(#a).to_string())
                    .unwrap_or_else(|| "?".to_string());
                return ("Box".to_string(), format!("Box({hint})"), false);
            }
            if full_path == "HashMap::new" {
                return ("HashMap".to_string(), "{{}}".to_string(), false);
            }
        }
        ("unknown".to_string(), "?".to_string(), false)
    }

    fn infer_from_method_call(&self, mc: &syn::ExprMethodCall) -> (String, String, bool) {
        let method = mc.method.to_string();
        match method.as_str() {
            "to_string" | "to_owned" => ("String".to_string(), "String".to_string(), false),
            "clone" => {
                let (ty, hint, is_copy) = self.infer_type_from_expr(&mc.receiver);
                (ty, hint, is_copy)
            }
            "to_vec" => ("Vec".to_string(), "Vec".to_string(), false),
            _ => ("unknown".to_string(), format!(".{method}()"), false),
        }
    }

    fn analyze_let_binding(&mut self, local: &syn::Local, line: usize) {
        // Extract variable name from pattern
        let var_name = match &local.pat {
            syn::Pat::Ident(pi) => pi.ident.to_string(),
            syn::Pat::Type(pt) => {
                if let syn::Pat::Ident(pi) = &*pt.pat {
                    pi.ident.to_string()
                } else {
                    return;
                }
            }
            _ => return,
        };

        // Determine type from annotation or init expression
        let (type_name, value_hint, is_copy) = if let Some(init) = &local.init {
            // Check type annotation first
            if let syn::Pat::Type(pat_type) = &local.pat {
                let ty = &pat_type.ty;
                let annotated = quote::quote!(#ty).to_string();
                let is_copy = Self::is_copy_type(&annotated);
                let (_, hint, _) = self.infer_type_from_expr(&init.expr);
                (annotated, hint, is_copy)
            } else {
                self.infer_type_from_expr(&init.expr)
            }
        } else {
            ("unknown".to_string(), "uninitialized".to_string(), false)
        };

        let is_ref = type_name.starts_with('&') || type_name.starts_with("& ");
        let memory = if Self::is_heap_type(&type_name) {
            MemoryLocation::Heap
        } else {
            MemoryLocation::Stack
        };

        // Check if init expression is a reference (borrow)
        if let Some(init) = &local.init
            && let Expr::Reference(r) = &*init.expr
            && let Expr::Path(p) = &*r.expr
            && let Some(ident) = p.path.get_ident()
        {
            let owner_name = ident.to_string();
            let mutable = r.mutability.is_some();

            let (status, owner_status) = if mutable {
                (VariableStatus::LiveMutRef, VariableStatus::BorrowedMut)
            } else {
                (VariableStatus::LiveRef, VariableStatus::BorrowedShared)
            };

            if let Some(owner) = self.variables.get_mut(&owner_name) {
                owner.status = owner_status;
                owner.borrowed_by.push(var_name.clone());
            }

            let info = VarInfo {
                type_name: type_name.clone(),
                status,
                memory: MemoryLocation::Stack,
                value_hint: value_hint.clone(),
                is_copy: false,
                borrowed_by: Vec::new(),
                borrows_from: Some(owner_name.clone()),
            };
            self.variables.insert(var_name.clone(), info);
            self.current_scope_mut().push(var_name.clone());

            let desc = if mutable {
                t!(
                    self.lang,
                    format!(
                        "`{var_name}` が `{owner_name}` を可変借用（&mut）します。この間 `{owner_name}` は読み書きできません（排他アクセス）"
                    ),
                    format!(
                        "`{var_name}` mutably borrows `{owner_name}` (&mut). `{owner_name}` cannot be read or written during this borrow (exclusive access)"
                    )
                )
            } else {
                t!(
                    self.lang,
                    format!(
                        "`{var_name}` が `{owner_name}` を共有借用（&）します。所有権は移動せず、`{owner_name}` も引き続き有効です"
                    ),
                    format!(
                        "`{var_name}` borrows `{owner_name}` by shared reference (&). Ownership does not move — `{owner_name}` remains valid"
                    )
                )
            };
            self.emit_step(
                line,
                desc,
                OwnershipEvent::BorrowStart {
                    from: owner_name,
                    to: var_name,
                    mutable,
                },
            );
            return;
        }

        // Check for move from existing variable
        if let Some(init) = &local.init
            && let Expr::Path(p) = &*init.expr
            && let Some(ident) = p.path.get_ident()
        {
            let source_name = ident.to_string();
            if let Some(source) = self.variables.get(&source_name)
                && !source.is_copy
                && source.status == VariableStatus::Owned
            {
                let moved_type = source.type_name.clone();
                let moved_hint = source.value_hint.clone();
                let moved_memory = source.memory;

                if let Some(src) = self.variables.get_mut(&source_name) {
                    src.status = VariableStatus::Moved;
                }

                let info = VarInfo {
                    type_name: moved_type.clone(),
                    status: VariableStatus::Owned,
                    memory: moved_memory,
                    value_hint: moved_hint,
                    is_copy: false,
                    borrowed_by: Vec::new(),
                    borrows_from: None,
                };
                self.variables.insert(var_name.clone(), info);
                self.current_scope_mut().push(var_name.clone());

                self.emit_step(
                    line,
                    t!(self.lang,
                        format!("`{moved_type}` の所有権が `{source_name}` から `{var_name}` に移動（move）します。以降 `{source_name}` は使用できません"),
                        format!("Ownership of `{moved_type}` moves from `{source_name}` to `{var_name}`. `{source_name}` can no longer be used")
                    ),
                    OwnershipEvent::Move {
                        from: source_name,
                        to: var_name,
                    },
                );
                return;
            }
        }

        // Check for .clone() call
        if let Some(init) = &local.init
            && let Expr::MethodCall(mc) = &*init.expr
            && mc.method == "clone"
            && mc.args.is_empty()
            && let Expr::Path(p) = &*mc.receiver
            && let Some(ident) = p.path.get_ident()
        {
            let source_name = ident.to_string();
            if let Some(source) = self.variables.get(&source_name) {
                let cloned_type = source.type_name.clone();
                let cloned_hint = source.value_hint.clone();
                let cloned_memory = source.memory;

                let info = VarInfo {
                    type_name: cloned_type,
                    status: VariableStatus::Owned,
                    memory: cloned_memory,
                    value_hint: cloned_hint,
                    is_copy: false,
                    borrowed_by: Vec::new(),
                    borrows_from: None,
                };
                self.variables.insert(var_name.clone(), info);
                self.current_scope_mut().push(var_name.clone());

                self.emit_step(
                    line,
                    t!(self.lang,
                        format!("`{var_name}` は `{source_name}` の深いコピー（clone）です。独立したメモリを持ち、元の値は影響を受けません"),
                        format!("`{var_name}` is a deep copy (clone) of `{source_name}`. It has its own memory — the original is unaffected")
                    ),
                    OwnershipEvent::Clone {
                        from: source_name,
                        to: var_name,
                    },
                );
                return;
            }
        }

        // Normal binding (new value)
        let status = if is_ref {
            VariableStatus::LiveRef
        } else {
            VariableStatus::Owned
        };

        let info = VarInfo {
            type_name: type_name.clone(),
            status,
            memory,
            value_hint: value_hint.clone(),
            is_copy,
            borrowed_by: Vec::new(),
            borrows_from: None,
        };
        self.variables.insert(var_name.clone(), info);
        self.current_scope_mut().push(var_name.clone());

        let mem_desc = if memory == MemoryLocation::Heap {
            t!(
                self.lang,
                "ヒープにデータを確保し、スタック上のポインタが参照します",
                "Allocates on the heap; a pointer on the stack references the data"
            )
        } else if is_copy {
            t!(
                self.lang,
                "スタックに直接値が格納されます（Copyトレイト）",
                "Stored directly on the stack (Copy trait)"
            )
        } else {
            t!(self.lang, "スタック上に格納されます", "Stored on the stack")
        };
        let bind_verb = t!(self.lang, "を束縛", "is bound to");
        self.emit_step(
            line,
            t!(
                self.lang,
                format!("`{var_name}` に `{type_name}` 型の値 {value_hint} を束縛。{mem_desc}"),
                format!("`{var_name}` {bind_verb} `{type_name}` = {value_hint}. {mem_desc}")
            ),
            OwnershipEvent::Bind {
                variable: var_name,
                type_name,
                value_hint,
            },
        );
    }

    fn analyze_expr_stmt(&mut self, expr: &Expr, line: usize) {
        // Check for function calls that consume variables
        match expr {
            Expr::Call(call) => {
                // Check each argument for ownership transfer
                for arg in &call.args {
                    self.check_use(arg, line);
                }
            }
            Expr::MethodCall(mc) => {
                self.check_use(&mc.receiver, line);
                for arg in &mc.args {
                    self.check_use(arg, line);
                }
            }
            _ => {}
        }
    }

    fn check_use(&mut self, expr: &Expr, _line: usize) {
        if let Expr::Path(p) = expr
            && let Some(ident) = p.path.get_ident()
        {
            let name = ident.to_string();
            if let Some(info) = self.variables.get(&name)
                && info.status == VariableStatus::Moved
            {
                let step_line = p
                    .path
                    .segments
                    .first()
                    .map(|s| s.ident.span().start().line)
                    .unwrap_or(0);
                self.emit_step(
                    step_line,
                    t!(self.lang,
                        format!("エラー: `{name}` は既にムーブ済みです。所有権が別の変数に移動した後は使用できません"),
                        format!("Error: `{name}` has already been moved. A variable cannot be used after its ownership has been transferred")
                    ),
                    OwnershipEvent::CompileError {
                        message: format!(
                            "borrow of moved value: `{name}`. Value was previously moved."
                        ),
                    },
                );
            }
        }
    }

    fn close_scope(&mut self, line: usize) {
        let scope_vars = self.scopes.pop().unwrap_or_default();
        // Drop in reverse order (LIFO), matching Rust semantics
        for var_name in scope_vars.into_iter().rev() {
            let status = self.variables.get(&var_name).map(|v| v.status);
            match status {
                Some(
                    VariableStatus::Owned
                    | VariableStatus::BorrowedShared
                    | VariableStatus::BorrowedMut,
                ) => {
                    if let Some(info) = self.variables.get_mut(&var_name) {
                        info.status = VariableStatus::Dropped;
                    }
                    let var_type = self
                        .variables
                        .get(&var_name)
                        .map(|v| v.type_name.clone())
                        .unwrap_or_default();
                    let heap = Self::is_heap_type(&var_type);
                    self.emit_step(
                        line,
                        t!(
                            self.lang,
                            format!(
                                "`{var_name}` がスコープを抜けて破棄（drop）されます{}",
                                if heap {
                                    "。ヒープメモリも解放されます"
                                } else {
                                    ""
                                }
                            ),
                            format!(
                                "`{var_name}` is dropped (goes out of scope){}",
                                if heap {
                                    ". Heap memory is also freed"
                                } else {
                                    ""
                                }
                            )
                        ),
                        OwnershipEvent::Drop { variable: var_name },
                    );
                }
                Some(VariableStatus::LiveRef | VariableStatus::LiveMutRef) => {
                    // End borrow
                    let owner = self
                        .variables
                        .get(&var_name)
                        .and_then(|v| v.borrows_from.clone())
                        .unwrap_or_default();

                    // Remove from owner's borrowed_by
                    if let Some(owner_info) = self.variables.get_mut(&owner) {
                        owner_info.borrowed_by.retain(|b| b != &var_name);
                        if owner_info.borrowed_by.is_empty() {
                            owner_info.status = VariableStatus::Owned;
                        }
                    }

                    if let Some(info) = self.variables.get_mut(&var_name) {
                        info.status = VariableStatus::Dropped;
                    }
                    self.emit_step(
                        line,
                        t!(self.lang,
                            format!("参照 `{var_name}` がスコープを抜け、`{owner}` への借用が終了。`{owner}` は再び自由に使えます"),
                            format!("Reference `{var_name}` goes out of scope, ending the borrow of `{owner}`. `{owner}` is free to use again")
                        ),
                        OwnershipEvent::BorrowEnd {
                            variable: var_name,
                            owner,
                        },
                    );
                }
                Some(VariableStatus::Moved) => {
                    // Already moved — no drop needed, just mark dropped
                    if let Some(info) = self.variables.get_mut(&var_name) {
                        info.status = VariableStatus::Dropped;
                    }
                }
                _ => {}
            }
        }
    }

    fn analyze_stmts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            match stmt {
                Stmt::Local(local) => {
                    let line = local.let_token.span.start().line;
                    self.analyze_let_binding(local, line);
                }
                Stmt::Expr(expr, _) => {
                    let line = self.expr_line(expr);
                    self.analyze_expr_stmt(expr, line);

                    // Handle block expressions (inner scopes)
                    if let Expr::Block(block) = expr {
                        self.scopes.push(Vec::new());
                        self.analyze_stmts(&block.block.stmts);
                        let close_line = block.block.brace_token.span.close().start().line;
                        self.close_scope(close_line);
                    }
                }
                Stmt::Item(_) => {} // Skip nested items
                Stmt::Macro(m) => {
                    // Handle println!, etc. — check args for use-after-move
                    let _line = m
                        .mac
                        .path
                        .segments
                        .first()
                        .map(|s| s.ident.span().start().line)
                        .unwrap_or(0);
                }
            }
        }
    }

    fn expr_line(&self, expr: &Expr) -> usize {
        match expr {
            Expr::Call(c) => {
                if let Expr::Path(p) = &*c.func {
                    p.path
                        .segments
                        .first()
                        .map(|s| s.ident.span().start().line)
                        .unwrap_or(0)
                } else {
                    0
                }
            }
            Expr::MethodCall(mc) => mc.method.span().start().line,
            Expr::Path(p) => p
                .path
                .segments
                .first()
                .map(|s| s.ident.span().start().line)
                .unwrap_or(0),
            _ => 0,
        }
    }
}

/// Analyze Rust source code and produce ownership step sequence.
pub fn analyze(source: &str) -> AnalysisResult {
    analyze_with_lang(source, Lang::default())
}

/// Analyze with a specific language for descriptions.
pub fn analyze_with_lang(source: &str, lang: Lang) -> AnalysisResult {
    let file = match syn::parse_str::<syn::File>(source) {
        Ok(f) => f,
        Err(e) => {
            return AnalysisResult {
                source: source.to_string(),
                steps: Vec::new(),
                has_error: true,
                error_message: Some(format!("Parse error: {e}")),
            };
        }
    };

    // Find fn main()
    let main_fn = file.items.iter().find_map(|item| {
        if let Item::Fn(f) = item
            && f.sig.ident == "main"
        {
            return Some(f);
        }
        None
    });

    let Some(main_fn) = main_fn else {
        return AnalysisResult {
            source: source.to_string(),
            steps: Vec::new(),
            has_error: true,
            error_message: Some("No `fn main()` found".to_string()),
        };
    };

    let mut analyzer = OwnershipAnalyzer::new(lang);
    analyzer.analyze_stmts(&main_fn.block.stmts);

    // Close the main scope
    let close_line = main_fn.block.brace_token.span.close().start().line;
    analyzer.close_scope(close_line);

    AnalysisResult {
        source: source.to_string(),
        steps: analyzer.steps,
        has_error: false,
        error_message: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_bind() {
        let result = analyze(
            r#"fn main() {
    let s = String::from("hello");
}"#,
        );
        assert!(!result.has_error);
        assert!(result.steps.len() >= 2); // bind + drop
        assert!(matches!(
            &result.steps[0].event,
            OwnershipEvent::Bind { variable, .. } if variable == "s"
        ));
    }

    #[test]
    fn test_move() {
        let result = analyze(
            r#"fn main() {
    let s = String::from("hello");
    let t = s;
}"#,
        );
        assert!(!result.has_error);
        let move_step = result
            .steps
            .iter()
            .find(|s| matches!(&s.event, OwnershipEvent::Move { .. }));
        assert!(move_step.is_some());
        let move_step = move_step.unwrap();
        assert!(matches!(
            &move_step.event,
            OwnershipEvent::Move { from, to } if from == "s" && to == "t"
        ));
        // After move, s should be Moved in variables
        let s_state = move_step.variables.iter().find(|v| v.name == "s");
        assert!(s_state.is_some());
        assert_eq!(s_state.unwrap().status, VariableStatus::Moved);
    }

    #[test]
    fn test_borrow() {
        let result = analyze(
            r#"fn main() {
    let s = String::from("hello");
    let r = &s;
}"#,
        );
        assert!(!result.has_error);
        let borrow_step = result
            .steps
            .iter()
            .find(|s| matches!(&s.event, OwnershipEvent::BorrowStart { mutable, .. } if !mutable));
        assert!(borrow_step.is_some());
    }

    #[test]
    fn test_mut_borrow() {
        let result = analyze(
            r#"fn main() {
    let mut s = String::from("hello");
    let r = &mut s;
}"#,
        );
        assert!(!result.has_error);
        let borrow_step = result
            .steps
            .iter()
            .find(|s| matches!(&s.event, OwnershipEvent::BorrowStart { mutable, .. } if *mutable));
        assert!(borrow_step.is_some());
    }

    #[test]
    fn test_clone() {
        let result = analyze(
            r#"fn main() {
    let s = String::from("hello");
    let t = s.clone();
}"#,
        );
        assert!(!result.has_error);
        let clone_step = result
            .steps
            .iter()
            .find(|s| matches!(&s.event, OwnershipEvent::Clone { .. }));
        assert!(clone_step.is_some());
        assert!(matches!(
            &clone_step.unwrap().event,
            OwnershipEvent::Clone { from, to } if from == "s" && to == "t"
        ));
    }

    #[test]
    fn test_copy_type_no_move() {
        let result = analyze(
            r#"fn main() {
    let x = 42;
    let y = x;
}"#,
        );
        assert!(!result.has_error);
        // Should have two binds, no moves (i32 is Copy)
        let moves: Vec<_> = result
            .steps
            .iter()
            .filter(|s| matches!(&s.event, OwnershipEvent::Move { .. }))
            .collect();
        assert!(moves.is_empty(), "Copy types should not trigger moves");
    }

    #[test]
    fn test_drop_order() {
        let result = analyze(
            r#"fn main() {
    let a = String::from("a");
    let b = String::from("b");
}"#,
        );
        assert!(!result.has_error);
        let drops: Vec<_> = result
            .steps
            .iter()
            .filter_map(|s| {
                if let OwnershipEvent::Drop { variable } = &s.event {
                    Some(variable.clone())
                } else {
                    None
                }
            })
            .collect();
        // Drops should be in reverse order: b then a
        assert_eq!(drops, vec!["b", "a"]);
    }

    #[test]
    fn test_parse_error() {
        let result = analyze("this is not valid rust");
        assert!(result.has_error);
        assert!(result.error_message.is_some());
    }

    #[test]
    fn test_no_main() {
        let result = analyze("fn foo() {}");
        assert!(result.has_error);
        assert!(
            result
                .error_message
                .as_deref()
                .unwrap()
                .contains("fn main()")
        );
    }
}
