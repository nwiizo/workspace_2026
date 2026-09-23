use ra_ap_syntax::ast::{self, AstNode};
use ra_ap_syntax::{SyntaxKind, SyntaxNode};

use crate::issue::{IssueType, RiskAxis};

use super::{
    CONDITION_HIGH, CONDITION_LOW, CONDITION_MEDIUM, FileContext, Finding, compact_path,
    excluded_test_context,
};

pub(crate) fn detect(root: &SyntaxNode, ctx: &FileContext<'_>, findings: &mut Vec<Finding>) {
    if ctx.config.is_allowed(IssueType::ChainedCast) {
        return;
    }
    for cast in root.descendants().filter_map(ast::CastExpr::cast) {
        if excluded_test_context(cast.syntax()) || parent_is_cast(cast.syntax()) {
            continue;
        }
        let types = cast_chain(&cast);
        if types.len() < 2 {
            continue;
        }
        let condition = chain_condition(&types);
        let chain = types.join(" as ");
        findings.push(Finding {
            issue_type: IssueType::ChainedCast,
            risk: RiskAxis::DataLoss,
            condition,
            file: ctx.file.to_path_buf(),
            rel_path: ctx.rel_path.to_string(),
            line: ctx.line(cast.syntax().text_range()),
            function: ctx.identity_for(cast.syntax()),
            target: types.join("->"),
            message: format!(
                "chained cast `as {chain}` fabricates type evidence through {} steps",
                types.len()
            ),
            remediation:
                "Cast once to the intended type, or use TryFrom/From with an explicit failure path."
                    .to_string(),
        });
    }
}

fn parent_is_cast(node: &SyntaxNode) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        match parent.kind() {
            SyntaxKind::PAREN_EXPR => current = parent.parent(),
            SyntaxKind::CAST_EXPR => return true,
            _ => return false,
        }
    }
    false
}

fn cast_chain(cast: &ast::CastExpr) -> Vec<String> {
    let mut types = Vec::new();
    let mut current = cast.clone();
    loop {
        if let Some(ty) = current.ty() {
            types.push(compact_path(ty.syntax().text().to_string()));
        }
        match unwrap_parens(current.expr()) {
            Some(ast::Expr::CastExpr(inner)) => current = inner,
            _ => break,
        }
    }
    types.reverse();
    types
}

fn unwrap_parens(expr: Option<ast::Expr>) -> Option<ast::Expr> {
    let mut current = expr?;
    while let ast::Expr::ParenExpr(paren) = current {
        current = paren.expr()?;
    }
    Some(current)
}

fn chain_condition(types: &[String]) -> u8 {
    let mut condition = CONDITION_LOW;
    for pair in types.windows(2) {
        condition = condition.max(step_loss(&pair[0], &pair[1]));
    }
    // Narrow-then-widen: `x as u8 as u64` only makes sense as a deliberate
    // truncation mask, so a later int type wider than the first written one
    // implies the first cast discarded bits from the unknown source.
    if let Some(first) = numeric_info(&types[0])
        && !first.float
        && types[1..].iter().any(|later| {
            numeric_info(later).is_some_and(|info| !info.float && info.bits > first.bits)
        })
    {
        condition = condition.max(CONDITION_HIGH);
    }
    condition
}

fn step_loss(source: &str, target: &str) -> u8 {
    let (Some(source), Some(target)) = (numeric_info(source), numeric_info(target)) else {
        return CONDITION_LOW;
    };
    if source.float && !target.float {
        return CONDITION_HIGH;
    }
    if target.bits < source.bits {
        return CONDITION_HIGH;
    }
    if !source.float && target.float && target.mantissa_bits() < source.bits {
        return CONDITION_MEDIUM;
    }
    if target.bits == source.bits && source.signed != target.signed {
        return CONDITION_MEDIUM;
    }
    CONDITION_LOW
}

#[derive(Debug, Clone, Copy)]
struct Numeric {
    bits: u32,
    signed: bool,
    float: bool,
}

impl Numeric {
    const fn mantissa_bits(self) -> u32 {
        match self.bits {
            32 => 24,
            64 => 53,
            _ => self.bits,
        }
    }
}

fn numeric_info(ty: &str) -> Option<Numeric> {
    let (bits, signed, float) = match ty {
        "u8" => (8, false, false),
        "u16" => (16, false, false),
        "u32" => (32, false, false),
        "u64" | "usize" => (64, false, false),
        "u128" => (128, false, false),
        "i8" => (8, true, false),
        "i16" => (16, true, false),
        "i32" => (32, true, false),
        "i64" | "isize" => (64, true, false),
        "i128" => (128, true, false),
        "f32" => (32, true, true),
        "f64" => (64, true, true),
        "char" => (32, false, false),
        "bool" => (1, false, false),
        _ => return None,
    };
    Some(Numeric {
        bits,
        signed,
        float,
    })
}

#[cfg(test)]
mod tests {
    use super::super::parse_fixture;
    use crate::issue::IssueType;

    #[test]
    fn chained_narrowing_cast_is_reported_once() {
        let findings = parse_fixture(
            r#"
            fn shrink(value: u64) -> u64 {
                value as u8 as u64
            }
            "#,
        );
        let chained: Vec<_> = findings
            .iter()
            .filter(|finding| finding.issue_type == IssueType::ChainedCast)
            .collect();
        assert_eq!(chained.len(), 1);
        assert_eq!(chained[0].condition, super::CONDITION_HIGH);
    }

    #[test]
    fn single_cast_is_not_reported() {
        let findings = parse_fixture(
            r#"
            fn narrow(value: u64) -> u8 {
                value as u8
            }
            "#,
        );
        assert!(
            !findings
                .iter()
                .any(|finding| finding.issue_type == IssueType::ChainedCast)
        );
    }
}
