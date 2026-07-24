use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use ra_ap_syntax::ast::{self, AstNode, HasArgList, HasAttrs, HasName};
use ra_ap_syntax::{Edition, SourceFile, SyntaxKind, SyntaxNode, TextRange};

use crate::config::Config;
use crate::error::{Error, Result};
use crate::issue::{IssueType, RiskAxis};

#[derive(Debug, Clone)]
pub(crate) struct ParsedFile {
    pub findings: Vec<Finding>,
    pub parse_errors: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct Finding {
    pub issue_type: IssueType,
    pub risk: RiskAxis,
    pub condition: u8,
    pub file: PathBuf,
    pub rel_path: String,
    pub line: usize,
    pub function: String,
    pub target: String,
    pub message: String,
    pub remediation: String,
}

#[derive(Debug, Clone)]
struct AsyncScope {
    node: SyntaxNode,
    function: String,
}

struct SpawnContext<'a> {
    file: &'a Path,
    rel_path: &'a str,
    source: &'a str,
    aliases: &'a ImportAliases,
    function_names: &'a HashMap<TextRange, String>,
}

#[derive(Debug, Clone, Default)]
struct ImportAliases {
    aliases: HashMap<String, String>,
}

const CONDITION_LOW: u8 = 1;
const CONDITION_MEDIUM: u8 = 2;
const CONDITION_HIGH: u8 = 3;

impl ImportAliases {
    fn collect(root: &SyntaxNode) -> Self {
        let mut aliases = Self::default();
        for item in root.descendants().filter_map(ast::Use::cast) {
            if let Some(tree) = item.use_tree() {
                aliases.collect_tree(&tree, "");
            }
        }
        aliases
    }

    fn collect_tree(&mut self, tree: &ast::UseTree, prefix: &str) {
        let local = tree
            .path()
            .map(|path| compact_path(path.syntax().text().to_string()))
            .unwrap_or_default();
        let full = join_path(prefix, &local);
        if let Some(list) = tree.use_tree_list() {
            for child in list.use_trees() {
                self.collect_tree(&child, &full);
            }
            return;
        }
        if full.is_empty() || tree.star_token().is_some() {
            return;
        }
        let alias = tree
            .rename()
            .and_then(|rename| rename.name())
            .map(|name| name.text().to_string())
            .or_else(|| full.rsplit("::").next().map(str::to_string));
        if let Some(alias) = alias.filter(|alias| is_ident(alias)) {
            self.aliases.insert(alias, full);
        }
    }

    fn resolve_path(&self, path: &str) -> String {
        let Some((head, tail)) = path.split_once("::") else {
            return self
                .aliases
                .get(path)
                .cloned()
                .unwrap_or_else(|| path.to_string());
        };
        self.aliases
            .get(head)
            .map(|prefix| format!("{prefix}::{tail}"))
            .unwrap_or_else(|| path.to_string())
    }

    fn is_timeout_path(&self, path: &str) -> bool {
        matches!(
            self.resolve_path(path).as_str(),
            "tokio::time::timeout" | "async_std::future::timeout" | "timeout"
        )
    }

    fn is_drop_path(&self, path: &str) -> bool {
        matches!(
            self.resolve_path(path).as_str(),
            "drop" | "std::mem::drop" | "core::mem::drop" | "mem::drop"
        )
    }
}

pub(crate) fn parse_file(
    root: &Path,
    path: &Path,
    source: &str,
    edition: Edition,
    config: &Config,
) -> ParsedFile {
    let rel_path = relative_path(root, path);
    let parsed = SourceFile::parse(source, edition);
    let tree = parsed.tree();
    let parse_errors = parsed.errors().len();
    let aliases = ImportAliases::collect(tree.syntax());
    let mut id_counts = HashMap::new();
    let function_names = function_names(&rel_path, tree.syntax(), &mut id_counts);
    let scopes = async_scopes(&rel_path, tree.syntax(), &function_names);
    let mut findings = Vec::new();

    for scope in scopes {
        if excluded_test_context(&scope.node) {
            continue;
        }
        detect_guard_across_await(
            path,
            &rel_path,
            source,
            &scope,
            config,
            &aliases,
            &mut findings,
        );
        detect_blocking_calls(
            path,
            &rel_path,
            source,
            &scope,
            config,
            &aliases,
            &mut findings,
        );
        detect_missing_timeout(
            path,
            &rel_path,
            source,
            &scope,
            config,
            &aliases,
            &mut findings,
        );
    }
    let spawn_context = SpawnContext {
        file: path,
        rel_path: &rel_path,
        source,
        aliases: &aliases,
        function_names: &function_names,
    };
    detect_spawns(tree.syntax(), config, &spawn_context, &mut findings);

    ParsedFile {
        findings,
        parse_errors,
    }
}

