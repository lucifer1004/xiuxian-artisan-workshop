use super::RerankScoreWeights;
use std::env;

/// Effective runtime host settings after precedence resolution.
#[derive(Clone, Debug, PartialEq)]
pub struct EffectiveRerankFlightHostSettings {
    /// Schema version expected by the host.
    pub expected_schema_version: String,
    /// Embedding dimension expected by the host rerank route.
    pub rerank_dimension: usize,
    /// Score weights used by the host scorer.
    pub rerank_weights: RerankScoreWeights,
}

/// Parsed host-setting overrides and remaining positional args.
#[derive(Clone, Debug, PartialEq)]
pub struct ParsedRerankFlightHostOverrides {
    /// Parsed schema-version override, if provided.
    pub schema_version_override: Option<String>,
    /// Parsed rerank-dimension override, if provided.
    pub rerank_dimension_override: Option<usize>,
    /// Remaining positional args after removing known flags.
    pub positional_args: Vec<String>,
}

/// Split optional explicit host-setting overrides from binary args.
///
/// Returns parsed overrides plus the remaining positional args.
///
/// # Errors
///
/// Returns an error when a known host-setting flag is present without a value,
/// with a blank value, or with an invalid rerank dimension.
pub fn split_rerank_flight_host_overrides<I>(
    args: I,
) -> Result<ParsedRerankFlightHostOverrides, String>
where
    I: IntoIterator<Item = String>,
{
    let mut schema_version_override = None;
    let mut rerank_dimension_override = None;
    let mut positional_args = Vec::new();
    for argument in args {
        if let Some(flag_value) = argument.strip_prefix("--schema-version=") {
            if flag_value.trim().is_empty() {
                return Err("--schema-version must not be blank".to_string());
            }
            schema_version_override = Some(flag_value.to_string());
            continue;
        }
        if argument == "--schema-version" {
            return Err(
                "--schema-version requires a value; use --schema-version=<value>".to_string(),
            );
        }
        if let Some(flag_value) = argument.strip_prefix("--rerank-dimension=") {
            if flag_value.trim().is_empty() {
                return Err("--rerank-dimension must not be blank".to_string());
            }
            let dimension = flag_value
                .parse::<usize>()
                .map_err(|error| format!("invalid --rerank-dimension: {error}"))?;
            if dimension == 0 {
                return Err("--rerank-dimension must be greater than zero".to_string());
            }
            rerank_dimension_override = Some(dimension);
            continue;
        }
        if argument == "--rerank-dimension" {
            return Err(
                "--rerank-dimension requires a value; use --rerank-dimension=<value>".to_string(),
            );
        }
        positional_args.push(argument);
    }
    Ok(ParsedRerankFlightHostOverrides {
        schema_version_override,
        rerank_dimension_override,
        positional_args,
    })
}

/// Resolve the effective runtime host settings after applying precedence.
///
/// Precedence:
/// 1. explicit schema-version override
/// 2. file-backed schema version
/// 3. default `"v2"`
///
/// For score weights:
/// 1. file-backed weights
/// 2. env/default fallback weights supplied by the caller
#[must_use]
pub fn resolve_effective_rerank_flight_host_settings(
    schema_version_override: Option<String>,
    rerank_dimension_override: Option<usize>,
    file_backed_schema_version: Option<String>,
    file_backed_weights: Option<RerankScoreWeights>,
    fallback_dimension: usize,
    fallback_weights: RerankScoreWeights,
) -> EffectiveRerankFlightHostSettings {
    EffectiveRerankFlightHostSettings {
        expected_schema_version: schema_version_override
            .or(file_backed_schema_version)
            .unwrap_or_else(|| "v2".to_string()),
        rerank_dimension: rerank_dimension_override.unwrap_or(fallback_dimension),
        rerank_weights: file_backed_weights.unwrap_or(fallback_weights),
    }
}

