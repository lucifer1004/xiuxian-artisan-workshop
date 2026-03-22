use crate::gateway::studio::router::StudioState;
use std::env;
use std::path::{Path, PathBuf};

pub fn resolve_path_like(base: &Path, input: &str) -> Option<PathBuf> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let expanded = expand_home_path_like(trimmed)?;
    let path = expanded.as_path();
    if path.is_absolute() {
        Some(path.to_path_buf())
    } else {
        Some(base.join(path))
    }
}

pub fn normalize_project_dir_root(dir: &str) -> Option<String> {
    normalize_path_like(dir)
}

pub fn normalize_path_like(raw: &str) -> Option<String> {
    let mut normalized = raw.trim().replace('\\', "/");
    if normalized.is_empty() {
        return None;
    }

    while normalized.contains("//") {
        normalized = normalized.replace("//", "/");
    }

    while normalized.len() > 1
        && normalized.ends_with('/')
        && !is_windows_drive_root(normalized.as_str())
    {
        normalized.pop();
    }

    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

pub fn studio_display_path(state: &StudioState, internal_path: &str) -> String {
    // Current MVP implementation: try to strip project root
    if let Ok(relative) = Path::new(internal_path).strip_prefix(&state.project_root) {
        relative.to_string_lossy().to_string()
    } else {
        internal_path.to_string()
    }
}

fn is_windows_drive_root(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 3 && bytes[1] == b':' && bytes[2] == b'/'
}

fn expand_home_path_like(input: &str) -> Option<PathBuf> {
    if input == "~" {
        return home_dir();
    }

    if let Some(relative) = input
        .strip_prefix("~/")
        .or_else(|| input.strip_prefix("~\\"))
    {
        return home_dir().map(|path| path.join(relative));
    }

    Some(PathBuf::from(input))
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from))
        .or_else(|| {
            let drive = env::var_os("HOMEDRIVE")?;
            let path = env::var_os("HOMEPATH")?;
            let mut combined = PathBuf::from(drive);
            combined.push(path);
            Some(combined)
        })
}

#[cfg(test)]
mod tests {
    use super::resolve_path_like;
    use std::path::Path;

    #[test]
    fn resolve_path_like_expands_tilde_prefixed_home_paths() {
        let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) else {
            return;
        };

        let resolved = resolve_path_like(Path::new("/tmp/studio"), "~/workspace/docs")
            .unwrap_or_else(|| panic!("tilde-prefixed path should resolve"));

        assert_eq!(resolved, home.join("workspace/docs"));
    }

    #[test]
    fn resolve_path_like_keeps_relative_paths_rooted_at_base() {
        let resolved = resolve_path_like(Path::new("/tmp/studio"), "docs")
            .unwrap_or_else(|| panic!("relative path should resolve"));

        assert_eq!(resolved, std::path::PathBuf::from("/tmp/studio/docs"));
    }
}