pub(crate) fn parse_file_from_disk(
    root: &Path,
    path: &Path,
    edition: Edition,
    config: &Config,
) -> Result<ParsedFile> {
    let source = fs::read_to_string(path).map_err(|source| Error::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(parse_file(root, path, &source, edition, config))
}

fn function_names(
    rel_path: &str,
    root: &SyntaxNode,
    id_counts: &mut HashMap<String, usize>,
) -> HashMap<TextRange, String> {
    let mut names = HashMap::new();
    for func in root.descendants().filter_map(ast::Fn::cast) {
        let name = func
            .name()
            .map(|name| name.text().to_string())
            .unwrap_or_else(|| "<anonymous>".to_string());
        let base = enclosing_impl_type(func.syntax())
            .map(|ty| format!("{rel_path}:{ty}::{name}"))
            .unwrap_or_else(|| format!("{rel_path}:{name}"));
        let id = disambiguate(id_counts, base);
        names.insert(func.syntax().text_range(), id);
    }
    names
}

fn async_scopes(
    rel_path: &str,
    root: &SyntaxNode,
    function_names: &HashMap<TextRange, String>,
) -> Vec<AsyncScope> {
    let mut scopes = Vec::new();
    for node in root.descendants() {
        if let Some(func) = ast::Fn::cast(node.clone()) {
            if func.async_token().is_some() {
                if let Some(body) = func.body() {
                    let function = function_names
                        .get(&func.syntax().text_range())
                        .cloned()
                        .unwrap_or_else(|| format!("{rel_path}:<async-fn>"));
                    scopes.push(AsyncScope {
                        node: body.syntax().clone(),
                        function,
                    });
                }
            }
            continue;
        }
        if let Some(block) = ast::BlockExpr::cast(node) {
            if block.async_token().is_some() {
                let function = enclosing_function(block.syntax(), function_names)
                    .map(|function| {
                        let index = async_block_index_in_function(block.syntax());
                        format!("{function}:async-block#{index}")
                    })
                    .unwrap_or_else(|| {
                        let index = root_async_block_index(root, block.syntax());
                        format!("{rel_path}:<async-block>#{index}")
                    });
                scopes.push(AsyncScope {
                    node: block.syntax().clone(),
                    function,
                });
            }
        }
    }
    scopes
}

fn detect_guard_across_await(
    file: &Path,
    rel_path: &str,
    source: &str,
    scope: &AsyncScope,
    config: &Config,
    aliases: &ImportAliases,
    findings: &mut Vec<Finding>,
) {
    if config.is_allowed(IssueType::GuardAcrossAwait) {
        return;
    }
    for let_stmt in scope.node.descendants().filter_map(ast::LetStmt::cast) {
        if excluded_test_context(let_stmt.syntax()) || !is_guard_binding(&let_stmt, aliases) {
            continue;
        }
        let Some(name) = binding_name(&let_stmt) else {
            continue;
        };
        let let_range = let_stmt.syntax().text_range();
        let block = nearest_block(let_stmt.syntax()).unwrap_or_else(|| scope.node.clone());
        let Some(await_node) = first_await_after(&block, let_range.end(), source) else {
            continue;
        };
        if has_drop_before(
            &block,
            &name,
            let_range.end(),
            await_node.text_range().start(),
            aliases,
        ) {
            continue;
        }
        findings.push(Finding {
            issue_type: IssueType::GuardAcrossAwait,
            risk: RiskAxis::Deadlock,
            condition: guard_condition(&block, let_range.end(), await_node.text_range().start()),
            file: file.to_path_buf(),
            rel_path: rel_path.to_string(),
            line: line_for_range(source, let_range),
            function: scope.function.clone(),
            target: name.clone(),
            message: format!("synchronous guard `{name}` is live across an .await"),
            remediation:
                "Drop the guard before awaiting, narrow the critical section, or switch to an async-aware design."
                    .to_string(),
        });
    }
    for let_expr in scope.node.descendants().filter_map(ast::LetExpr::cast) {
        if excluded_test_context(let_expr.syntax()) || !is_guard_let_expr(&let_expr, aliases) {
            continue;
        }
        let Some(pat) = let_expr.pat() else {
            continue;
        };
        let Some(name) = binding_name_from_pat(pat.syntax()) else {
            continue;
        };
        let let_range = let_expr.syntax().text_range();
        let block = nearest_block(let_expr.syntax()).unwrap_or_else(|| scope.node.clone());
        let Some(await_node) = first_await_after(&block, let_range.end(), source) else {
            continue;
        };
        if has_drop_before(
            &block,
            &name,
            let_range.end(),
            await_node.text_range().start(),
            aliases,
        ) {
            continue;
        }
        findings.push(Finding {
            issue_type: IssueType::GuardAcrossAwait,
            risk: RiskAxis::Deadlock,
            condition: guard_condition(&block, let_range.end(), await_node.text_range().start()),
            file: file.to_path_buf(),
            rel_path: rel_path.to_string(),
            line: line_for_range(source, let_range),
            function: scope.function.clone(),
            target: name.clone(),
            message: format!("synchronous guard `{name}` is live across an .await"),
            remediation:
                "Drop the guard before awaiting, narrow the critical section, or switch to an async-aware design."
                    .to_string(),
        });
    }
}

fn detect_blocking_calls(
    file: &Path,
    rel_path: &str,
    source: &str,
    scope: &AsyncScope,
    config: &Config,
    aliases: &ImportAliases,
    findings: &mut Vec<Finding>,
) {
    if config.is_allowed(IssueType::BlockingInAsync) {
        return;
    }
    let mut seen = HashSet::new();
    for call in scope.node.descendants().filter_map(ast::CallExpr::cast) {
        if excluded_test_context(call.syntax()) {
            continue;
        }
        let Some(expr) = call.expr() else {
            continue;
        };
        let path = aliases.resolve_path(&compact_path(expr.syntax().text().to_string()));
        let Some(matched) = config
            .blocking_calls
            .iter()
            .find(|entry| path == entry.as_str() || path.starts_with(&format!("{entry}::")))
        else {
            continue;
        };
        let range = call.syntax().text_range();
        if !seen.insert((range, matched.clone())) {
            continue;
        }
        findings.push(Finding {
            issue_type: IssueType::BlockingInAsync,
            risk: RiskAxis::Latency,
            condition: blocking_condition(&path),
            file: file.to_path_buf(),
            rel_path: rel_path.to_string(),
            line: line_for_range(source, range),
            function: scope.function.clone(),
            target: path,
            message: format!("blocking call `{matched}` appears inside async code"),
            remediation:
                "Move the call to spawn_blocking, use an async API, or isolate it behind bounded worker capacity."
                    .to_string(),
        });
    }
    for method in scope
        .node
        .descendants()
        .filter_map(ast::MethodCallExpr::cast)
    {
        if excluded_test_context(method.syntax()) {
            continue;
        }
        let Some(name) = method.name_ref().map(|name| name.text().to_string()) else {
            continue;
        };
        let Some(matched) = config.blocking_calls.iter().find(|entry| {
            entry.rsplit("::").next().is_some_and(|tail| tail == name)
                || entry.rsplit('.').next().is_some_and(|tail| tail == name)
        }) else {
            continue;
        };
        let range = method.syntax().text_range();
        if !seen.insert((range, matched.clone())) {
            continue;
        }
        findings.push(Finding {
            issue_type: IssueType::BlockingInAsync,
            risk: RiskAxis::Latency,
            condition: blocking_condition(matched),
            file: file.to_path_buf(),
            rel_path: rel_path.to_string(),
            line: line_for_range(source, range),
            function: scope.function.clone(),
            target: name.clone(),
            message: format!("blocking method `{name}` matches configured `{matched}` inside async code"),
            remediation:
                "Move the call to spawn_blocking, use an async API, or isolate it behind bounded worker capacity."
                    .to_string(),
        });
    }
}

fn detect_spawns(
    root: &SyntaxNode,
    config: &Config,
    context: &SpawnContext<'_>,
    findings: &mut Vec<Finding>,
) {
    let allow_unbounded = config.is_allowed(IssueType::UnboundedSpawn);
    let allow_detached = config.is_allowed(IssueType::DetachedTask);
    if allow_unbounded && allow_detached {
        return;
    }
    for call in root.descendants().filter_map(ast::CallExpr::cast) {
        if excluded_test_context(call.syntax()) || !is_tokio_spawn(&call, context.aliases) {
            continue;
        }
        let function = enclosing_function(call.syntax(), context.function_names)
            .unwrap_or_else(|| format!("{}:<module>", context.rel_path));
        let range = call.syntax().text_range();
        if !allow_unbounded && in_loop(call.syntax()) && !join_handle_retained(&call) {
            findings.push(Finding {
                issue_type: IssueType::UnboundedSpawn,
                risk: RiskAxis::Starvation,
                condition: spawn_condition(call.syntax()),
                file: context.file.to_path_buf(),
                rel_path: context.rel_path.to_string(),
                line: line_for_range(context.source, range),
                function: function.clone(),
                target: "tokio::spawn".to_string(),
                message: "tokio::spawn is called in a loop and the JoinHandle is discarded"
                    .to_string(),
                remediation:
                    "Use JoinSet, a Semaphore, a bounded channel, or store handles and apply backpressure."
                        .to_string(),
            });
        }
        if !allow_detached
            && contains_infinite_loop(call.syntax())
            && !under_cancellation_context(call.syntax(), context.aliases)
        {
            findings.push(Finding {
                issue_type: IssueType::DetachedTask,
                risk: RiskAxis::Leak,
                condition: detached_condition(call.syntax()),
                file: context.file.to_path_buf(),
                rel_path: context.rel_path.to_string(),
                line: line_for_range(context.source, range),
                function: function.clone(),
                target: "tokio::spawn(loop)".to_string(),
                message: "long-running spawned task has no visible cancellation boundary".to_string(),
                remediation:
                    "Run the task under JoinSet/select!/timeout or pass an explicit cancellation token."
                        .to_string(),
            });
        }
    }
}

fn detect_missing_timeout(
    file: &Path,
    rel_path: &str,
    source: &str,
    scope: &AsyncScope,
    config: &Config,
    aliases: &ImportAliases,
    findings: &mut Vec<Finding>,
) {
    if config.is_allowed(IssueType::MissingTimeout) {
        return;
    }
    for method in scope
        .node
        .descendants()
        .filter_map(ast::MethodCallExpr::cast)
    {
        if excluded_test_context(method.syntax())
            || under_timeout(method.syntax(), aliases)
            || method_chain_has_timeout(&method)
            || is_channel_operation(&method)
        {
            continue;
        }
        let Some(name) = method.name_ref().map(|name| name.text().to_string()) else {
            continue;
        };
        if !config.timeout_methods.contains(&name) {
            continue;
        }
        let range = method.syntax().text_range();
        findings.push(Finding {
            issue_type: IssueType::MissingTimeout,
            risk: RiskAxis::Latency,
            condition: timeout_condition(&name),
            file: file.to_path_buf(),
            rel_path: rel_path.to_string(),
            line: line_for_range(source, range),
            function: scope.function.clone(),
            target: name.clone(),
            message: format!("external-looking `{name}` call is not wrapped in a timeout"),
            remediation:
                "Wrap the operation in tokio::time::timeout or enforce a timeout in the client configuration."
                    .to_string(),
        });
    }
}

fn guard_condition(
    block: &SyntaxNode,
    start: ra_ap_syntax::TextSize,
    end: ra_ap_syntax::TextSize,
) -> u8 {
    let statements_between = block
        .descendants()
        .filter(|node| {
            let range = node.text_range();
            range.start() > start
                && range.end() < end
                && matches!(
                    node.kind(),
                    SyntaxKind::LET_STMT
                        | SyntaxKind::EXPR_STMT
                        | SyntaxKind::IF_EXPR
                        | SyntaxKind::MATCH_EXPR
                        | SyntaxKind::LOOP_EXPR
                        | SyntaxKind::FOR_EXPR
                        | SyntaxKind::WHILE_EXPR
                )
        })
        .count();
    if block
        .descendants()
        .any(|node| matches!(node.kind(), SyntaxKind::IF_EXPR | SyntaxKind::MATCH_EXPR))
    {
        CONDITION_HIGH
    } else if statements_between > 1 {
        CONDITION_MEDIUM
    } else {
        CONDITION_LOW
    }
}

fn blocking_condition(path: &str) -> u8 {
    if path.contains("std::thread::sleep") || path.contains("rusqlite") || path.contains("ureq") {
        CONDITION_HIGH
    } else if path.contains("std::fs") || path.contains("std::net") {
        CONDITION_MEDIUM
    } else {
        CONDITION_LOW
    }
}

fn spawn_condition(node: &SyntaxNode) -> u8 {
    if contains_infinite_loop(node) {
        CONDITION_HIGH
    } else if node
        .ancestors()
        .any(|ancestor| ancestor.kind() == SyntaxKind::FOR_EXPR)
    {
        CONDITION_MEDIUM
    } else {
        CONDITION_LOW
    }
}

fn detached_condition(node: &SyntaxNode) -> u8 {
    if node
        .descendants()
        .any(|descendant| descendant.kind() == SyntaxKind::AWAIT_EXPR)
    {
        CONDITION_HIGH
    } else {
        CONDITION_MEDIUM
    }
}

fn timeout_condition(name: &str) -> u8 {
    match name {
        "connect" | "request" => CONDITION_HIGH,
        "send" => CONDITION_MEDIUM,
        _ => CONDITION_LOW,
    }
}

fn is_guard_binding(let_stmt: &ast::LetStmt, aliases: &ImportAliases) -> bool {
    if let Some(ty) = let_stmt.ty() {
        let text = compact_path(ty.syntax().text().to_string());
        if text.contains("MutexGuard") || text.contains("RwLockGuard") {
            return !text.contains("tokio::sync") && !is_tokio_guard_type(&text, aliases);
        }
    }
    let Some(initializer) = let_stmt.initializer() else {
        return false;
    };
    is_guard_initializer(&initializer)
}

fn is_guard_let_expr(let_expr: &ast::LetExpr, _aliases: &ImportAliases) -> bool {
    let Some(initializer) = let_expr.expr() else {
        return false;
    };
    is_guard_initializer(&initializer)
}

fn is_guard_initializer(initializer: &ast::Expr) -> bool {
    let text = compact_path(initializer.syntax().text().to_string());
    if text.contains("tokio::sync") {
        return false;
    }
    if initializer_is_async_guard_await(initializer) {
        return false;
    }
    initializer.syntax().descendants().any(|node| {
        ast::MethodCallExpr::cast(node)
            .and_then(|method| method.name_ref().map(|name| name.text().to_string()))
            .map(|name| is_guard_method(&name))
            .unwrap_or(false)
    })
}

fn is_guard_method(name: &str) -> bool {
    matches!(
        name,
        "lock" | "read" | "write" | "try_lock" | "try_read" | "try_write"
    )
}

fn is_tokio_guard_type(text: &str, aliases: &ImportAliases) -> bool {
    let head = text
        .split(['<', ':'])
        .find(|part| !part.is_empty())
        .unwrap_or(text);
    aliases
        .aliases
        .get(head)
        .is_some_and(|path| path.starts_with("tokio::sync::"))
}

fn initializer_is_async_guard_await(initializer: &ast::Expr) -> bool {
    let text = compact_path(initializer.syntax().text().to_string());
    if !text.ends_with(".await") {
        return false;
    }
    [".lock().await", ".read().await", ".write().await"]
        .iter()
        .any(|suffix| text.ends_with(suffix))
}

fn binding_name(let_stmt: &ast::LetStmt) -> Option<String> {
    binding_name_from_pat(let_stmt.pat()?.syntax())
}

fn binding_name_from_pat(pat: &SyntaxNode) -> Option<String> {
    let mut last = None;
    for text in pat
        .descendants_with_tokens()
        .filter_map(|item| item.into_token())
        .map(|token| token.text().to_string())
        .filter(|text| is_ident(text) && text != "mut" && text != "ref" && text != "Ok")
    {
        last = Some(text);
    }
    last
}

fn first_await_after(
    block: &SyntaxNode,
    after: ra_ap_syntax::TextSize,
    source: &str,
) -> Option<SyntaxNode> {
    let mut awaits = block
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::AWAIT_EXPR && node.text_range().start() > after)
        .collect::<Vec<_>>();
    awaits.sort_by_key(|node| line_for_range(source, node.text_range()));
    awaits.into_iter().next()
}

