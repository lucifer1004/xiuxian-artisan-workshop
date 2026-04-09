use std::str::FromStr;

/// Return a trimmed non-empty string candidate.
#[must_use]
pub fn trimmed_non_empty(candidate: Option<String>) -> Option<String> {
    candidate
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Parse a trimmed scalar value.
#[must_use]
pub fn parse_trimmed<T>(raw: &str) -> Option<T>
where
    T: FromStr,
{
    raw.trim().parse::<T>().ok()
}

/// Parse a positive numeric-like value from trimmed input.
#[must_use]
pub fn parse_positive<T>(raw: &str) -> Option<T>
where
    T: FromStr + Default + PartialOrd,
{
    let value = parse_trimmed::<T>(raw)?;
    (value > T::default()).then_some(value)
}

/// Parse a bool-style flag from trimmed input.
#[must_use]
pub fn parse_bool_flag(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

/// Return the first non-empty environment-style value from a precedence list.
#[must_use]
pub fn first_non_empty_lookup(
    names: &[&str],
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    names
        .iter()
        .find_map(|name| trimmed_non_empty(lookup(name)))
}

/// Return the first non-empty environment-style value and its lookup key.
#[must_use]
pub fn first_non_empty_named_lookup(
    names: &[&str],
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Option<(String, String)> {
    names
        .iter()
        .find_map(|name| trimmed_non_empty(lookup(name)).map(|value| ((*name).to_string(), value)))
}

/// Resolve a TOML-owned string first and otherwise return the first named
/// env-style lookup candidate.
#[must_use]
pub fn toml_first_named_string(
    setting_name: &str,
    setting_value: Option<String>,
    lookup: &dyn Fn(&str) -> Option<String>,
    env_names: &[&str],
) -> Option<(String, String)> {
    trimmed_non_empty(setting_value)
        .map(|value| (setting_name.to_string(), value))
        .or_else(|| first_non_empty_named_lookup(env_names, lookup))
}

/// Resolve a parsed env-style value from a single lookup key.
#[must_use]
pub fn lookup_parsed<T, F>(
    name: &str,
    lookup: &dyn Fn(&str) -> Option<String>,
    parse: F,
) -> Option<T>
where
    F: Fn(&str) -> Option<T>,
{
    trimmed_non_empty(lookup(name)).and_then(|value| parse(value.as_str()))
}

/// Resolve a positive parsed env-style value from a single lookup key.
#[must_use]
pub fn lookup_positive_parsed<T>(name: &str, lookup: &dyn Fn(&str) -> Option<String>) -> Option<T>
where
    T: FromStr + Default + PartialOrd,
{
    lookup_parsed(name, lookup, parse_positive::<T>)
}

/// Resolve a bool-style env flag from a single lookup key.
#[must_use]
pub fn lookup_bool_flag(name: &str, lookup: &dyn Fn(&str) -> Option<String>) -> Option<bool> {
    lookup_parsed(name, lookup, parse_bool_flag)
}

/// Resolve a TOML-owned string first and fall back to env-style lookups.
///
/// Invalid or blank TOML values are treated as absent for this permissive
/// helper, which keeps optional runtime surfaces recoverable through the
/// fallback env chain.
#[must_use]
pub fn toml_first_env_string(
    setting_value: Option<String>,
    lookup: &dyn Fn(&str) -> Option<String>,
    env_names: &[&str],
) -> Option<String> {
    trimmed_non_empty(setting_value).or_else(|| first_non_empty_lookup(env_names, lookup))
}

/// Resolve a TOML-owned value first and fall back to env-style lookups.
///
/// Invalid or blank TOML values are treated as absent for this permissive
/// helper, which keeps optional runtime surfaces recoverable through the
/// fallback env chain.
#[must_use]
pub fn toml_first_env_parsed<T, F>(
    setting_value: Option<String>,
    lookup: &dyn Fn(&str) -> Option<String>,
    env_names: &[&str],
    parse: F,
) -> Option<T>
where
    F: Fn(&str) -> Option<T>,
{
    trimmed_non_empty(setting_value)
        .and_then(|value| parse(value.as_str()))
        .or_else(|| {
            first_non_empty_lookup(env_names, lookup).and_then(|value| parse(value.as_str()))
        })
}

#[cfg(test)]
mod tests {
    use super::{
        first_non_empty_lookup, first_non_empty_named_lookup, lookup_bool_flag,
        lookup_positive_parsed, parse_bool_flag, parse_positive, parse_trimmed,
        toml_first_env_parsed, toml_first_env_string, toml_first_named_string, trimmed_non_empty,
    };
    use std::collections::BTreeMap;

    #[test]
    fn trimmed_non_empty_rejects_blank_values() {
        assert_eq!(trimmed_non_empty(Some("   ".to_string())), None);
    }

    #[test]
    fn trimmed_non_empty_trims_non_blank_values() {
        assert_eq!(
            trimmed_non_empty(Some(" redis://127.0.0.1/0 ".to_string())),
            Some("redis://127.0.0.1/0".to_string())
        );
    }

    #[test]
    fn first_non_empty_lookup_returns_first_trimmed_candidate() {
        let value = first_non_empty_lookup(&["A", "B", "C"], &|name| match name {
            "A" => Some("   ".to_string()),
            "B" => Some(" redis://127.0.0.1/1 ".to_string()),
            _ => Some("redis://127.0.0.1/2".to_string()),
        });

        assert_eq!(value, Some("redis://127.0.0.1/1".to_string()));
    }

    #[test]
    fn first_non_empty_named_lookup_returns_source_name_and_trimmed_candidate() {
        let value = first_non_empty_named_lookup(&["A", "B", "C"], &|name| match name {
            "A" => Some("   ".to_string()),
            "B" => Some(" redis://127.0.0.1/1 ".to_string()),
            _ => Some("redis://127.0.0.1/2".to_string()),
        });

        assert_eq!(
            value,
            Some(("B".to_string(), "redis://127.0.0.1/1".to_string()))
        );
    }

    #[test]
    fn toml_first_named_string_prefers_toml_and_preserves_setting_name() {
        let value = toml_first_named_string(
            "search.cache.valkey_url",
            Some(" redis://127.0.0.1/3 ".to_string()),
            &|_| Some("redis://127.0.0.1/9".to_string()),
            &["VALKEY_URL"],
        );

        assert_eq!(
            value,
            Some((
                "search.cache.valkey_url".to_string(),
                "redis://127.0.0.1/3".to_string()
            ))
        );
    }

    #[test]
    fn parse_trimmed_parses_scalar_values() {
        assert_eq!(parse_trimmed::<u64>(" 42 "), Some(42));
    }

    #[test]
    fn parse_positive_rejects_zero_and_negative_like_values() {
        assert_eq!(parse_positive::<u64>("0"), None);
        assert_eq!(parse_positive::<i64>("-1"), None);
    }

    #[test]
    fn parse_bool_flag_recognizes_common_aliases() {
        assert_eq!(parse_bool_flag(" yes "), Some(true));
        assert_eq!(parse_bool_flag("OFF"), Some(false));
    }

    #[test]
    fn lookup_positive_parsed_uses_trimmed_lookup_values() {
        let value = lookup_positive_parsed::<usize>("LIMIT", &|name| {
            (name == "LIMIT").then(|| " 7 ".to_string())
        });
        assert_eq!(value, Some(7));
    }

    #[test]
    fn lookup_bool_flag_rejects_invalid_values() {
        let value = lookup_bool_flag("ENABLED", &|name| {
            (name == "ENABLED").then(|| "sometimes".to_string())
        });
        assert_eq!(value, None);
    }

    #[test]
    fn toml_first_env_string_prefers_toml_over_env() {
        let value = toml_first_env_string(
            Some(" redis://127.0.0.1/3 ".to_string()),
            &|name| match name {
                "VALKEY_URL" => Some("redis://127.0.0.1/9".to_string()),
                _ => None,
            },
            &["VALKEY_URL"],
        );

        assert_eq!(value, Some("redis://127.0.0.1/3".to_string()));
    }

    #[test]
    fn toml_first_env_parsed_falls_back_when_toml_is_invalid() {
        let value = toml_first_env_parsed(
            Some("not-a-number".to_string()),
            &|name| match name {
                "TTL" => Some("42".to_string()),
                _ => None,
            },
            &["TTL"],
            |raw| raw.parse::<u64>().ok(),
        );

        assert_eq!(value, Some(42));
    }

    #[test]
    fn toml_first_env_macro_prefers_toml_values() {
        fn get_setting(
            settings: &BTreeMap<&'static str, String>,
            dotted_key: &str,
        ) -> Option<String> {
            settings.get(dotted_key).cloned()
        }

        let mut settings = BTreeMap::new();
        settings.insert(
            "search.cache.valkey_url",
            " redis://127.0.0.1/7 ".to_string(),
        );

        let value = crate::toml_first_env!(
            &settings,
            "search.cache.valkey_url",
            &|name| match name {
                "VALKEY_URL" => Some("redis://127.0.0.1/9".to_string()),
                _ => None,
            },
            ["VALKEY_URL"],
            get_setting
        );

        assert_eq!(value, Some("redis://127.0.0.1/7".to_string()));
    }

    #[test]
    fn first_some_macro_returns_first_present_candidate() {
        let value = crate::first_some!(None::<usize>, Some(7_usize), Some(9_usize));
        assert_eq!(value, Some(7));
    }

    #[test]
    fn first_some_macro_short_circuits_later_candidates() {
        let mut evaluated = false;
        let value = crate::first_some!(Some(3_u64), {
            evaluated = true;
            Some(5_u64)
        });
        assert_eq!(value, Some(3));
        assert!(!evaluated);
    }
}
