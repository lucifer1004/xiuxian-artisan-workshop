pub(in crate::agent) struct ToolCallOutput {
    pub(in crate::agent) text: String,
    pub(in crate::agent) is_error: bool,
    pub(in crate::agent) tool_call_id: Option<String>,
}

impl ToolCallOutput {
    pub(in crate::agent) fn success(text: String, tool_call_id: Option<&str>) -> Self {
        Self::new(text, false, tool_call_id)
    }

    pub(in crate::agent) fn error(text: String, tool_call_id: Option<&str>) -> Self {
        Self::new(text, true, tool_call_id)
    }

    fn new(text: String, is_error: bool, tool_call_id: Option<&str>) -> Self {
        Self {
            text,
            is_error,
            tool_call_id: normalize_tool_call_id(tool_call_id),
        }
    }
}

fn normalize_tool_call_id(tool_call_id: Option<&str>) -> Option<String> {
    tool_call_id
        .map(|tool_call_id| tool_call_id.split('|').next().unwrap_or(tool_call_id))
        .map(str::trim)
        .filter(|tool_call_id| !tool_call_id.is_empty())
        .map(ToString::to_string)
}
