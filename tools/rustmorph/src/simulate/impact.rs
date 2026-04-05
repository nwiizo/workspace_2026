use crate::graph::{DepGraph, EdgeKind};
use crate::simulate::score::SafetyScore;
use crate::simulate::transform::Transform;
use crate::types::SpanInfo;
use petgraph::graph::NodeIndex;
use serde::{Deserialize, Serialize};

/// A single required change at a call site.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredChange {
    /// Where the change is needed.
    pub span: SpanInfo,
    /// The calling function name.
    pub caller: String,
    /// Description of what needs to change.
    pub description: String,
    /// The kind of change.
    pub kind: ChangeKind,
    /// Original expression.
    pub original: String,
    /// Suggested replacement.
    pub suggested: String,
}

/// Categories of required changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChangeKind {
    /// Need to add `.clone()`.
    AddClone,
    /// Value will be moved; subsequent uses become invalid.
    ConvertToMove,
    /// Need to add `&` or `&mut`.
    AddBorrow,
    /// Need to remove `&` / `&mut`.
    RemoveBorrow,
    /// Lifetime annotation must be added.
    AddLifetime,
    /// Mutable borrow conflict detected.
    MutBorrowConflict,
    /// Need to add `.to_string()` / `.to_owned()`.
    AddToOwned,
    /// Need to add `.as_str()` / `.as_slice()` / borrow.
    AddAsRef,
}

impl std::fmt::Display for ChangeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AddClone => write!(f, "Clone追加"),
            Self::ConvertToMove => write!(f, "move変換"),
            Self::AddBorrow => write!(f, "借用追加"),
            Self::RemoveBorrow => write!(f, "借用除去"),
            Self::AddLifetime => write!(f, "ライフタイム注釈追加"),
            Self::MutBorrowConflict => write!(f, "可変借用競合"),
            Self::AddToOwned => write!(f, "所有権取得追加"),
            Self::AddAsRef => write!(f, "参照変換追加"),
        }
    }
}

/// Full impact analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Impact {
    /// The function being changed.
    pub target_function: String,
    /// The transform being applied.
    pub transform_description: String,
    /// The parameter index being changed.
    pub param_index: usize,
    /// All required changes at call sites.
    pub changes: Vec<RequiredChange>,
    /// Number of files affected.
    pub affected_files: usize,
    /// Safety score.
    pub safety_score: SafetyScore,
}

impl Impact {
    /// Total number of affected call sites.
    pub fn affected_count(&self) -> usize {
        self.changes.len()
    }

    /// Count changes by kind.
    pub fn count_by_kind(&self, kind: ChangeKind) -> usize {
        self.changes.iter().filter(|c| c.kind == kind).count()
    }
}

/// Performs impact analysis on a dependency graph.
pub struct ImpactAnalyzer<'g> {
    graph: &'g DepGraph,
}

impl<'g> ImpactAnalyzer<'g> {
    pub fn new(graph: &'g DepGraph) -> Self {
        Self { graph }
    }

    /// Simulate a transform on a function parameter and return the impact.
    pub fn analyze(
        &self,
        function_name: &str,
        param_index: usize,
        transform: &Transform,
    ) -> Option<Impact> {
        let node_idx = self.graph.find_function(function_name)?;
        let sig = self.graph.get_signature(node_idx)?;

        if param_index >= sig.params.len() {
            return None;
        }

        let param = &sig.params[param_index];
        let source_ownership = param.type_info.ownership;

        // Verify the transform is applicable.
        if source_ownership != transform.source_ownership() {
            return None;
        }

        let changes = self.compute_call_site_changes(node_idx, param_index, transform);

        let affected_files = {
            let mut files: Vec<_> = changes.iter().map(|c| &c.span.file).collect();
            files.sort();
            files.dedup();
            files.len()
        };

        let safety_score = SafetyScore::compute(&changes, transform);

        Some(Impact {
            target_function: function_name.to_string(),
            transform_description: transform.to_string(),
            param_index,
            changes,
            affected_files,
            safety_score,
        })
    }

    fn compute_call_site_changes(
        &self,
        target: NodeIndex,
        param_index: usize,
        transform: &Transform,
    ) -> Vec<RequiredChange> {
        let mut changes = Vec::new();

        for (caller_idx, edge) in self.graph.callers(target) {
            if edge.param_index != param_index {
                continue;
            }

            let caller_name = self
                .graph
                .get_signature(caller_idx)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "unknown".to_string());

            if let Some(change) = self.derive_change(
                &caller_name,
                edge.kind,
                &edge.call_site,
                &edge.arg_expr,
                transform,
            ) {
                changes.push(change);
            }
        }

