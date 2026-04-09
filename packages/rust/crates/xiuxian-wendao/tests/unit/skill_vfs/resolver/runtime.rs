use super::SkillVfsResolver;
use std::path::Path;

#[test]
fn resolve_runtime_internal_root_with_resolves_relative_override() {
    let resolved = SkillVfsResolver::resolve_runtime_internal_root_with(
        Path::new("/repo/project"),
        Some(" internal_skills/custom "),
    );
    assert_eq!(resolved, Path::new("/repo/project/internal_skills/custom"));
}

#[test]
fn resolve_runtime_internal_root_with_preserves_absolute_override() {
    let resolved = SkillVfsResolver::resolve_runtime_internal_root_with(
        Path::new("/repo/project"),
        Some(" /tmp/internal-skills "),
    );
    assert_eq!(resolved, Path::new("/tmp/internal-skills"));
}
