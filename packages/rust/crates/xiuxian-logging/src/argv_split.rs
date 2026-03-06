use crate::types::{LogColor, LogFormat, LogLevel, LogSettings};

/// Split logging flags from raw argv while keeping remaining positional args.
///
/// This helper is for non-`clap` binaries that still want to share the same
/// logging surface.
#[must_use]
pub fn split_logging_args(raw_args: &[String]) -> (LogSettings, Vec<String>) {
    let mut settings = LogSettings::default();
    let mut remaining = Vec::with_capacity(raw_args.len());

    if let Some(program) = raw_args.first() {
        remaining.push(program.clone());
    }

    let mut index = 1;
    while index < raw_args.len() {
        let arg = &raw_args[index];

        if let Some(count) = parse_short_verbose(arg) {
            settings.verbose = settings.verbose.saturating_add(count);
            index += 1;
            continue;
        }

        if arg == "--log-verbose" {
            settings.verbose = settings.verbose.saturating_add(1);
            index += 1;
            continue;
        }

        if let Some(value) = arg.strip_prefix("--log-format=") {
            if let Ok(parsed) = value.parse::<LogFormat>() {
                settings.format = parsed;
            }
            index += 1;
            continue;
        }

        if arg == "--log-format" {
            if let Some(next) = raw_args.get(index + 1)
                && let Ok(parsed) = next.parse::<LogFormat>()
            {
                settings.format = parsed;
                index += 2;
                continue;
            }
            index += 1;
            continue;
        }

        if let Some(value) = arg.strip_prefix("--log-color=") {
            if let Ok(parsed) = value.parse::<LogColor>() {
                settings.color = parsed;
            }
            index += 1;
            continue;
        }

        if arg == "--log-color" {
            if let Some(next) = raw_args.get(index + 1)
                && let Ok(parsed) = next.parse::<LogColor>()
            {
                settings.color = parsed;
                index += 2;
                continue;
            }
            index += 1;
            continue;
        }

        if let Some(value) = arg.strip_prefix("--log-level=") {
            if let Ok(parsed) = value.parse::<LogLevel>() {
                settings.level = Some(parsed);
            }
            index += 1;
            continue;
        }

        if arg == "--log-level" {
            if let Some(next) = raw_args.get(index + 1)
                && let Ok(parsed) = next.parse::<LogLevel>()
            {
                settings.level = Some(parsed);
                index += 2;
                continue;
            }
            index += 1;
            continue;
        }

        if let Some(value) = arg.strip_prefix("--log-filter=") {
            settings.filter = Some(value.to_string());
            index += 1;
            continue;
        }

        if arg == "--log-filter" {
            if let Some(next) = raw_args.get(index + 1) {
                settings.filter = Some(next.clone());
                index += 2;
                continue;
            }
            index += 1;
            continue;
        }

        remaining.push(arg.clone());
        index += 1;
    }

    (settings, remaining)
}

fn parse_short_verbose(arg: &str) -> Option<u8> {
    if !arg.starts_with('-') || arg.starts_with("--") {
        return None;
    }

    let payload = arg.strip_prefix('-')?;
    if payload.is_empty() || !payload.chars().all(|ch| ch == 'v') {
        return None;
    }

    let count = u8::try_from(payload.len()).ok()?;
    Some(count)
}
