/// Parse a positive integer in `u64` form.
#[must_use]
pub fn parse_positive_u64(raw: &str) -> Option<u64> {
    raw.trim().parse::<u64>().ok().filter(|value| *value > 0)
}

/// Parse a positive integer in `usize` form.
#[must_use]
pub fn parse_positive_usize(raw: &str) -> Option<usize> {
    raw.trim().parse::<usize>().ok().filter(|value| *value > 0)
}

/// Parse a positive finite float.
#[must_use]
pub fn parse_positive_f64(raw: &str) -> Option<f64> {
    raw.trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value > 0.0)
}

/// Parse a conventional truthy/falsy string into a boolean.
#[must_use]
pub fn parse_bool(raw: &str) -> Option<bool> {
    match raw.trim().to_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

/// Return the first non-empty string after trimming whitespace.
#[must_use]
pub fn first_non_empty(values: &[Option<String>]) -> Option<String> {
    values.iter().flatten().find_map(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

#[cfg(test)]
mod tests {
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
}
