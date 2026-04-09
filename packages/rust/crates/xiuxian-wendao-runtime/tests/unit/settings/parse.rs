use super::{
    first_non_empty, parse_bool, parse_positive_f64, parse_positive_u64, parse_positive_usize,
};

#[test]
fn parse_helpers_accept_expected_values() {
    assert_eq!(parse_positive_u64("10"), Some(10));
    assert_eq!(parse_positive_usize("12"), Some(12));
    assert_eq!(parse_positive_f64("0.5"), Some(0.5));
    assert_eq!(parse_bool("yes"), Some(true));
    assert_eq!(
        first_non_empty(&[Some("   ".to_string()), Some(" v1 ".to_string())]),
        Some("v1".to_string())
    );
}
