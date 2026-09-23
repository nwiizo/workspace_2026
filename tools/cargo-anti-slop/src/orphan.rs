//! Module-tree reachability: find .rs files under src/ that no `mod`
//! declaration reaches. Refactors (human and agent alike) leave the old
//! monolith behind; the leftover is never compiled but keeps getting read.

use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use ra_ap_syntax::ast::{self, AstNode, HasAttrs, HasName};
use ra_ap_syntax::{Edition, SourceFile};

use crate::parser::compact_path;

#[derive(Debug, Default)]
pub(crate) struct OrphanScan {
    pub orphans: Vec<PathBuf>,
    pub skipped: bool,
    pub unreliable: bool,
}

pub(crate) fn scan(root: &Path, files: &[PathBuf], edition: Edition) -> OrphanScan {
    let src = root.join("src");
    if root.is_file() || !src.is_dir() || !root.join("Cargo.toml").is_file() {
        return OrphanScan {
            orphans: Vec::new(),
            skipped: true,
            unreliable: false,
        };
    }
    let set: HashSet<PathBuf> = files.iter().cloned().collect();
    let mut roots = Vec::new();
    for candidate in [src.join("lib.rs"), src.join("main.rs")] {
        if set.contains(&candidate) {
            roots.push(candidate);
        }
    }
    let bin_dir = src.join("bin");
    for file in &set {
        let Ok(rel) = file.strip_prefix(&bin_dir) else {
            continue;
        };
        // src/bin/foo.rs and src/bin/foo/main.rs are crate roots.
        let is_root = rel.components().count() == 1
            || file.file_name().and_then(|name| name.to_str()) == Some("main.rs");
        if is_root {
            roots.push(file.clone());
        }
    }
    if roots.is_empty() {
        return OrphanScan {
            orphans: Vec::new(),
            skipped: true,
            unreliable: false,
        };
    }

    let mut reached: HashSet<PathBuf> = roots.iter().cloned().collect();
    let mut queue: VecDeque<PathBuf> = roots.into();
    while let Some(file) = queue.pop_front() {
        for child in module_children(&file, edition) {
            if set.contains(&child) && reached.insert(child.clone()) {
                queue.push_back(child);
            }
        }
    }
    let mut orphans: Vec<PathBuf> = set.difference(&reached).cloned().collect();
    orphans.sort();
    // When a large share of the tree looks unreachable, the module structure
    // is almost certainly driven by macros this scan cannot resolve; abort
    // rather than flood the report.
    if orphans.len() >= 5 && orphans.len() * 5 > set.len() {
        return OrphanScan {
            orphans: Vec::new(),
            skipped: false,
            unreliable: true,
        };
    }
    OrphanScan {
        orphans,
        skipped: false,
        unreliable: false,
    }
}

fn module_children(file: &Path, edition: Edition) -> Vec<PathBuf> {
    let Ok(source) = fs::read_to_string(file) else {
        return Vec::new();
    };
    let Some(parent) = file.parent() else {
        return Vec::new();
    };
    let stem = file
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    let base = if matches!(stem, "lib" | "main" | "mod") {
        parent.to_path_buf()
    } else {
        parent.join(stem)
    };

    let parsed = SourceFile::parse(&source, edition);
    let mut children = Vec::new();
    for module in parsed
        .tree()
        .syntax()
        .descendants()
        .filter_map(ast::Module::cast)
    {
        if module.item_list().is_some() {
            continue;
        }
        let Some(name) = module.name().map(|name| name.text().to_string()) else {
            continue;
        };
        let chain = inline_module_chain(&module);
        let mut dir = base.clone();
        for ancestor in &chain {
            dir = dir.join(ancestor);
        }
        // `#[path]` on a non-inline mod resolves against the file's own
        // directory; the stem-appended base applies only inside inline
        // blocks. `#[cfg_attr(..., path = "...")]` keeps the default
        // location as its other branch, so candidates are added, never
        // substituted.
        let path_dir = if chain.is_empty() {
            parent.to_path_buf()
        } else {
            dir.clone()
        };
        for custom in path_attr_values(module.attrs()) {
            children.push(path_dir.join(custom));
        }
        children.push(dir.join(format!("{name}.rs")));
        children.push(dir.join(&name).join("mod.rs"));
    }
    // `cfg_net! { pub mod net; }`-style declarations live inside macro token
    // trees and never become ast::Module nodes; scan the tokens too.
    for mac in parsed
        .tree()
        .syntax()
        .descendants()
        .filter_map(ast::MacroCall::cast)
    {
        let Some(tokens) = mac.token_tree() else {
            continue;
        };
        let chain = macro_inline_chain(&mac);
        let mut dir = base.clone();
        for ancestor in &chain {
            dir = dir.join(ancestor);
        }
        walk_macro_tokens(
            &tokens,
            &dir,
            if chain.is_empty() { parent } else { &dir },
            &mut children,
        );
    }
    children
}

