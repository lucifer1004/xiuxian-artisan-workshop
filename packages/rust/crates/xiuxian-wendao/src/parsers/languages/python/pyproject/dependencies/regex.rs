use std::sync::LazyLock;

use regex::Regex;

/// Regex for parsing dependency format: name[extras]==version.
pub(super) static RE_DEP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([a-zA-Z0-9_-]+)(?:\[[^\]]+\])?(?:==|~=|>=|<=|<|>|=)([^,\]\s]+)")
        .unwrap_or_else(|err| panic!("invalid RE_DEP regex: {err}"))
});

/// Regex for parsing exact dependency format: package==version.
pub(super) static RE_EXACT_DEP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([a-zA-Z0-9_-]+)(?:\[[^\]]+\])?==([0-9][^\s,\]]*)")
        .unwrap_or_else(|err| panic!("invalid RE_EXACT_DEP regex: {err}"))
});

/// Regex for simple package name extraction.
pub(super) static RE_SIMPLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([a-zA-Z0-9_-]+)").unwrap_or_else(|err| panic!("invalid RE_SIMPLE regex: {err}"))
});
