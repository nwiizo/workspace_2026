use std::collections::HashMap;

use rustc_middle::mir::visit::Visitor;
use rustc_middle::mir::{self, BasicBlock, Body, Location, Operand, Rvalue, TerminatorKind};
use rustc_middle::ty::{self, TyCtxt};

use rustprobe_analysis::FunctionProbe;

pub(crate) struct MirAnalyzer<'tcx> {
    tcx: TyCtxt<'tcx>,
    terminator_kinds: HashMap<String, usize>,
    call_targets: Vec<String>,
    move_count: usize,
    clone_count: usize,
    drop_count: usize,
    statement_count: usize,
}

impl<'tcx> MirAnalyzer<'tcx> {
    pub(crate) fn new(tcx: TyCtxt<'tcx>) -> Self {
        Self {
            tcx,
            terminator_kinds: HashMap::new(),
            call_targets: Vec::new(),
            move_count: 0,
            clone_count: 0,
            drop_count: 0,
            statement_count: 0,
        }
    }

    pub(crate) fn analyze(
        &mut self,
        body: &Body<'tcx>,
        name: String,
        def_path: String,
        file: String,
        line_start: u32,
        line_end: u32,
    ) -> FunctionProbe {
        self.visit_body(body);

        let has_loops = detect_loops(body);

        let mut probe = FunctionProbe {
            name,
            def_path,
            file,
            line_start,
            line_end,
            basic_block_count: body.basic_blocks.len(),
            statement_count: self.statement_count,
            terminator_kinds: self.terminator_kinds.clone(),
            call_targets: self.call_targets.clone(),
            has_loops,
            move_count: self.move_count,
            clone_count: self.clone_count,
            drop_count: self.drop_count,
            complexity_score: 0.0,
        };
        probe.compute_complexity();
        probe
    }

    fn is_clone_call(name: &str) -> bool {
        if name.contains("Arc::clone") || name.contains("Rc::clone") {
            return false;
        }
        name.ends_with("::clone") || name.contains("core::clone::Clone")
    }
}

impl<'tcx> Visitor<'tcx> for MirAnalyzer<'tcx> {
    fn visit_statement(&mut self, statement: &mir::Statement<'tcx>, _location: Location) {
        self.statement_count += 1;

        if let mir::StatementKind::Assign(assign) = &statement.kind {
            let (_, ref rvalue) = **assign;
            if let Rvalue::Use(Operand::Move(_)) = rvalue {
                self.move_count += 1;
            }
        }
    }

    fn visit_terminator(&mut self, terminator: &mir::Terminator<'tcx>, _location: Location) {
        let kind_name = terminator_kind_name(&terminator.kind);
        *self.terminator_kinds.entry(kind_name).or_insert(0) += 1;

        match &terminator.kind {
            TerminatorKind::Call {
                func: Operand::Constant(constant),
                args,
                ..
            } => {
                for arg in args {
                    if matches!(arg.node, Operand::Move(_)) {
                        self.move_count += 1;
                    }
                }

                let ty = constant.const_.ty();
                if let ty::FnDef(def_id, _) = ty.kind() {
                    let name = self.tcx.def_path_str(*def_id);
                    if Self::is_clone_call(&name) {
                        self.clone_count += 1;
                    }
                    self.call_targets.push(name);
                }
            }
            TerminatorKind::Drop { .. } => {
                self.drop_count += 1;
            }
            _ => {}
        }
    }
}

/// Detect loops via iterative DFS (avoids stack overflow on large MIR bodies).
fn detect_loops(body: &Body<'_>) -> bool {
    let num_blocks = body.basic_blocks.len();
    if num_blocks == 0 {
        return false;
    }

    #[derive(Clone, Copy)]
    enum Action {
        Enter(BasicBlock),
        Exit(BasicBlock),
    }

    let mut visited = vec![false; num_blocks];
    let mut on_stack = vec![false; num_blocks];
    let mut stack = vec![Action::Enter(BasicBlock::from_u32(0))];

    while let Some(action) = stack.pop() {
        match action {
            Action::Enter(bb) => {
                let idx = bb.index();
                if visited[idx] {
                    continue;
                }
                visited[idx] = true;
                on_stack[idx] = true;

                stack.push(Action::Exit(bb));

                let data = &body.basic_blocks[bb];
                if let Some(ref term) = data.terminator {
                    for succ in term.successors() {
                        let succ_idx = succ.index();
                        if on_stack[succ_idx] {
                            return true;
                        }
                        if !visited[succ_idx] {
                            stack.push(Action::Enter(succ));
                        }
                    }
                }
            }
            Action::Exit(bb) => {
                on_stack[bb.index()] = false;
            }
        }
    }

    false
}

fn terminator_kind_name(kind: &TerminatorKind<'_>) -> String {
    match kind {
        TerminatorKind::Goto { .. } => "Goto".to_string(),
        TerminatorKind::SwitchInt { .. } => "SwitchInt".to_string(),
        TerminatorKind::Return => "Return".to_string(),
        TerminatorKind::Unreachable => "Unreachable".to_string(),
        TerminatorKind::Drop { .. } => "Drop".to_string(),
        TerminatorKind::Call { .. } => "Call".to_string(),
        TerminatorKind::Assert { .. } => "Assert".to_string(),
        TerminatorKind::Yield { .. } => "Yield".to_string(),
        TerminatorKind::FalseEdge { .. } => "FalseEdge".to_string(),
        TerminatorKind::FalseUnwind { .. } => "FalseUnwind".to_string(),
        TerminatorKind::UnwindResume => "UnwindResume".to_string(),
        TerminatorKind::UnwindTerminate(_) => "UnwindTerminate".to_string(),
        TerminatorKind::CoroutineDrop => "CoroutineDrop".to_string(),
        TerminatorKind::InlineAsm { .. } => "InlineAsm".to_string(),
        TerminatorKind::TailCall { .. } => "TailCall".to_string(),
    }
}
