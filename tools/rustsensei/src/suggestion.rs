use crate::analyzer;
use crate::model::{FixStrategy, FixSuggestion, OwnershipEvent, SuggestionResult};

/// Analyze source code for ownership errors and suggest fixes.
pub fn suggest_fixes(source: &str) -> SuggestionResult {
    let analysis = analyzer::analyze(source);

    // Find ownership errors in steps
    let errors: Vec<_> = analysis
        .steps
        .iter()
        .filter(|s| matches!(&s.event, OwnershipEvent::CompileError { .. }))
        .collect();

    if errors.is_empty() {
        // Check for moves that might cause issues
        let moves: Vec<_> = analysis
            .steps
            .iter()
            .filter(|s| matches!(&s.event, OwnershipEvent::Move { .. }))
            .collect();

        if moves.is_empty() {
            return SuggestionResult {
                source: source.to_string(),
                error_pattern: None,
                suggestions: Vec::new(),
            };
        }

        // Suggest improvements even without errors
        let mut suggestions = Vec::new();
        for step in &moves {
            if let OwnershipEvent::Move { from, to } = &step.event {
                suggestions.push(FixSuggestion {
                    strategy: FixStrategy::Borrow,
                    title: format!("借用を検討: `{from}` → `{to}`"),
                    description: format!(
                        "`{to}` で `{from}` の所有権を奪う代わりに、`&{from}` で借用すれば `{from}` を後でも使えます"
                    ),
                    fixed_code: String::new(),
                    trade_off: "借用には参照のライフタイム制約が加わります".to_string(),
                });
            }
        }

        return SuggestionResult {
            source: source.to_string(),
            error_pattern: Some("move_detected".to_string()),
            suggestions,
        };
    }

    // Generate fix suggestions for each error pattern
    let mut suggestions = Vec::new();
    let mut error_pattern = None;

    for error_step in &errors {
        if let OwnershipEvent::CompileError { message } = &error_step.event {
            if message.contains("moved value") {
                error_pattern = Some("use_after_move".to_string());
                // Find which variable was moved
                let moved_var = extract_moved_var(message);
                suggestions.extend(suggest_use_after_move_fixes(source, &moved_var, &analysis));
            } else if message.contains("mutable") && message.contains("borrow") {
                error_pattern = Some("double_mut_borrow".to_string());
                suggestions.extend(suggest_double_borrow_fixes(source));
            }
        }
    }

    SuggestionResult {
        source: source.to_string(),
        error_pattern,
        suggestions,
    }
}

fn extract_moved_var(message: &str) -> String {
    // Extract variable name from "borrow of moved value: `x`"
    if let Some(start) = message.find('`')
        && let Some(end) = message[start + 1..].find('`')
    {
        return message[start + 1..start + 1 + end].to_string();
    }
    "unknown".to_string()
}