fn has_drop_before(
    block: &SyntaxNode,
    binding: &str,
    start: ra_ap_syntax::TextSize,
    end: ra_ap_syntax::TextSize,
    aliases: &ImportAliases,
) -> bool {
    block
        .descendants()
        .filter_map(ast::CallExpr::cast)
        .any(|call| {
            let range = call.syntax().text_range();
            if range.start() <= start || range.end() >= end {
                return false;
            }
            let Some(expr) = call.expr() else {
                return false;
            };
            aliases.is_drop_path(&compact_path(expr.syntax().text().to_string()))
                && call_single_arg_ident(&call).is_some_and(|arg| arg == binding)
        })
}

fn is_tokio_spawn(call: &ast::CallExpr, aliases: &ImportAliases) -> bool {
    call.expr()
        .map(|expr| {
            let path = aliases.resolve_path(&compact_path(expr.syntax().text().to_string()));
            path == "tokio::spawn" || path == "tokio::task::spawn"
        })
        .unwrap_or(false)
}

fn in_loop(node: &SyntaxNode) -> bool {
    node.ancestors().skip(1).any(|ancestor| {
        matches!(
            ancestor.kind(),
            SyntaxKind::LOOP_EXPR | SyntaxKind::FOR_EXPR | SyntaxKind::WHILE_EXPR
        )
    })
}

