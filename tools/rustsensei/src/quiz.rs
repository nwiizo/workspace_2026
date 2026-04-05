use crate::analyzer;
use crate::model::{QuizQuestion, QuizResult};

/// Built-in quiz questions for error prediction mode.
pub fn builtin_questions() -> Vec<QuizQuestion> {
    vec![
        QuizQuestion {
            id: "q01".to_string(),
            code: r#"fn main() {
    let s = String::from("hello");
    let t = s;
    println!("{}", t);
}"#
            .to_string(),
            compiles: true,
            explanation: "`s` の所有権は `t` に移動しますが、移動後に `s` は使われていないのでコンパイルは成功します。".to_string(),
            related_concept: "move".to_string(),
        },
        QuizQuestion {
            id: "q02".to_string(),
            code: r#"fn main() {
    let s = String::from("hello");
    let t = s;
    println!("{}", s);
}"#
            .to_string(),
            compiles: false,
            explanation: "`let t = s;` で所有権が移動した後に `s` を使おうとしているため、コンパイルエラーになります。\n\
                          エラー: `borrow of moved value: s`".to_string(),
            related_concept: "use_after_move".to_string(),
        },
        QuizQuestion {
            id: "q03".to_string(),
            code: r#"fn main() {
    let x = 42;
    let y = x;
    println!("{} {}", x, y);
}"#
            .to_string(),
            compiles: true,
            explanation: "`i32` は `Copy` トレイトを実装しているため、`let y = x;` はコピーであり移動ではありません。\
                          `x` は引き続き使用できます。".to_string(),
            related_concept: "copy".to_string(),
        },
        QuizQuestion {
            id: "q04".to_string(),
            code: r#"fn main() {
    let s = String::from("hello");
    let r1 = &s;
    let r2 = &s;
    println!("{} {}", r1, r2);
}"#
            .to_string(),
            compiles: true,
            explanation: "共有参照（`&T`）は同時に複数作成できます。読み取りだけなので安全です。".to_string(),
            related_concept: "shared_borrow".to_string(),
        },
        QuizQuestion {
            id: "q05".to_string(),
            code: r#"fn main() {
    let mut s = String::from("hello");
    let r1 = &mut s;
    let r2 = &mut s;
    println!("{} {}", r1, r2);
}"#
            .to_string(),
            compiles: false,
            explanation: "同じ変数への `&mut` 参照は同時に1つしか存在できません。\
                          `r1` がまだ生きている間に `r2` を作ろうとするとエラーになります。\n\
                          エラー: `cannot borrow s as mutable more than once at a time`".to_string(),
            related_concept: "mut_borrow_conflict".to_string(),
        },
        QuizQuestion {
            id: "q06".to_string(),
            code: r#"fn main() {
    let mut s = String::from("hello");
    let r1 = &s;
    let r2 = &mut s;
    println!("{} {}", r1, r2);
}"#
            .to_string(),
            compiles: false,
            explanation: "共有参照（`&s`）と可変参照（`&mut s`）は同時に存在できません。\
                          `r1` が生きている間に `r2` で変更しようとするとエラーです。".to_string(),
            related_concept: "borrow_conflict".to_string(),
        },
        QuizQuestion {
            id: "q07".to_string(),
            code: r#"fn main() {
    let s = String::from("hello");
    let t = s.clone();
    println!("{} {}", s, t);
}"#
            .to_string(),
            compiles: true,
            explanation: "`.clone()` は値の深いコピーを作成します。`s` と `t` はそれぞれ独立した `String` を所有するため、\
                          両方とも有効です。".to_string(),
            related_concept: "clone".to_string(),
        },
        QuizQuestion {
            id: "q08".to_string(),
            code: r#"fn main() {
    let s = String::from("hello");
    takes_ownership(s);
    println!("{}", s);
}

fn takes_ownership(s: String) {
    println!("{}", s);
}"#
            .to_string(),
            compiles: false,
            explanation: "関数に値を渡すと所有権が移動します。`takes_ownership(s)` で `s` の所有権は関数内の引数に移動し、\
                          関数終了時にドロップされます。その後 `s` は使えません。".to_string(),
            related_concept: "function_move".to_string(),
        },
        QuizQuestion {
            id: "q09".to_string(),
            code: r#"fn main() {
    let s = String::from("hello");
    let len = calculate_length(&s);
    println!("'{}' has length {}", s, len);
}

fn calculate_length(s: &String) -> usize {
    s.len()
}"#
            .to_string(),
            compiles: true,
            explanation: "`&s` で共有参照を渡しているため、所有権は移動しません。\
                          関数終了後も `s` は有効です。これが借用の基本パターンです。".to_string(),
            related_concept: "borrow_function".to_string(),
        },
        QuizQuestion {
            id: "q10".to_string(),
            code: r#"fn main() {
    let mut v = vec![1, 2, 3];
    let first = &v[0];
    v.push(4);
    println!("{}", first);
}"#
            .to_string(),
            compiles: false,
            explanation: "`&v[0]` で `v` への共有参照を取得した後に `v.push(4)` で可変操作しようとしています。\
                          Vec の push はメモリの再割り当てが起こる可能性があり、参照が無効になるためエラーです。".to_string(),
            related_concept: "vec_invalidation".to_string(),
        },
    ]
}

/// Check a user's prediction against the actual compile result.
pub fn check_prediction(question_id: &str, user_prediction: bool) -> Option<QuizResult> {
    let questions = builtin_questions();
    let question = questions.iter().find(|q| q.id == question_id)?;

    let analysis = analyzer::analyze(&question.code);

    Some(QuizResult {
        correct: user_prediction == question.compiles,
        expected: question.compiles,
        explanation: question.explanation.clone(),
        analysis,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_questions_count() {
        assert_eq!(builtin_questions().len(), 10);
    }

    #[test]
    fn test_correct_prediction() {
        let result = check_prediction("q01", true);
        assert!(result.is_some());
        assert!(result.unwrap().correct);
    }

    #[test]
    fn test_incorrect_prediction() {
        let result = check_prediction("q02", true);
        assert!(result.is_some());
        assert!(!result.unwrap().correct);
    }

    #[test]
    fn test_unknown_question() {
        let result = check_prediction("nonexistent", true);
        assert!(result.is_none());
    }
}
