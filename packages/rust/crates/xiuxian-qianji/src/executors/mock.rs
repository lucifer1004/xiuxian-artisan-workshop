//! A simple mock mechanism for testing and simulation.

use crate::contracts::{FlowInstruction, QianjiMechanism, QianjiOutput};
use async_trait::async_trait;
use serde_json::{Value, json};

/// A simple mock mechanism for testing and simulation.
pub struct MockMechanism {
    /// Friendly name of the mock node.
    pub name: String,
    /// Scheduling weight.
    pub weight: f32,
    /// Optional static output key.
    pub output_key: Option<String>,
    /// Optional static output data.
    pub mock_output: Option<Value>,
}

#[async_trait]
impl QianjiMechanism for MockMechanism {
    async fn execute(&self, _context: &serde_json::Value) -> Result<QianjiOutput, String> {
        // Simulate some work
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        let data = if let Some(key) = &self.output_key {
            let val = self.mock_output.clone().unwrap_or(json!("done"));
            let mut map = serde_json::Map::new();
            map.insert(key.clone(), val);
            Value::Object(map)
        } else {
            json!({ self.name.clone(): "done" })
        };

        Ok(QianjiOutput {
            data,
            instruction: FlowInstruction::Continue,
        })
    }

    fn weight(&self) -> f32 {
        self.weight
    }
}
