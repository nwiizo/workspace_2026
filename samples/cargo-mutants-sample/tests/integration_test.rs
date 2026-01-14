//! 統合テスト

use cargo_mutants_sample::*;

#[test]
fn test_email_validation_integration() {
    // 有効なメールアドレス
    assert!(is_valid_email("user@domain.com"));
    assert!(is_valid_email("test.user@example.org"));

    // 無効なメールアドレス
    assert!(!is_valid_email("invalid"));
    assert!(!is_valid_email("nodot@example"));
}

#[test]
fn test_number_parsing_integration() {
    // 正常系
    assert_eq!(parse_positive_number("1"), Ok(1));
    assert_eq!(parse_positive_number("999"), Ok(999));

    // 異常系
    assert!(parse_positive_number("0").is_err());
    assert!(parse_positive_number("-5").is_err());
    assert!(parse_positive_number("not a number").is_err());
}

#[test]
fn test_scoring_integration() {
    // 合格ライン
    assert_eq!(calculate_score(80, 100), 1);
    assert_eq!(calculate_score(85, 100), 1);

    // 普通
    assert_eq!(calculate_score(50, 100), 0);
    assert_eq!(calculate_score(79, 100), 0);

    // 不合格
    assert_eq!(calculate_score(49, 100), -1);
    assert_eq!(calculate_score(0, 100), -1);
}
