use crate::analysis::call_visitor::extract_call_sites;
use crate::analysis::parser::{self, module_path_from_file};
use crate::error::{Result, RustMorphError};
use crate::types::{CallSite, FunctionSignature};
use std::path::Path;
use walkdir::WalkDir;

/// Aggregated analysis result for a whole project.
#[derive(Debug, Default)]
pub struct ProjectAnalysis {
    pub functions: Vec<FunctionSignature>,
    pub call_sites: Vec<CallSite>,
    pub file_count: usize,
}

impl ProjectAnalysis {
    /// Walk a project directory, parse all `.rs` files, and collect results.
    pub fn analyze(root: &Path) -> Result<Self> {
        let mut result = ProjectAnalysis::default();

        for entry in WalkDir::new(root)
            .into_iter()
            .filter_entry(|e| !is_hidden(e) && !is_target_dir(e))
        {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "rs") {
                result.file_count += 1;

                // Read and parse the file once.
                let source = std::fs::read_to_string(path).map_err(|e| RustMorphError::Parse {
                    path: path.display().to_string(),
                    reason: e.to_string(),
                });
                let source = match source {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("warning: skipping {}: {e}", path.display());
                        continue;
                    }
                };

                let ast = match syn::parse_file(&source) {
                    Ok(ast) => ast,
                    Err(e) => {
                        eprintln!("warning: skipping {}: {e}", path.display());
                        continue;
                    }
                };

                // Extract function signatures.
                let module_path = module_path_from_file(path);
                let file_result = parser::parse_ast(path, &module_path, &ast);
                result.functions.extend(file_result.functions);

                // Extract call sites from the same AST.
                let calls = extract_call_sites(path, &ast);
                result.call_sites.extend(calls);
            }
        }

        Ok(result)
    }
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    // Skip hidden dirs/files, but not the root directory itself.
    entry.depth() > 0
        && entry
            .file_name()
            .to_str()
            .is_some_and(|s| s.starts_with('.'))
}

fn is_target_dir(entry: &walkdir::DirEntry) -> bool {
    entry.file_type().is_dir() && entry.file_name().to_str().is_some_and(|s| s == "target")
}
