use kindle_capture::web::{reader_url, validate_asin};

#[test]
fn asin_is_normalized_without_allowing_url_or_script_injection() {
    assert_eq!(validate_asin("b0dskptjm5").unwrap(), "B0DSKPTJM5");
    assert!(validate_asin("B0DSKPTJM5&x=1").is_err());
    assert!(validate_asin("../../etc").is_err());
    assert_eq!(
        reader_url("b0dskptjm5").unwrap(),
        "https://read.amazon.co.jp/?asin=B0DSKPTJM5"
    );
}