// Walk a macro token stream tracking `mod name { ... }` nesting so that
// `mod x;` and `#[path = "..."] mod x;` declarations resolve against the
// right directory, mirroring rustc's module loading rules.
fn walk_macro_tokens(
    tokens: &ast::TokenTree,
    base: &Path,
    file_dir: &Path,
    children: &mut Vec<PathBuf>,
) {
    let significant: Vec<String> = tokens
        .syntax()
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .map(|token| token.text().to_string())
        .filter(|text| {
            let trimmed = text.trim();
            !trimmed.is_empty() && !trimmed.starts_with("//") && !trimmed.starts_with("/*")
        })
        .collect();
    let mut stack: Vec<(String, usize)> = Vec::new();
    let mut depth = 0usize;
    let mut pending_path: Option<String> = None;
    let mut index = 0usize;
    while index < significant.len() {
        let token = significant[index].as_str();
        match token {
            "{" => depth += 1,
            "}" => {
                depth = depth.saturating_sub(1);
                while stack.last().is_some_and(|(_, open)| *open > depth) {
                    stack.pop();
                }
            }
            "path"
                if significant.get(index + 1).map(String::as_str) == Some("=")
                    && significant
                        .get(index + 2)
                        .is_some_and(|literal| literal.starts_with('"')) =>
            {
                let literal = significant[index + 2].trim_matches('"');
                if !literal.is_empty() {
                    pending_path = Some(literal.to_string());
                }
                index += 3;
                continue;
            }
            "mod"
                if significant
                    .get(index + 1)
                    .is_some_and(|name| crate::parser::is_ident(name)) =>
            {
                let name = significant[index + 1].clone();
                match significant.get(index + 2).map(String::as_str) {
                    Some(";") => {
                        let mut dir = base.to_path_buf();
                        for (segment, _) in &stack {
                            dir = dir.join(segment);
                        }
                        if let Some(custom) = pending_path.take() {
                            let path_dir = if stack.is_empty() {
                                file_dir.to_path_buf()
                            } else {
                                dir.clone()
                            };
                            children.push(path_dir.join(custom));
                        }
                        children.push(dir.join(format!("{name}.rs")));
                        children.push(dir.join(&name).join("mod.rs"));
                        index += 3;
                        continue;
                    }
                    Some("{") => {
                        pending_path = None;
                        depth += 1;
                        stack.push((name, depth));
                        index += 3;
                        continue;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        index += 1;
    }
}

fn macro_inline_chain(mac: &ast::MacroCall) -> Vec<String> {
    let mut chain: Vec<String> = mac
        .syntax()
        .ancestors()
        .skip(1)
        .filter_map(ast::Module::cast)
        .filter(|ancestor| ancestor.item_list().is_some())
        .filter_map(|ancestor| ancestor.name().map(|name| name.text().to_string()))
        .collect();
    chain.reverse();
    chain
}

fn inline_module_chain(module: &ast::Module) -> Vec<String> {
    let mut chain: Vec<String> = module
        .syntax()
        .ancestors()
        .skip(1)
        .filter_map(ast::Module::cast)
        .filter(|ancestor| ancestor.item_list().is_some())
        .filter_map(|ancestor| ancestor.name().map(|name| name.text().to_string()))
        .collect();
    chain.reverse();
    chain
}

fn path_attr_values(attrs: impl Iterator<Item = ast::Attr>) -> Vec<String> {
    let mut values = Vec::new();
    for attr in attrs {
        let text = compact_path(attr.syntax().text().to_string());
        let mut rest = text.as_str();
        while let Some(index) = rest.find("path=\"") {
            rest = &rest[index + 6..];
            let Some(end) = rest.find('"') else {
                break;
            };
            if end > 0 {
                values.push(rest[..end].to_string());
            }
            rest = &rest[end..];
        }
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn touch(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
        fs::write(path, content).expect("write");
    }

    #[test]
    fn unreferenced_file_is_an_orphan_across_module_styles() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        touch(&root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n");
        touch(
            &root.join("src/lib.rs"),
            "mod app;\nmod nested { pub mod inner; }\n#[path = \"special_name.rs\"]\nmod aliased;\n",
        );
        touch(&root.join("src/app.rs"), "pub mod sub;\n");
        touch(&root.join("src/app/sub.rs"), "");
        touch(&root.join("src/nested/inner.rs"), "");
        touch(&root.join("src/special_name.rs"), "");
        touch(&root.join("src/bin/extra.rs"), "fn main() {}\n");
        touch(&root.join("src/legacy.rs"), "pub fn stale() {}\n");
        let files: Vec<PathBuf> = [
            "src/lib.rs",
            "src/app.rs",
            "src/app/sub.rs",
            "src/nested/inner.rs",
            "src/special_name.rs",
            "src/bin/extra.rs",
            "src/legacy.rs",
        ]
        .iter()
        .map(|rel| root.join(rel))
        .collect();
        let scan = scan(root, &files, Edition::Edition2024);
        assert!(!scan.skipped);
        assert_eq!(scan.orphans, vec![root.join("src/legacy.rs")]);
    }

    #[test]
    fn macro_wrapped_mod_declarations_are_resolved() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        touch(&root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n");
        touch(
            &root.join("src/lib.rs"),
            "cfg_net! {\n    pub mod net;\n}\nmod plain;\n",
        );
        touch(&root.join("src/net.rs"), "");
        touch(&root.join("src/plain.rs"), "");
        let files: Vec<PathBuf> = ["src/lib.rs", "src/net.rs", "src/plain.rs"]
            .iter()
            .map(|rel| root.join(rel))
            .collect();
        let scan = scan(root, &files, Edition::Edition2024);
        assert!(!scan.skipped && !scan.unreliable);
        assert!(scan.orphans.is_empty(), "{:?}", scan.orphans);
    }

    #[test]
    fn tokio_style_macro_and_path_dispatch_is_resolved() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        touch(&root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n");
        touch(
            &root.join("src/lib.rs"),
            "mod io;\nmod atomic_u64;\nmod signal;\n",
        );
        // #[path] inside a macro token tree, non-mod-rs file: resolves
        // against the file's own directory (src/), not src/atomic_u64/.
        touch(
            &root.join("src/atomic_u64.rs"),
            "cfg_x! {\n    #[path = \"atomic_native.rs\"]\n    mod imp;\n}\n",
        );
        touch(&root.join("src/atomic_native.rs"), "");
        // plain mod decl nested in an inline block inside a macro.
        touch(
            &root.join("src/io.rs"),
            "cfg_aio! {\n    pub mod bsd {\n        mod poll_aio;\n    }\n}\n",
        );
        touch(&root.join("src/io/bsd/poll_aio.rs"), "");
        // AST-level #[path] on a non-inline mod in a non-mod-rs file.
        touch(
            &root.join("src/signal.rs"),
            "#[path = \"signal/sys.rs\"]\nmod imp;\n",
        );
        touch(&root.join("src/signal/sys.rs"), "");
        let files: Vec<PathBuf> = [
            "src/lib.rs",
            "src/atomic_u64.rs",
            "src/atomic_native.rs",
            "src/io.rs",
            "src/io/bsd/poll_aio.rs",
            "src/signal.rs",
            "src/signal/sys.rs",
        ]
        .iter()
        .map(|rel| root.join(rel))
        .collect();
        let scan = scan(root, &files, Edition::Edition2024);
        assert!(!scan.skipped && !scan.unreliable);
        assert!(scan.orphans.is_empty(), "{:?}", scan.orphans);
    }

    #[test]
    fn mostly_unreachable_tree_aborts_as_unreliable() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        touch(&root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n");
        touch(&root.join("src/lib.rs"), "");
        let mut files = vec![root.join("src/lib.rs")];
        for index in 0..5 {
            let path = root.join(format!("src/unlinked_{index}.rs"));
            touch(&path, "");
            files.push(path);
        }
        let scan = scan(root, &files, Edition::Edition2024);
        assert!(scan.unreliable);
        assert!(scan.orphans.is_empty());
    }

    #[test]
    fn scan_is_skipped_without_single_crate_layout() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        touch(&root.join("member/src/lib.rs"), "");
        let scan = scan(
            root,
            &[root.join("member/src/lib.rs")],
            Edition::Edition2024,
        );
        assert!(scan.skipped);
        assert!(scan.orphans.is_empty());
    }
}
