use std::collections::HashSet;

use rustc_middle::mir::{BasicBlock, Body};

/// Detect basic blocks that are inside a loop by finding back-edges in the CFG,
/// then marking all blocks in the natural loop (all blocks that can reach the
/// back-edge tail without going through the loop header).
pub fn detect_loop_blocks(body: &Body<'_>) -> HashSet<BasicBlock> {
    let num_blocks = body.basic_blocks.len();
    let mut visited = vec![false; num_blocks];
    let mut on_stack = vec![false; num_blocks];
    let mut back_edges = Vec::new();

    // Phase 1: Find all back-edges via DFS
    for bb in body.basic_blocks.indices() {
        if !visited[bb.index()] {
            dfs_find_back_edges(body, bb, &mut visited, &mut on_stack, &mut back_edges);
        }
    }

    // Phase 2: For each back-edge (tail -> header), compute the natural loop
    let mut loop_blocks = HashSet::new();
    for &(header, tail) in &back_edges {
        collect_natural_loop(body, header, tail, &mut loop_blocks);
    }

    loop_blocks
}

fn dfs_find_back_edges(
    body: &Body<'_>,
    bb: BasicBlock,
    visited: &mut [bool],
    on_stack: &mut [bool],
    back_edges: &mut Vec<(BasicBlock, BasicBlock)>,
) {
    visited[bb.index()] = true;
    on_stack[bb.index()] = true;

    let terminator = &body.basic_blocks[bb].terminator();
    for succ in terminator.successors() {
        if on_stack[succ.index()] {
            // Back-edge: bb -> succ (succ is the loop header)
            back_edges.push((succ, bb));
        } else if !visited[succ.index()] {
            dfs_find_back_edges(body, succ, visited, on_stack, back_edges);
        }
    }

    on_stack[bb.index()] = false;
}

/// Compute the natural loop for a back-edge (tail -> header).
/// The natural loop consists of the header plus all blocks that can reach
/// the tail without passing through the header (reverse BFS from tail).
fn collect_natural_loop(
    body: &Body<'_>,
    header: BasicBlock,
    tail: BasicBlock,
    loop_blocks: &mut HashSet<BasicBlock>,
) {
    loop_blocks.insert(header);
    if header == tail {
        return; // Self-loop: only the header block
    }

    loop_blocks.insert(tail);
    let mut worklist = vec![tail];

    // Reverse walk: for each block in the worklist, find predecessors
    // and add them to the loop if not already included.
    while let Some(bb) = worklist.pop() {
        // Find predecessors of bb by scanning all blocks
        for pred_bb in body.basic_blocks.indices() {
            if loop_blocks.contains(&pred_bb) {
                continue;
            }
            let term = body.basic_blocks[pred_bb].terminator();
            if term.successors().any(|s| s == bb) {
                loop_blocks.insert(pred_bb);
                if pred_bb != header {
                    worklist.push(pred_bb);
                }
            }
        }
    }
}