fn join_handle_retained(call: &ast::CallExpr) -> bool {
    for ancestor in call.syntax().ancestors().skip(1) {
        if let Some(let_stmt) = ast::LetStmt::cast(ancestor.clone()) {
            let pat = let_stmt
                .pat()
                .map(|pat| pat.syntax().text().to_string())
                .unwrap_or_default();
            return pat.trim() != "_";
        }
        if let Some(method) = ast::MethodCallExpr::cast(ancestor.clone()) {
            let name = method
                .name_ref()
                .map(|name| name.text().to_string())
                .unwrap_or_default();
            if name == "push" || name == "insert" {
                return true;
            }
        }
        if ancestor.kind() == SyntaxKind::STMT_LIST {
            break;
        }
    }
    false
}

fn contains_infinite_loop(node: &SyntaxNode) -> bool {
    node.descendants()
        .filter(|descendant| descendant.kind() == SyntaxKind::LOOP_EXPR)
        .any(|loop_node| !loop_has_own_break(&loop_node))
}

fn under_cancellation_context(node: &SyntaxNode, aliases: &ImportAliases) -> bool {
    node.ancestors().skip(1).any(|ancestor| {
        if let Some(call) = ast::CallExpr::cast(ancestor.clone()) {
            return call
                .expr()
                .map(|expr| {
                    aliases.is_timeout_path(&compact_path(expr.syntax().text().to_string()))
                })
                .unwrap_or(false);
        }
        if let Some(method) = ast::MethodCallExpr::cast(ancestor.clone()) {
            let method_name = method
                .name_ref()
                .map(|name| name.text().to_string())
                .unwrap_or_default();
            let receiver = method
                .receiver()
                .map(|receiver| compact_path(receiver.syntax().text().to_string()))
                .unwrap_or_default();
            return method_name == "spawn" && receiver.contains("JoinSet");
        }
        if let Some(mac) = ast::MacroCall::cast(ancestor) {
            let text = compact_path(mac.syntax().text().to_string());
            return text.starts_with("tokio::select!") || text.starts_with("select!");
        }
        false
    })
}

