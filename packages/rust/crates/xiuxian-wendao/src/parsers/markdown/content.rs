use serde_yaml::Value;
use xiuxian_wendao_parsers::frontmatter::split_frontmatter;

fn value_to_non_negative_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64().filter(|v| v.is_finite() && *v >= 0.0),
        Value::String(raw) => raw
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite() && *v >= 0.0),
        _ => None,
    }
}

pub(in crate::parsers::markdown) fn parse_frontmatter(content: &str) -> (Option<Value>, &str) {
    split_frontmatter(content)
}

pub(super) fn extract_saliency_params(frontmatter: Option<&Value>) -> (f64, f64) {
    let default_base = crate::link_graph::saliency::DEFAULT_SALIENCY_BASE;
    let default_decay = crate::link_graph::saliency::DEFAULT_DECAY_RATE;
    let Some(frontmatter) = frontmatter else {
        return (default_base, default_decay);
    };

    let saliency_base = frontmatter
        .get("saliency_base")
        .and_then(value_to_non_negative_f64)
        .unwrap_or(default_base);
    let decay_rate = frontmatter
        .get("decay_rate")
        .and_then(value_to_non_negative_f64)
        .unwrap_or(default_decay);

    (saliency_base, decay_rate)
}
