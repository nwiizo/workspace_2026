use std::collections::HashSet;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

use walkdir::WalkDir;

use crate::{CoreError, Result};

#[derive(Clone, Copy, Default)]
pub struct RustFileWalkerOptions<'a> {
    pub prefer_src: bool,
    pub on_no_files: Option<&'a dyn Fn(&Path)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoRustFiles;

pub fn rust_files(root: &Path, options: RustFileWalkerOptions<'_>) -> Result<Vec<PathBuf>> {
    if root.is_file() {
        let files = if root.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            vec![root.to_path_buf()]
        } else {
            Vec::new()
        };
        if files.is_empty()
            && let Some(hook) = options.on_no_files
        {
            hook(root);
        }
        return Ok(files);
    }
    let scan_root = if options.prefer_src && root.join("src").is_dir() {
        root.join("src")
    } else {
        root.to_path_buf()
    };
    let mut files = Vec::new();
    for entry in WalkDir::new(&scan_root).follow_links(false) {
        let entry = entry.map_err(|source| CoreError::Walk {
            path: scan_root.clone(),
            source,
        })?;
        if entry.file_type().is_dir() && is_excluded_dir(entry.path()) {
            continue;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if is_excluded_dir(path) {
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path.to_path_buf());
        }
    }
    remove_gitignored(&scan_root, &mut files);
    remove_simple_gitignored(root, &mut files);
    files.sort();
    if files.is_empty()
        && let Some(hook) = options.on_no_files
    {
        hook(root);
    }
    Ok(files)
}

pub fn relative_path(root: &Path, path: &Path) -> PathBuf {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if root.is_file()
        && path == root
        && let Some(name) = path.file_name()
    {
        return PathBuf::from(name);
    }
    match path.strip_prefix(&root) {
        Ok(relative) if relative.as_os_str().is_empty() => path
            .file_name()
            .map(PathBuf::from)
            .unwrap_or_else(|| path.clone()),
        Ok(relative) => relative.to_path_buf(),
        Err(_) => path,
    }
}

pub fn relative_path_string(root: &Path, path: &Path) -> String {
    relative_path(root, path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn remove_simple_gitignored(root: &Path, files: &mut Vec<PathBuf>) {
    let source = std::fs::read_to_string(root.join(".gitignore")).unwrap_or_default();
    let patterns = source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with('!'))
        .map(|line| line.trim_start_matches('/').to_string())
        .collect::<Vec<_>>();
    if patterns.is_empty() {
        return;
    }
    files.retain(|path| {
        let Ok(relative) = path.strip_prefix(root) else {
            return true;
        };
        let relative = relative.to_string_lossy().replace('\\', "/");
        !patterns.iter().any(|pattern| {
            if let Some(dir) = pattern.strip_suffix('/') {
                relative == dir || relative.starts_with(&format!("{dir}/"))
            } else {
                relative == *pattern || relative.starts_with(&format!("{pattern}/"))
            }
        })
    });
}

fn is_excluded_dir(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(component, Component::Normal(name) if name == "target" || name == ".git")
    })
}

fn remove_gitignored(scan_root: &Path, files: &mut Vec<PathBuf>) {
    let Ok(mut child) = Command::new("git")
        .args(["check-ignore", "--stdin"])
        .current_dir(scan_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return;
    };
    if let Some(stdin) = child.stdin.as_mut() {
        for file in files.iter() {
            let _ = writeln!(stdin, "{}", file.display());
        }
    }
    let Ok(output) = child.wait_with_output() else {
        return;
    };
    let ignored = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(PathBuf::from)
        .collect::<HashSet<_>>();
    if ignored.is_empty() {
        return;
    }
    files.retain(|file| !ignored.contains(file));
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicBool, Ordering};

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn walker_respects_gitignore_and_excludes_target_and_git() {
        let dir = TempDir::new().expect("tempdir");
        fs::create_dir_all(dir.path().join("src")).expect("src");
        fs::create_dir_all(dir.path().join("target/debug")).expect("target");
        fs::create_dir_all(dir.path().join(".git/hooks")).expect("git");
        fs::create_dir_all(dir.path().join("vendor")).expect("vendor");
        fs::write(dir.path().join("src/lib.rs"), "").expect("lib");
        fs::write(dir.path().join("target/debug/build.rs"), "").expect("target file");
        fs::write(dir.path().join(".git/hooks/hook.rs"), "").expect("git file");
        fs::write(dir.path().join("vendor/ignored.rs"), "").expect("ignored");
        fs::write(dir.path().join(".gitignore"), "vendor/\n").expect("gitignore");
        let files = rust_files(dir.path(), RustFileWalkerOptions::default()).expect("walk");
        assert_eq!(files, vec![dir.path().join("src/lib.rs")]);
    }

    #[test]
    fn no_file_hook_runs() {
        let dir = TempDir::new().expect("tempdir");
        let called = AtomicBool::new(false);
        let hook = |_: &Path| called.store(true, Ordering::SeqCst);
        let files = rust_files(
            dir.path(),
            RustFileWalkerOptions {
                prefer_src: false,
                on_no_files: Some(&hook),
            },
        )
        .expect("walk");
        assert!(files.is_empty());
        assert!(called.load(Ordering::SeqCst));
    }

    #[test]
    fn file_root_relative_path_uses_file_name() {
        let dir = TempDir::new().expect("tempdir");
        let file = dir.path().join("src/lib.rs");
        fs::create_dir_all(file.parent().expect("parent")).expect("src");
        fs::write(&file, "").expect("file");
        assert_eq!(relative_path(&file, &file), PathBuf::from("lib.rs"));
        assert_eq!(relative_path_string(&file, &file), "lib.rs");
    }
}