        changes
    }

    fn derive_change(
        &self,
        caller: &str,
        current_edge: EdgeKind,
        span: &SpanInfo,
        arg_expr: &str,
        transform: &Transform,
    ) -> Option<RequiredChange> {
        let (kind, description, suggested) = match transform {
            // &T → T: callers currently pass &x, now need to pass x (move) or x.clone()
            Transform::RefToOwned => match current_edge {
                EdgeKind::Borrows => {
                    // Remove the & and either move or clone.
                    let inner = arg_expr.strip_prefix('&').unwrap_or(arg_expr).trim();
                    (
                        ChangeKind::ConvertToMove,
                        format!("{arg_expr} → {inner} (move、以降使用不可) or {inner}.clone()"),
                        inner.to_string(),
                    )
                }
                EdgeKind::Clones => {
                    // Already cloning — just remove the & if present.
                    (
                        ChangeKind::RemoveBorrow,
                        format!("{arg_expr} はClone済み、借用除去のみ"),
                        arg_expr.to_string(),
                    )
                }
                _ => return None,
            },
            // T → &T: callers currently pass x (move), now need to pass &x
            Transform::OwnedToRef => match current_edge {
                EdgeKind::Owns => (
                    ChangeKind::AddBorrow,
                    format!("{arg_expr} → &{arg_expr}"),
                    format!("&{arg_expr}"),
                ),
                EdgeKind::Clones => (
                    ChangeKind::AddBorrow,
                    format!("{arg_expr} → &{arg_expr} (clone不要に)"),
                    format!("&{}", arg_expr.strip_suffix(".clone()").unwrap_or(arg_expr)),
                ),
                _ => return None,
            },
            // &T → &mut T: callers need to pass &mut instead of &
            Transform::RefToMutRef => match current_edge {
                EdgeKind::Borrows => {
                    // quote! produces "& expr", so handle both "& " and "&" patterns.
                    let suggested = if let Some(rest) = arg_expr.strip_prefix("& ") {
                        format!("& mut {rest}")
                    } else if let Some(rest) = arg_expr.strip_prefix('&') {
                        format!("&mut {rest}")
                    } else {
                        format!("&mut {arg_expr}")
                    };
                    (
                        ChangeKind::MutBorrowConflict,
                        format!("{arg_expr} → &mut (排他借用制約を確認)"),
                        suggested,
                    )
                }
                _ => return None,
            },
            // &mut T → &T: callers can downgrade to shared borrow
            Transform::MutRefToRef => match current_edge {
                EdgeKind::MutBorrows => {
                    // quote! produces "& mut expr", so handle both patterns.
                    let suggested = if let Some(rest) = arg_expr.strip_prefix("& mut ") {
                        format!("& {rest}")
                    } else if let Some(rest) = arg_expr.strip_prefix("&mut ") {
                        format!("&{rest}")
                    } else {
                        arg_expr.to_string()
                    };
                    (
                        ChangeKind::RemoveBorrow,
                        format!("{arg_expr} → 共有借用に変更"),
                        suggested,
                    )
                }
                _ => return None,
            },
            // String → &str: callers pass owned String, now pass &str
            Transform::StringToStr => match current_edge {
                EdgeKind::Owns => (
                    ChangeKind::AddAsRef,
                    format!("{arg_expr} → &{arg_expr} or {arg_expr}.as_str()"),
                    format!("{arg_expr}.as_str()"),
                ),
                EdgeKind::Clones => (
                    ChangeKind::AddAsRef,
                    format!("{arg_expr} → clone不要、&str として渡す"),
                    format!(
                        "{}.as_str()",
                        arg_expr.strip_suffix(".clone()").unwrap_or(arg_expr)
                    ),
                ),
                _ => return None,
            },
            // &str → String: callers pass &str, now need to pass String
            Transform::StrToString => match current_edge {
                EdgeKind::Borrows => (
                    ChangeKind::AddToOwned,
                    format!("{arg_expr} → {arg_expr}.to_string()"),
                    format!(
                        "{}.to_string()",
                        arg_expr.strip_prefix('&').unwrap_or(arg_expr).trim()
                    ),
                ),
                _ => return None,
            },
            // Vec<T> → &[T]: callers pass owned Vec, now pass slice
            Transform::VecToSlice => match current_edge {
                EdgeKind::Owns => (
                    ChangeKind::AddAsRef,
                    format!("{arg_expr} → &{arg_expr}"),
                    format!("&{arg_expr}"),
                ),
                _ => return None,
            },
            // &[T] → Vec<T>: callers pass slice, now need owned Vec
            Transform::SliceToVec => match current_edge {
                EdgeKind::Borrows => (
                    ChangeKind::AddToOwned,
                    format!("{arg_expr} → {arg_expr}.to_vec()"),
                    format!("{arg_expr}.to_vec()"),
                ),
                _ => return None,
            },
            // Box<T> → T: callers pass Box, now pass T directly
            Transform::BoxToInline => match current_edge {
                EdgeKind::Owns => (
                    ChangeKind::RemoveBorrow,
                    format!("{arg_expr} → *{arg_expr} (unbox)"),
                    format!("*{arg_expr}"),
                ),
                _ => return None,
            },
        };

        Some(RequiredChange {
            span: span.clone(),
            caller: caller.to_string(),
            description,
            kind,
            original: arg_expr.to_string(),
            suggested,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::NodeKind;
    use crate::types::*;
    use std::path::PathBuf;

    fn span(line: usize) -> SpanInfo {
        SpanInfo {
            file: PathBuf::from("test.rs"),
            line,
            col: 0,
        }
    }

    fn make_param(name: &str, ownership: OwnershipKind) -> ParamInfo {
        ParamInfo {
            name: name.to_string(),
            type_info: TypeInfo {
                ownership,
                raw: format!("{ownership} {name}"),
                inner: name.to_string(),
                lifetime: None,
                is_generic: false,
            },
            span: span(1),
        }
    }

    fn make_sig(name: &str, params: Vec<ParamInfo>) -> FunctionSignature {
        FunctionSignature {
            name: name.to_string(),
            short_name: name.to_string(),
            impl_target: None,
            params,
            return_type: None,
            span: span(1),
        }
    }

    #[test]
    fn ref_to_owned_impact() {
        let mut graph = DepGraph::new();

        let process_sig = make_sig("process", vec![make_param("data", OwnershipKind::Ref)]);
        let caller_sig = make_sig("caller", vec![]);

        graph.add_function(process_sig, NodeKind::Function);
        graph.add_function(caller_sig, NodeKind::Function);
        graph.add_call(&CallSite {
            caller: "caller".to_string(),
            callee: "process".to_string(),
            args: vec![CallArg {
                expr: "&config".to_string(),
                usage: ArgUsage::Borrow,
            }],
            span: span(10),
        });

        let analyzer = ImpactAnalyzer::new(&graph);
        let impact = analyzer
            .analyze("process", 0, &Transform::RefToOwned)
            .unwrap();

        assert_eq!(impact.changes.len(), 1);
        assert_eq!(impact.changes[0].kind, ChangeKind::ConvertToMove);
    }

    #[test]
    fn owned_to_ref_impact() {
        let mut graph = DepGraph::new();

        let process_sig = make_sig("process", vec![make_param("data", OwnershipKind::Owned)]);
        let caller_sig = make_sig("caller", vec![]);

        graph.add_function(process_sig, NodeKind::Function);
        graph.add_function(caller_sig, NodeKind::Function);
        graph.add_call(&CallSite {
            caller: "caller".to_string(),
            callee: "process".to_string(),
            args: vec![CallArg {
                expr: "config".to_string(),
                usage: ArgUsage::Move,
            }],
            span: span(10),
        });

        let analyzer = ImpactAnalyzer::new(&graph);
        let impact = analyzer
            .analyze("process", 0, &Transform::OwnedToRef)
            .unwrap();

        assert_eq!(impact.changes.len(), 1);
        assert_eq!(impact.changes[0].kind, ChangeKind::AddBorrow);
        assert_eq!(impact.changes[0].suggested, "&config");
    }

    #[test]
    fn mismatched_transform_returns_none() {
        let mut graph = DepGraph::new();
        let sig = make_sig("f", vec![make_param("x", OwnershipKind::Owned)]);
        graph.add_function(sig, NodeKind::Function);

        let analyzer = ImpactAnalyzer::new(&graph);
        // RefToOwned requires Ref, but param is Owned
        assert!(analyzer.analyze("f", 0, &Transform::RefToOwned).is_none());
    }
}
