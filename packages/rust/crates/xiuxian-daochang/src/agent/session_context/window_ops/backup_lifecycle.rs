use anyhow::Result;

use crate::agent::Agent;
use crate::shortcuts::parse_react_shortcut;

impl Agent {
    pub(crate) async fn handle_shortcuts(
        &self,
        _session_id: &str,
        user_message_owned: &mut String,
        force_react: &mut bool,
        _turn_id: u64,
    ) -> Result<Option<String>> {
        if let Some(rewritten) = parse_react_shortcut(user_message_owned.as_str()) {
            *force_react = true;
            *user_message_owned = rewritten;
        }

        Ok(None)
    }
}
