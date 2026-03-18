//! Scenario-based snapshot tests for xiuxian-types.

use std::error::Error;
use std::path::{Path, PathBuf};

use serde_json::Value;
use xiuxian_testing::{Scenario, ScenarioFramework, ScenarioRunner, ScenarioSnapshotPolicy};
use xiuxian_types::SkillDefinition;

struct SkillDefinitionRunner;

impl ScenarioRunner for SkillDefinitionRunner {
    fn category(&self) -> &str {
        "skill_definition"
    }

    fn run(&self, _scenario: &Scenario, temp_dir: &Path) -> Result<Value, Box<dyn Error>> {
        let input_path = temp_dir.join("skill.json");
        let raw = std::fs::read_to_string(&input_path)?;
        let value: serde_json::Value = serde_json::from_str(&raw)?;
        let def: SkillDefinition = serde_json::from_value(value)?;
        let output = serde_json::to_value(def)?;
        Ok(output)
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn test_skill_definition_scenarios() {
    let manifest = manifest_dir();
    let scenarios_root = manifest.join("tests").join("fixtures").join("scenarios");
    let snapshot_path = manifest.join("tests").join("snapshots");

    let mut framework = ScenarioFramework::with_snapshot_path(&snapshot_path)
        .with_snapshot_policy(ScenarioSnapshotPolicy::portable_ci());
    framework.register(Box::new(SkillDefinitionRunner));
    let count = framework
        .run_all_at(&scenarios_root)
        .unwrap_or_else(|error| panic!("skill definition scenarios should pass: {error}"));
    assert!(
        count > 0,
        "should run at least one skill definition scenario"
    );
}