fn suggest_use_after_move_fixes(
    source: &str,
    moved_var: &str,
    analysis: &crate::model::AnalysisResult,
) -> Vec<FixSuggestion> {
    let mut fixes = Vec::new();

    // Find the move step to understand context
    let move_step = analysis.steps.iter().find(|s| {
        if let OwnershipEvent::Move { from, .. } = &s.event {
            from == moved_var
        } else {
            false
        }
    });

    let move_target = move_step.and_then(|s| {
        if let OwnershipEvent::Move { to, .. } = &s.event {
            Some(to.clone())
        } else {
            None
        }
    });

    let target = move_target.unwrap_or_else(|| "t".to_string());

    // Fix 1: Clone
    fixes.push(FixSuggestion {
        strategy: FixStrategy::Clone,
        title: "`.clone()` で複製する".to_string(),
        description: format!(
            "`let {target} = {moved_var}.clone();` とすると、`{moved_var}` の独立したコピーが作られ、\
             元の `{moved_var}` も引き続き使用できます。"
        ),
        fixed_code: source.replace(
            &format!("let {target} = {moved_var};"),
            &format!("let {target} = {moved_var}.clone();"),
        ),
        trade_off: "ヒープメモリの追加確保が発生します。パフォーマンスが重要な場面では借用を検討してください。"
            .to_string(),
    });

    // Fix 2: Borrow
    fixes.push(FixSuggestion {
        strategy: FixStrategy::Borrow,
        title: format!("`&{moved_var}` で借用する"),
        description: format!(
            "`let {target} = &{moved_var};` とすると、所有権を移動せず共有参照を作ります。\
             ただし `{target}` は `{moved_var}` のライフタイム内でのみ有効です。"
        ),
        fixed_code: source.replace(
            &format!("let {target} = {moved_var};"),
            &format!("let {target} = &{moved_var};"),
        ),
        trade_off: "参照はライフタイムに制約されます。参照を関数の外に返すことはできません。"
            .to_string(),
    });

    // Fix 3: Scope change
    fixes.push(FixSuggestion {
        strategy: FixStrategy::ScopeChange,
        title: "スコープを分離する".to_string(),
        description: format!(
            "`{target}` の使用を先に完了させてから `{moved_var}` を使うか、\
             ブロック `{{ }}` でスコープを分けることで、両方の変数を安全に使えます。"
        ),
        fixed_code: String::new(), // Context-dependent, hard to auto-generate
        trade_off: "コードの構造変更が必要です。場合によっては可読性が下がります。".to_string(),
    });

    fixes
}

fn suggest_double_borrow_fixes(source: &str) -> Vec<FixSuggestion> {
    vec![
        // Fix 1: Sequential borrows via scope
        FixSuggestion {
            strategy: FixStrategy::ScopeChange,
            title: "可変借用を順番に行う".to_string(),
            description:
                "最初の `&mut` 参照の使用を完了してから、2つ目の `&mut` 参照を作ります。\
                 NLL（Non-Lexical Lifetimes）により、参照の最後の使用箇所でライフタイムが終了します。"
                    .to_string(),
            fixed_code: String::new(),
            trade_off: "コードの実行順序に制約が生まれます。".to_string(),
        },
        // Fix 2: Clone to avoid borrow
        FixSuggestion {
            strategy: FixStrategy::Clone,
            title: "値を複製して独立操作する".to_string(),
            description:
                "変数を `.clone()` して独立したコピーを作り、それぞれ独立に変更します。\
                 変更後の統合が必要な場合は適していません。"
                    .to_string(),
            fixed_code: String::new(),
            trade_off: "メモリ使用量が増加します。変更の同期が必要な場合は使えません。".to_string(),
        },
        // Fix 3: Rc<RefCell<T>>
        FixSuggestion {
            strategy: FixStrategy::Rc,
            title: "`Rc<RefCell<T>>` で共有可変性".to_string(),
            description:
                "`Rc<RefCell<T>>` を使うと、実行時の借用チェックにより複数箇所から可変アクセスできます。\
                 ただし実行時パニックのリスクがあります。"
                    .to_string(),
            fixed_code: source
                .replace("let mut s =", "let s = Rc::new(RefCell::new(")
                .replace("&mut s", "s.borrow_mut()"),
            trade_off:
                "実行時オーバーヘッドとパニックリスクがあります。通常はスコープ分離を優先してください。"
                    .to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_error_no_suggestions() {
        let result = suggest_fixes(
            r#"fn main() {
    let x = 42;
    let y = x;
}"#,
        );
        assert!(result.suggestions.is_empty());
    }

    #[test]
    fn test_move_detected() {
        let result = suggest_fixes(
            r#"fn main() {
    let s = String::from("hello");
    let t = s;
}"#,
        );
        assert_eq!(result.error_pattern.as_deref(), Some("move_detected"));
        assert!(!result.suggestions.is_empty());
    }

    #[test]
    fn test_use_after_move_suggestions() {
        let result = suggest_fixes(
            r#"fn main() {
    let s = String::from("hello");
    let t = s;
    println!("{}", s);
}"#,
        );
        // The analyzer detects use-after-move as a CompileError step
        // so suggestions should include clone and borrow fixes
        assert!(!result.suggestions.is_empty());
    }
}
