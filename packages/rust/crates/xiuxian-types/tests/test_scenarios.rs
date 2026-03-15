//! Scenario-based snapshot tests for xiuxian-types.

use std::error::Error;
use std::path::Path;

use serde_json::Value;
use xiuxian_testing::{Scenario, ScenarioFramework, ScenarioRunner};
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

#[test]
fn test_skill_definition_scenarios() {
    let mut framework = ScenarioFramework::new();
    framework.register(Box::new(SkillDefinitionRunner));
    framework.run_category("skill_definition").unwrap();
}
