use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Top-level probe data for a single crate analysis run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeData {
    pub crate_name: String,
    pub functions: Vec<FunctionProbe>,
}

/// Analysis data for a single function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionProbe {
    /// Fully qualified function name.
    pub name: String,
    /// Compiler definition path.
    pub def_path: String,
    /// Source file path.
    pub file: String,
    /// Line range in source.
    pub line_start: u32,
    pub line_end: u32,
    /// Number of basic blocks in the MIR.
    pub basic_block_count: usize,
    /// Total number of MIR statements.
    pub statement_count: usize,
    /// Count of each terminator kind.
    pub terminator_kinds: HashMap<String, usize>,
    /// Functions called from this function.
    pub call_targets: Vec<String>,
    /// Whether any loops were detected.
    pub has_loops: bool,
    /// Estimated number of move operations.
    pub move_count: usize,
    /// Estimated number of Clone calls.
    pub clone_count: usize,
    /// Estimated number of Drop calls.
    pub drop_count: usize,
    /// Heuristic complexity score.
    pub complexity_score: f64,
}

impl FunctionProbe {
    pub fn compute_complexity(&mut self) {
        let branch_count = self
            .terminator_kinds
            .iter()
            .filter(|(k, _)| k.as_str() == "SwitchInt" || k.as_str() == "FalseEdge")
            .map(|(_, v)| *v)
            .sum::<usize>();

        let loop_bonus = if self.has_loops { 5.0 } else { 0.0 };
        let ownership_weight = (self.move_count as f64 * 0.5)
            + (self.clone_count as f64 * 2.0)
            + (self.drop_count as f64 * 1.0);

        self.complexity_score = self.basic_block_count as f64
            + branch_count as f64 * 1.5
            + loop_bonus
            + ownership_weight;
    }
}

impl ProbeData {
    pub fn sort_by_complexity(&mut self) {
        self.functions.sort_by(|a, b| {
            b.complexity_score
                .partial_cmp(&a.complexity_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    pub fn function_count(&self) -> usize {
        self.functions.len()
    }

    pub fn total_basic_blocks(&self) -> usize {
        self.functions.iter().map(|f| f.basic_block_count).sum()
    }

    pub fn functions_with_loops(&self) -> usize {
        self.functions.iter().filter(|f| f.has_loops).count()
    }

    pub fn average_complexity(&self) -> f64 {
        if self.functions.is_empty() {
            return 0.0;
        }
        let total: f64 = self.functions.iter().map(|f| f.complexity_score).sum();
        total / self.functions.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_function(name: &str, blocks: usize, loops: bool, clones: usize) -> FunctionProbe {
        let mut f = FunctionProbe {
            name: name.to_string(),
            def_path: name.to_string(),
            file: "src/main.rs".to_string(),
            line_start: 1,
            line_end: 10,
            basic_block_count: blocks,
            statement_count: blocks * 3,
            terminator_kinds: HashMap::new(),
            call_targets: vec![],
            has_loops: loops,
            move_count: 0,
            clone_count: clones,
            drop_count: 0,
            complexity_score: 0.0,
        };
        f.compute_complexity();
        f
    }

    #[test]
    fn complexity_ordering() {
        let f1 = make_function("simple", 2, false, 0);
        let f2 = make_function("complex", 10, true, 5);
        assert!(f2.complexity_score > f1.complexity_score);
    }

    #[test]
    fn sort_by_complexity() {
        let mut data = ProbeData {
            crate_name: "test".to_string(),
            functions: vec![
                make_function("a", 1, false, 0),
                make_function("c", 20, true, 10),
                make_function("b", 5, false, 2),
            ],
        };
        data.sort_by_complexity();
        assert_eq!(data.functions[0].name, "c");
        assert_eq!(data.functions[2].name, "a");
    }

    #[test]
    fn statistics() {
        let data = ProbeData {
            crate_name: "test".to_string(),
            functions: vec![
                make_function("a", 4, false, 0),
                make_function("b", 6, true, 0),
            ],
        };
        assert_eq!(data.function_count(), 2);
        assert_eq!(data.total_basic_blocks(), 10);
        assert_eq!(data.functions_with_loops(), 1);
    }
}
