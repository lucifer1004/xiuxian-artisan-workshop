//! Tests for editor module - `StructuralEditor` functionality.

use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

use xiuxian_edit::StructuralEditor;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn test_simple_replace() -> TestResult {
    let content = "x = connect(host, port)";
    let result = StructuralEditor::replace(
        content,
        "connect($$$ARGS)",
        "async_connect($$$ARGS)",
        "python",
    )?;

    assert_eq!(result.count, 1);
    assert!(result.modified.contains("async_connect"));
    assert!(!result.diff.is_empty());
    Ok(())
}

#[test]
fn test_multiple_replacements() -> TestResult {
    let content = r"
def foo():
    connect(a, b)
    connect(c, d)
    connect(e, f)
";
    let result = StructuralEditor::replace(
        content,
        "connect($$$ARGS)",
        "safe_connect($$$ARGS)",
        "python",
    )?;

    assert_eq!(result.count, 3);
    assert_eq!(result.modified.matches("safe_connect").count(), 3);
    Ok(())
}

#[test]
fn test_no_matches() -> TestResult {
    let content = "x = 1 + 2";
    let result =
        StructuralEditor::replace(content, "connect($$$)", "async_connect($$$)", "python")?;

    assert_eq!(result.count, 0);
    assert_eq!(result.original, result.modified);
    Ok(())
}

#[test]
fn test_rust_replace() -> TestResult {
    let content = "let x = old_function(arg1, arg2);";
    let result = StructuralEditor::replace(
        content,
        "old_function($$$ARGS)",
        "new_function($$$ARGS)",
        "rust",
    )?;

    assert_eq!(result.count, 1);
    assert!(result.modified.contains("new_function"));
    Ok(())
}

#[test]
fn test_class_rename() -> TestResult {
    let content = r"
class OldName:
    pass

x = OldName()
";
    let result = StructuralEditor::replace(content, "OldName", "NewName", "python")?;

    assert!(result.count >= 1);
    assert!(result.modified.contains("NewName"));
    Ok(())
}

#[test]
fn test_file_preview() -> TestResult {
    let dir = TempDir::new()?;
    let path = dir.path().join("test.py");
    let content = "result = old_api(data)";

    File::create(&path)?.write_all(content.as_bytes())?;

    let result = StructuralEditor::preview(&path, "old_api($$$)", "new_api($$$)", None)?;

    assert_eq!(result.count, 1);

    let file_content = std::fs::read_to_string(&path)?;
    assert!(file_content.contains("old_api")); // Original unchanged
    Ok(())
}

#[test]
fn test_file_apply() -> TestResult {
    let dir = TempDir::new()?;
    let path = dir.path().join("test.py");
    let content = "result = deprecated_call(x)";

    File::create(&path)?.write_all(content.as_bytes())?;

    let result = StructuralEditor::apply(&path, "deprecated_call($$$)", "modern_call($$$)", None)?;

    assert_eq!(result.count, 1);

    let file_content = std::fs::read_to_string(&path)?;
    assert!(file_content.contains("modern_call"));
    assert!(!file_content.contains("deprecated_call"));
    Ok(())
}

#[test]
fn test_format_result() -> TestResult {
    let content = "x = connect(a)";
    let result = StructuralEditor::replace(
        content,
        "connect($$$ARGS)",
        "async_connect($$$ARGS)",
        "python",
    )?;

    let formatted = StructuralEditor::format_result(&result, Some("test.py"));

    assert!(formatted.contains("EDIT: test.py"));
    assert!(formatted.contains("Replacements: 1"));
    assert!(formatted.contains("Diff:"));
    Ok(())
}
