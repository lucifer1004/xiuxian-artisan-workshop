//! Integration tests for command-line logging argument extraction.

use xiuxian_logging::{LogColor, LogFormat, LogLevel, LogSettings, split_logging_args};

#[test]
fn split_logging_args_extracts_verbose_and_format() {
    let raw = vec![
        "qianji".to_string(),
        "-vv".to_string(),
        "--log-format=json".to_string(),
        "graph".to_string(),
        "manifest.toml".to_string(),
        "output.bpmn".to_string(),
    ];

    let (settings, remaining) = split_logging_args(&raw);

    assert_eq!(settings.verbose, 2);
    assert_eq!(settings.format, LogFormat::Json);
    assert_eq!(
        remaining,
        vec!["qianji", "graph", "manifest.toml", "output.bpmn"]
    );
}

#[test]
fn split_logging_args_extracts_long_form_values() {
    let raw = vec![
        "qianji".to_string(),
        "--log-color".to_string(),
        "never".to_string(),
        "--log-level".to_string(),
        "warn".to_string(),
        "--log-filter".to_string(),
        "xiuxian_qianji=trace".to_string(),
        "repo".to_string(),
        "manifest.toml".to_string(),
        "{}".to_string(),
    ];

    let (settings, remaining) = split_logging_args(&raw);

    assert_eq!(settings.color, LogColor::Never);
    assert_eq!(settings.level, Some(LogLevel::Warn));
    assert_eq!(settings.filter, Some("xiuxian_qianji=trace".to_string()));
    assert_eq!(remaining, vec!["qianji", "repo", "manifest.toml", "{}"]);
}

#[test]
fn split_logging_args_ignores_unrelated_flags() {
    let raw = vec![
        "qianji".to_string(),
        "--unknown".to_string(),
        "-x".to_string(),
        "repo".to_string(),
        "manifest.toml".to_string(),
        "{}".to_string(),
    ];

    let (settings, remaining) = split_logging_args(&raw);

    assert_eq!(settings.verbose, 0);
    assert_eq!(settings.format, LogFormat::Pretty);
    assert_eq!(remaining, raw);
}

#[test]
fn log_settings_default_matches_cli_defaults() {
    let settings = LogSettings::default();
    assert_eq!(settings.verbose, 0);
    assert_eq!(settings.level, None);
    assert_eq!(settings.format, LogFormat::Pretty);
    assert_eq!(settings.color, LogColor::Auto);
    assert_eq!(settings.filter, None);
}