fn under_timeout(node: &SyntaxNode, aliases: &ImportAliases) -> bool {
    node.ancestors().skip(1).any(|ancestor| {
        ast::CallExpr::cast(ancestor)
            .and_then(|call| call.expr())
            .map(|expr| aliases.is_timeout_path(&compact_path(expr.syntax().text().to_string())))
            .unwrap_or(false)
    })
}

fn method_chain_has_timeout(method: &ast::MethodCallExpr) -> bool {
    method
        .receiver()
        .map(|receiver| compact_path(receiver.syntax().text().to_string()).contains(".timeout("))
        .unwrap_or(false)
}

fn is_channel_operation(method: &ast::MethodCallExpr) -> bool {
    let Some(name) = method.name_ref().map(|name| name.text().to_string()) else {
        return false;
    };
    if !matches!(
        name.as_str(),
        "send" | "recv" | "blocking_send" | "blocking_recv"
    ) {
        return false;
    }
    let receiver = method
        .receiver()
        .map(|receiver| compact_path(receiver.syntax().text().to_string()))
        .unwrap_or_default();
    if contains_channel_marker(&receiver) {
        return true;
    }
    let Some(receiver_ident) = receiver
        .split(['.', '(', ')'])
        .find(|part| is_ident(part))
        .map(str::to_string)
    else {
        return false;
    };
    nearest_block(method.syntax()).is_some_and(|block| {
        block
            .descendants()
            .filter_map(ast::LetStmt::cast)
            .take_while(|stmt| {
                stmt.syntax().text_range().start() < method.syntax().text_range().start()
            })
            .any(|stmt| {
                stmt.pat()
                    .is_some_and(|pat| pat.syntax().text().to_string().contains(&receiver_ident))
                    && stmt.initializer().is_some_and(|initializer| {
                        contains_channel_marker(&compact_path(
                            initializer.syntax().text().to_string(),
                        ))
                    })
            })
    })
}

