use std::path::Path;

pub fn sanitize_path(text: &str, skill_path: &Path) -> String {
    text.replace(skill_path.to_string_lossy().as_ref(), "<SKILL_PATH>")
}
