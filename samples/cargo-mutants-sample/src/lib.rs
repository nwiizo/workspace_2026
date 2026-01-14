//! cargo-mutants のミューテーションテスト機能を実証するサンプルコード
//!
//! このライブラリは、cargo-mutants がサポートする様々なミューテーション種類を
//! 実際に検証するためのサンプル実装を提供します。

// ============================================================================
// 関数本体の置換（戻り値型別）
// ============================================================================

/// 符号付き整数を返す関数 (i32 → 0, 1, -1 に変異)
pub fn calculate_score(correct: u32, total: u32) -> i32 {
    if total == 0 {
        return 0;
    }
    let percentage = (correct * 100) / total;
    if percentage >= 80 {
        1 // 合格
    } else if percentage >= 50 {
        0 // 普通
    } else {
        -1 // 不合格
    }
}

/// 符号なし整数を返す関数 (u32 → 0, 1 に変異)
pub fn count_valid_items(items: &[Option<i32>]) -> u32 {
    items.iter().filter(|item| item.is_some()).count() as u32
}

/// bool を返す関数 (bool → true, false に変異)
pub fn is_valid_email(email: &str) -> bool {
    email.contains('@') && email.contains('.')
}

/// String を返す関数 (String → String::new(), "xyzzy".into() に変異)
pub fn format_greeting(name: &str) -> String {
    format!("Hello, {}!", name)
}

/// Option<T> を返す関数 (Option → Some(...), None に変異)
pub fn find_first_even(numbers: &[i32]) -> Option<i32> {
    numbers.iter().find(|&&n| n % 2 == 0).copied()
}

/// Result<T, E> を返す関数 (Result → Ok(...) に変異)
pub fn parse_positive_number(s: &str) -> Result<u32, String> {
    let n: i32 = s.parse().map_err(|_| "invalid number".to_string())?;
    if n > 0 {
        Ok(n as u32)
    } else {
        Err("number must be positive".to_string())
    }
}

/// Vec<T> を返す関数 (Vec → vec![], vec![element] に変異)
pub fn get_even_numbers(numbers: &[i32]) -> Vec<i32> {
    numbers.iter().filter(|&&n| n % 2 == 0).copied().collect()
}

/// Unit型を返す関数 (副作用のみ)
pub fn log_message(messages: &mut Vec<String>, msg: &str) {
    messages.push(msg.to_string());
}

// ============================================================================
// 二項演算子の置換
// ============================================================================

/// 比較演算子を使う関数 (==, !=, <, >, <=, >= の相互変異)
pub fn compare_values(a: i32, b: i32) -> &'static str {
    if a == b {
        "equal"
    } else if a < b {
        "less"
    } else {
        "greater"
    }
}

/// 論理演算子を使う関数 (&& ↔ || の変異)
pub fn check_range(value: i32, min: i32, max: i32) -> bool {
    value >= min && value <= max
}

/// 算術演算子を使う関数 (+ ↔ - ↔ * の変異)
pub fn calculate_area(width: u32, height: u32) -> u32 {
    width * height
}

/// 複合代入演算子を使う関数 (+= ↔ -= ↔ *= の変異)
pub fn accumulate(values: &[i32]) -> i32 {
    let mut sum = 0;
    for &v in values {
        sum += v;
    }
    sum
}

/// ビット演算子を使う関数 (<< ↔ >> の変異)
pub fn shift_bits(value: u32, shift: u32) -> u32 {
    value << shift
}

// ============================================================================
// 単項演算子の削除
// ============================================================================

/// 単項マイナスを使う関数 (-a → a に変異)
pub fn negate(value: i32) -> i32 {
    -value
}

/// 論理否定を使う関数 (!a → a に変異)
pub fn invert(flag: bool) -> bool {
    !flag
}

// ============================================================================
// match 式のミューテーション
// ============================================================================

/// match アームの削除（ワイルドカードパターン存在時）
pub fn classify_number(n: i32) -> &'static str {
    match n {
        0 => "zero",
        1..=10 => "small",
        11..=100 => "medium",
        _ => "large",
    }
}

/// match ガードの置換 (ガード式 → true/false に変異)
#[allow(clippy::redundant_guards)] // デモのために意図的にガードを使用
pub fn describe_value(n: i32) -> &'static str {
    match n {
        x if x < 0 => "negative",
        x if x == 0 => "zero",
        x if x > 0 => "positive",
        _ => unreachable!(),
    }
}

// ============================================================================
// 意図的にテストが甘い関数（missed を発生させるため）
// ============================================================================

/// この関数のテストは甘い（境界値をテストしていない）
pub fn is_adult(age: u32) -> bool {
    age >= 18
}

/// この関数のテストは甘い（一部のケースしかテストしていない）
pub fn calculate_discount(price: u32, member_level: u32) -> u32 {
    match member_level {
        0 => price,                // 割引なし
        1 => price - (price / 10), // 10% 割引
        2 => price - (price / 5),  // 20% 割引
        _ => price - (price / 4),  // 25% 割引
    }
}

