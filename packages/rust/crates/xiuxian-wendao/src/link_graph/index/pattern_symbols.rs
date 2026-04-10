use std::sync::OnceLock;

fn capture_symbol(pattern: &str, regex: Option<&regex::Regex>) -> Option<String> {
    regex.and_then(|compiled| {
        compiled
            .captures(pattern)
            .and_then(|caps| caps.get(1))
            .map(|capture| capture.as_str().to_string())
    })
}

fn push_captured_symbol(symbols: &mut Vec<String>, pattern: &str, regex: Option<&regex::Regex>) {
    if let Some(symbol) = capture_symbol(pattern, regex) {
        symbols.push(symbol);
    }
}

fn push_unique_captured_symbol(
    symbols: &mut Vec<String>,
    pattern: &str,
    regex: Option<&regex::Regex>,
) {
    if let Some(symbol) = capture_symbol(pattern, regex)
        && !symbols.contains(&symbol)
    {
        symbols.push(symbol);
    }
}

pub(super) fn extract_pattern_symbols(pattern: &str) -> Vec<String> {
    static RE_FN: OnceLock<Option<regex::Regex>> = OnceLock::new();
    static RE_STRUCT: OnceLock<Option<regex::Regex>> = OnceLock::new();
    static RE_CLASS: OnceLock<Option<regex::Regex>> = OnceLock::new();
    static RE_ENUM: OnceLock<Option<regex::Regex>> = OnceLock::new();
    static RE_METHOD: OnceLock<Option<regex::Regex>> = OnceLock::new();
    static RE_TRAIT: OnceLock<Option<regex::Regex>> = OnceLock::new();
    static RE_IMPL: OnceLock<Option<regex::Regex>> = OnceLock::new();

    let re_fn = RE_FN
        .get_or_init(|| regex::Regex::new(r"\bfn\s+([a-z_][a-z0-9_]*)").ok())
        .as_ref();
    let re_struct = RE_STRUCT
        .get_or_init(|| regex::Regex::new(r"\bstruct\s+([A-Z][a-zA-Z0-9_]*)").ok())
        .as_ref();
    let re_class = RE_CLASS
        .get_or_init(|| regex::Regex::new(r"\bclass\s+([A-Z][a-zA-Z0-9_]*)").ok())
        .as_ref();
    let re_enum = RE_ENUM
        .get_or_init(|| regex::Regex::new(r"\benum\s+([A-Z][a-zA-Z0-9_]*)").ok())
        .as_ref();
    let re_method = RE_METHOD
        .get_or_init(|| regex::Regex::new(r"\b(?:async\s+)?fn\s+([a-z_][a-z0-9_]*)\s*\(").ok())
        .as_ref();
    let re_trait = RE_TRAIT
        .get_or_init(|| regex::Regex::new(r"\btrait\s+([A-Z][a-zA-Z0-9_]*)").ok())
        .as_ref();
    let re_impl = RE_IMPL
        .get_or_init(|| {
            regex::Regex::new(r"\bimpl\s+(?:[A-Z][a-zA-Z0-9_]*\s+for\s+)?([A-Z][a-zA-Z0-9_]*)").ok()
        })
        .as_ref();

    let mut symbols = Vec::new();
    push_captured_symbol(&mut symbols, pattern, re_fn);
    push_captured_symbol(&mut symbols, pattern, re_struct);
    push_captured_symbol(&mut symbols, pattern, re_class);
    push_captured_symbol(&mut symbols, pattern, re_enum);
    push_unique_captured_symbol(&mut symbols, pattern, re_method);
    push_captured_symbol(&mut symbols, pattern, re_trait);
    push_captured_symbol(&mut symbols, pattern, re_impl);
    symbols
}