fn contains_channel_marker(text: &str) -> bool {
    text.contains("mpsc::")
        || text.contains("oneshot::")
        || text.contains("watch::")
        || text.contains("broadcast::")
        || text.contains("channel(")
}

fn call_single_arg_ident(call: &ast::CallExpr) -> Option<String> {
    let args = call.arg_list()?;
    let mut exprs = args.syntax().children().filter_map(ast::Expr::cast);
    let first = exprs.next()?;
    if exprs.next().is_some() {
        return None;
    }
    let text = compact_path(first.syntax().text().to_string());
    is_ident(&text).then_some(text)
}

fn loop_has_own_break(loop_node: &SyntaxNode) -> bool {
    let loop_label = loop_label(loop_node);
    loop_node
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::BREAK_EXPR)
        .any(|break_node| {
            if let Some(label) = break_label(&break_node) {
                return loop_label.as_deref() == Some(label.as_str());
            }
            nearest_enclosing_loop(&break_node).is_some_and(|nearest| nearest == *loop_node)
        })
}

fn nearest_enclosing_loop(node: &SyntaxNode) -> Option<SyntaxNode> {
    node.ancestors().skip(1).find(|ancestor| {
        matches!(
            ancestor.kind(),
            SyntaxKind::LOOP_EXPR | SyntaxKind::FOR_EXPR | SyntaxKind::WHILE_EXPR
        )
    })
}