// ============================================================================
// #[mutants::skip] の使用例
// ============================================================================

/// この関数はミューテーション対象から除外される
#[mutants::skip]
pub fn should_not_mutate() -> bool {
    true
}

// ============================================================================
// テストモジュール
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- 関数本体の置換 ---

    #[test]
    fn test_calculate_score() {
        assert_eq!(calculate_score(8, 10), 1); // 80%
        assert_eq!(calculate_score(5, 10), 0); // 50%
        assert_eq!(calculate_score(3, 10), -1); // 30%
        assert_eq!(calculate_score(0, 0), 0); // ゼロ除算回避
    }

    #[test]
    fn test_count_valid_items() {
        assert_eq!(count_valid_items(&[Some(1), None, Some(3)]), 2);
        assert_eq!(count_valid_items(&[None, None]), 0);
        assert_eq!(count_valid_items(&[]), 0);
    }

    #[test]
    fn test_is_valid_email() {
        assert!(is_valid_email("test@example.com"));
        assert!(!is_valid_email("invalid"));
        assert!(!is_valid_email("no-at.com"));
        assert!(!is_valid_email("no-dot@com"));
    }

    #[test]
    fn test_format_greeting() {
        assert_eq!(format_greeting("World"), "Hello, World!");
        assert_eq!(format_greeting("Rust"), "Hello, Rust!");
    }

    #[test]
    fn test_find_first_even() {
        assert_eq!(find_first_even(&[1, 3, 4, 5]), Some(4));
        assert_eq!(find_first_even(&[1, 3, 5]), None);
        assert_eq!(find_first_even(&[]), None);
    }

    #[test]
    fn test_parse_positive_number() {
        assert_eq!(parse_positive_number("42"), Ok(42));
        assert!(parse_positive_number("-1").is_err());
        assert!(parse_positive_number("0").is_err());
        assert!(parse_positive_number("abc").is_err());
    }

    #[test]
    fn test_get_even_numbers() {
        assert_eq!(get_even_numbers(&[1, 2, 3, 4, 5, 6]), vec![2, 4, 6]);
        assert_eq!(get_even_numbers(&[1, 3, 5]), Vec::<i32>::new());
    }

    #[test]
    fn test_log_message() {
        let mut messages = Vec::new();
        log_message(&mut messages, "test");
        assert_eq!(messages, vec!["test"]);
    }

    // --- 二項演算子 ---

    #[test]
    fn test_compare_values() {
        assert_eq!(compare_values(5, 5), "equal");
        assert_eq!(compare_values(3, 5), "less");
        assert_eq!(compare_values(7, 5), "greater");
    }

    #[test]
    fn test_check_range() {
        assert!(check_range(5, 0, 10));
        assert!(check_range(0, 0, 10));
        assert!(check_range(10, 0, 10));
        assert!(!check_range(-1, 0, 10));
        assert!(!check_range(11, 0, 10));
    }

    #[test]
    fn test_calculate_area() {
        assert_eq!(calculate_area(3, 4), 12);
        assert_eq!(calculate_area(0, 5), 0);
        assert_eq!(calculate_area(5, 0), 0);
    }

    #[test]
    fn test_accumulate() {
        assert_eq!(accumulate(&[1, 2, 3, 4]), 10);
        assert_eq!(accumulate(&[-1, 1]), 0);
        assert_eq!(accumulate(&[]), 0);
    }

    #[test]
    fn test_shift_bits() {
        assert_eq!(shift_bits(1, 3), 8);
        assert_eq!(shift_bits(4, 1), 8);
    }

    // --- 単項演算子 ---

    #[test]
    fn test_negate() {
        assert_eq!(negate(5), -5);
        assert_eq!(negate(-3), 3);
        assert_eq!(negate(0), 0);
    }

    #[test]
    fn test_invert() {
        assert!(!invert(true));
        assert!(invert(false));
    }

    // --- match 式 ---

    #[test]
    fn test_classify_number() {
        assert_eq!(classify_number(0), "zero");
        assert_eq!(classify_number(5), "small");
        assert_eq!(classify_number(50), "medium");
        assert_eq!(classify_number(200), "large");
    }

    #[test]
    fn test_describe_value() {
        assert_eq!(describe_value(-5), "negative");
        assert_eq!(describe_value(0), "zero");
        assert_eq!(describe_value(5), "positive");
    }

    // --- 意図的に甘いテスト（missed を発生させる） ---

    #[test]
    fn test_is_adult_weak() {
        // 境界値 18 をテストしていない → >= を > に変異しても検出できない可能性
        assert!(is_adult(20));
        assert!(!is_adult(10));
    }

    #[test]
    fn test_calculate_discount_weak() {
        // member_level 0 のみテスト → 他のケースの変異を検出できない
        assert_eq!(calculate_discount(100, 0), 100);
    }
}