/// Parse rerank score weights from environment variables.
///
/// Uses:
/// - `WENDAO_RERANK_VECTOR_WEIGHT`
/// - `WENDAO_RERANK_SEMANTIC_WEIGHT`
///
/// Missing values fall back to the shared `RerankScoreWeights::default()`.
///
/// # Errors
///
/// Returns an error when either env value cannot be parsed as `f64` or the
/// resolved weights are invalid.
pub fn rerank_score_weights_from_env() -> Result<RerankScoreWeights, String> {
    let vector_weight = env::var("WENDAO_RERANK_VECTOR_WEIGHT")
        .ok()
        .map(|value| {
            value
                .parse::<f64>()
                .map_err(|error| format!("invalid WENDAO_RERANK_VECTOR_WEIGHT: {error}"))
        })
        .transpose()?
        .unwrap_or(RerankScoreWeights::default().vector_weight);
    let semantic_weight = env::var("WENDAO_RERANK_SEMANTIC_WEIGHT")
        .ok()
        .map(|value| {
            value
                .parse::<f64>()
                .map_err(|error| format!("invalid WENDAO_RERANK_SEMANTIC_WEIGHT: {error}"))
        })
        .transpose()?
        .unwrap_or(RerankScoreWeights::default().semantic_weight);
    RerankScoreWeights::new(vector_weight, semantic_weight)
}

#[cfg(test)]
mod tests {
    use super::{
        EffectiveRerankFlightHostSettings, ParsedRerankFlightHostOverrides,
        rerank_score_weights_from_env, resolve_effective_rerank_flight_host_settings,
        split_rerank_flight_host_overrides,
    };
    use crate::transport::RerankScoreWeights;

    #[test]
    fn split_rerank_flight_host_overrides_extracts_flags() {
        let overrides: ParsedRerankFlightHostOverrides = split_rerank_flight_host_overrides(vec![
            "--schema-version=v8".to_string(),
            "--rerank-dimension=4".to_string(),
            "alpha/repo".to_string(),
            "3".to_string(),
        ])
        .expect("host-setting flags should parse");

        assert_eq!(overrides.schema_version_override.as_deref(), Some("v8"));
        assert_eq!(overrides.rerank_dimension_override, Some(4));
        assert_eq!(
            overrides.positional_args,
            vec!["alpha/repo".to_string(), "3".to_string()]
        );
    }

    #[test]
    fn split_rerank_flight_host_overrides_rejects_blank_schema_version() {
        let error = split_rerank_flight_host_overrides(vec!["--schema-version=".to_string()])
            .expect_err("blank schema-version should fail");

        assert_eq!(error, "--schema-version must not be blank");
    }

    #[test]
    fn split_rerank_flight_host_overrides_rejects_zero_dimension() {
        let error = split_rerank_flight_host_overrides(vec!["--rerank-dimension=0".to_string()])
            .expect_err("zero rerank dimension should fail");

        assert_eq!(error, "--rerank-dimension must be greater than zero");
    }

    #[test]
    fn resolve_effective_rerank_flight_host_settings_applies_precedence() {
        let fallback_weights =
            RerankScoreWeights::new(0.4, 0.6).expect("fallback weights should validate");
        let file_backed_weights =
            RerankScoreWeights::new(0.9, 0.1).expect("file-backed weights should validate");

        let settings: EffectiveRerankFlightHostSettings =
            resolve_effective_rerank_flight_host_settings(
                Some("v8".to_string()),
                Some(4),
                Some("v9".to_string()),
                Some(file_backed_weights.clone()),
                3,
                fallback_weights,
            );

        assert_eq!(settings.expected_schema_version, "v8");
        assert_eq!(settings.rerank_dimension, 4);
        assert_eq!(settings.rerank_weights, file_backed_weights);
    }

    #[test]
    fn rerank_score_weights_from_env_defaults_when_unset() {
        let weights = rerank_score_weights_from_env().expect("default env weights should resolve");

        assert_eq!(weights, RerankScoreWeights::default());
    }
}