fn loop_label(loop_node: &SyntaxNode) -> Option<String> {
    let text = compact_path(loop_node.text().to_string());
    text.strip_prefix('\'')
        .and_then(|rest| rest.split(':').next())
        .filter(|label| !label.is_empty())
        .map(|label| format!("'{label}"))
}

fn break_label(break_node: &SyntaxNode) -> Option<String> {
    let text = compact_path(break_node.text().to_string());
    text.strip_prefix("break'")
        .and_then(|rest| rest.split([';', ' ']).next())
        .filter(|label| !label.is_empty())
        .map(|label| format!("'{label}"))
}

fn nearest_block(node: &SyntaxNode) -> Option<SyntaxNode> {
    node.ancestors()
        .skip(1)
        .find(|ancestor| ast::BlockExpr::can_cast(ancestor.kind()))
}

fn enclosing_impl_type(node: &SyntaxNode) -> Option<String> {
    let imp = node.ancestors().skip(1).find_map(ast::Impl::cast)?;
    let text = strip_whitespace(&imp.syntax().text().to_string());
    let head = text.split('{').next().unwrap_or(&text);
    let raw = head
        .rsplit_once("for")
        .map(|(_, ty)| ty)
        .or_else(|| head.strip_prefix("impl"))
        .unwrap_or(head)
        .trim();
    let ty = raw
        .trim_start_matches("impl")
        .split(['<', ' ', ':'])
        .find(|part| !part.is_empty() && *part != "unsafe" && *part != "const")
        .unwrap_or(raw);
    (!ty.is_empty()).then(|| ty.to_string())
}

