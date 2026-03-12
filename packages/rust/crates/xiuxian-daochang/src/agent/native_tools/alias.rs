use std::sync::Arc;

use serde_json::Value;

use super::registry::{NativeTool, NativeToolCallContext};

pub(crate) struct NativeAliasTool {
    alias_name: String,
    description: String,
    parameters: Value,
    target: Arc<dyn NativeTool>,
}

impl NativeAliasTool {
    pub(crate) fn new(
        alias_name: String,
        description: String,
        parameters: Value,
        target: Arc<dyn NativeTool>,
    ) -> Self {
        Self {
            alias_name,
            description,
            parameters,
            target,
        }
    }
}

#[async_trait::async_trait]
impl NativeTool for NativeAliasTool {
    fn name(&self) -> &str {
        self.alias_name.as_str()
    }

    fn description(&self) -> &str {
        self.description.as_str()
    }

    fn parameters(&self) -> Value {
        self.parameters.clone()
    }

    async fn call(
        &self,
        arguments: Option<Value>,
        context: &NativeToolCallContext,
    ) -> anyhow::Result<String> {
        self.target.call(arguments, context).await
    }
}