fn async_block_index_in_function(block: &SyntaxNode) -> usize {
    let Some(func) = block
        .ancestors()
        .skip(1)
        .find(|node| ast::Fn::can_cast(node.kind()))
    else {
        return 1;
    };
    async_block_index(func, block)
}

fn root_async_block_index(root: &SyntaxNode, block: &SyntaxNode) -> usize {
    async_block_index(root.clone(), block)
}

fn async_block_index(root: SyntaxNode, block: &SyntaxNode) -> usize {
    root.descendants()
        .filter_map(ast::BlockExpr::cast)
        .filter(|candidate| candidate.async_token().is_some())
        .filter(|candidate| candidate.syntax().text_range().start() <= block.text_range().start())
        .count()
        .max(1)
}

fn enclosing_function(
    node: &SyntaxNode,
    function_names: &HashMap<TextRange, String>,
) -> Option<String> {
    for ancestor in node.ancestors().skip(1) {
        if ast::Fn::can_cast(ancestor.kind()) {
            return function_names.get(&ancestor.text_range()).cloned();
        }
    }
    None
}

fn excluded_test_context(node: &SyntaxNode) -> bool {
    node.ancestors().any(|ancestor| {
        ast::Fn::cast(ancestor.clone())
            .map(|func| has_test_attr(func.attrs()) || has_cfg_test_attr(func.attrs()))
            .unwrap_or(false)
            || ast::Module::cast(ancestor)
                .map(|module| has_cfg_test_attr(module.attrs()))
                .unwrap_or(false)
    })
}

fn has_test_attr(attrs: impl Iterator<Item = ast::Attr>) -> bool {
    attrs.into_iter().any(|attr| {
        let text = strip_whitespace(&attr.syntax().text().to_string());
        text == "#[test]"
            || text.starts_with("#[tokio::test")
            || text.starts_with("#[async_std::test")
    })
}

fn has_cfg_test_attr(attrs: impl Iterator<Item = ast::Attr>) -> bool {
    attrs
        .into_iter()
        .any(|attr| strip_whitespace(&attr.syntax().text().to_string()).contains("#[cfg(test)]"))
}

fn compact_path(value: String) -> String {
    value.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn join_path(prefix: &str, local: &str) -> String {
    match (prefix.is_empty(), local.is_empty()) {
        (true, true) => String::new(),
        (true, false) => local.to_string(),
        (false, true) => prefix.to_string(),
        (false, false) => format!("{prefix}::{local}"),
    }
}

fn strip_whitespace(value: &str) -> String {
    value.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn is_ident(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn line_for_range(source: &str, range: TextRange) -> usize {
    let start = u32::from(range.start()) as usize;
    source[..start.min(source.len())]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

fn disambiguate(counts: &mut HashMap<String, usize>, base: String) -> String {
    let count = counts.entry(base.clone()).or_insert(0);
    *count += 1;
    if *count == 1 {
        base
    } else {
        format!("{base}#{}", *count)
    }
}

fn relative_path(root: &Path, path: &Path) -> String {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    path.strip_prefix(&root)
        .unwrap_or(&path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> Vec<Finding> {
        parse_file(
            Path::new("/tmp/example"),
            Path::new("/tmp/example/src/lib.rs"),
            source,
            Edition::Edition2024,
            &Config::default(),
        )
        .findings
    }

    #[test]
    fn comments_do_not_produce_blocking_calls() {
        let findings = parse(
            r#"
            async fn ok() {
                /* std::thread::sleep(std::time::Duration::from_secs(1)); */
            }
            "#,
        );
        assert!(findings.is_empty());
    }

    #[test]
    fn dropped_guard_before_await_is_not_reported() {
        let findings = parse(
            r#"
            async fn ok(lock: std::sync::Mutex<u8>) {
                let guard = lock.lock();
                drop(guard);
                work().await;
            }
            async fn work() {}
            "#,
        );
        assert!(
            !findings
                .iter()
                .any(|finding| finding.issue_type == IssueType::GuardAcrossAwait)
        );
    }
}
