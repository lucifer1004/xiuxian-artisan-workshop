//! Shared Discord runtime test support for mock channels and agent harnesses.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::{Mutex, RwLock, mpsc};
pub(super) use xiuxian_daochang::test_support::{
    DiscordForegroundInterruptController as ForegroundInterruptController,
    build_discord_foreground_runtime, process_discord_message_with_interrupt,
};
use xiuxian_daochang::{
    Agent, AgentConfig, Channel, ChannelMessage, ForegroundQueueMode, JobManager, JobManagerConfig,
    RecipientCommandAdminUsersMutation, TurnRunner, set_config_home_override,
};

pub(super) async fn process_discord_message(
    agent: Arc<Agent>,
    channel: Arc<dyn Channel>,
    msg: ChannelMessage,
    job_manager: &Arc<JobManager>,
    turn_timeout_secs: u64,
) {
    let interrupt_controller = ForegroundInterruptController::default();
    process_discord_message_with_interrupt(
        agent,
        channel,
        msg,
        job_manager,
        turn_timeout_secs,
        ForegroundQueueMode::Queue,
        &interrupt_controller,
    )
    .await;
}

#[derive(Default)]
pub(super) struct MockChannel {
    sent: Mutex<Vec<(String, String)>>,
    partition_mode: RwLock<String>,
    allow_control_commands: bool,
    denied_slash_scopes: Vec<String>,
    recipient_admin_users: RwLock<std::collections::HashMap<String, Vec<String>>>,
}

impl MockChannel {
    pub(super) fn with_acl(
        allow_control_commands: bool,
        denied_slash_scopes: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Self {
        Self {
            sent: Mutex::new(Vec::new()),
            partition_mode: RwLock::new("guild_channel_user".to_string()),
            allow_control_commands,
            denied_slash_scopes: denied_slash_scopes
                .into_iter()
                .map(|scope| scope.as_ref().to_string())
                .collect(),
            recipient_admin_users: RwLock::new(std::collections::HashMap::new()),
        }
    }

    pub(super) async fn sent_messages(&self) -> Vec<(String, String)> {
        self.sent.lock().await.clone()
    }

    pub(super) async fn partition_mode(&self) -> String {
        self.partition_mode.read().await.clone()
    }
}

#[async_trait]
impl Channel for MockChannel {
    fn name(&self) -> &'static str {
        "discord-runtime-mock"
    }

    fn session_partition_mode(&self) -> Option<String> {
        Some(
            self.partition_mode
                .try_read()
                .map_or_else(|_| "guild_channel_user".to_string(), |guard| guard.clone()),
        )
    }

    fn set_session_partition_mode(&self, mode: &str) -> anyhow::Result<()> {
        if let Ok(mut guard) = self.partition_mode.try_write() {
            *guard = mode.to_string();
            Ok(())
        } else {
            Err(anyhow::anyhow!("failed to acquire partition write lock"))
        }
    }

    fn is_authorized_for_control_command(&self, _identity: &str, _command_text: &str) -> bool {
        self.allow_control_commands
    }

    fn is_authorized_for_control_command_for_recipient(
        &self,
        identity: &str,
        _command_text: &str,
        recipient: &str,
    ) -> bool {
        if self.allow_control_commands {
            return true;
        }
        self.recipient_admin_users
            .try_read()
            .ok()
            .and_then(|guard| guard.get(recipient).cloned())
            .is_some_and(|admins| admins.iter().any(|entry| entry == "*" || entry == identity))
    }

    fn is_authorized_for_slash_command(&self, _identity: &str, command_scope: &str) -> bool {
        !self
            .denied_slash_scopes
            .iter()
            .any(|scope| scope == command_scope)
    }

    fn is_authorized_for_slash_command_for_recipient(
        &self,
        identity: &str,
        command_scope: &str,
        recipient: &str,
    ) -> bool {
        if self.is_authorized_for_slash_command(identity, command_scope) {
            return true;
        }
        self.recipient_admin_users
            .try_read()
            .ok()
            .and_then(|guard| guard.get(recipient).cloned())
            .is_some_and(|admins| admins.iter().any(|entry| entry == "*" || entry == identity))
    }

    fn recipient_command_admin_users(
        &self,
        recipient: &str,
    ) -> anyhow::Result<Option<Vec<String>>> {
        Ok(self
            .recipient_admin_users
            .try_read()
            .ok()
            .and_then(|guard| guard.get(recipient).cloned()))
    }

    fn mutate_recipient_command_admin_users(
        &self,
        recipient: &str,
        mutation: RecipientCommandAdminUsersMutation,
    ) -> anyhow::Result<Option<Vec<String>>> {
        let recipient = recipient.trim();
        if recipient.is_empty() {
            return Err(anyhow::anyhow!("recipient is required"));
        }
        let mut guard = self
            .recipient_admin_users
            .try_write()
            .map_err(|_| anyhow::anyhow!("failed to acquire recipient ACL lock"))?;
        let current = guard.get(recipient).cloned();
        let next = match mutation {
            RecipientCommandAdminUsersMutation::Clear => None,
            RecipientCommandAdminUsersMutation::Set(entries) => Some(entries),
            RecipientCommandAdminUsersMutation::Add(entries) => {
                let mut merged = current.unwrap_or_default();
                merged.extend(entries);
                Some(merged)
            }
            RecipientCommandAdminUsersMutation::Remove(entries) => {
                let Some(existing) = current else {
                    return Ok(None);
                };
                let filtered: Vec<String> = existing
                    .into_iter()
                    .filter(|entry| !entries.iter().any(|candidate| candidate == entry))
                    .collect();
                if filtered.is_empty() {
                    None
                } else {
                    Some(filtered)
                }
            }
        };
        match next.clone() {
            Some(entries) => {
                guard.insert(recipient.to_string(), entries);
            }
            None => {
                guard.remove(recipient);
            }
        }
        Ok(next)
    }

    async fn send(&self, message: &str, recipient: &str) -> Result<()> {
        self.sent
            .lock()
            .await
            .push((message.to_string(), recipient.to_string()));
        Ok(())
    }

    async fn listen(&self, _tx: mpsc::Sender<ChannelMessage>) -> Result<()> {
        Ok(())
    }
}

pub(super) fn inbound(content: &str) -> ChannelMessage {
    inbound_for_session(content, "3001:2001:1001")
}

pub(super) fn inbound_for_session(content: &str, session_key: &str) -> ChannelMessage {
    ChannelMessage {
        id: format!("discord_msg_{session_key}"),
        sender: "1001".to_string(),
        recipient: "2001".to_string(),
        session_key: session_key.to_string(),
        content: content.to_string(),
        attachments: Vec::new(),
        channel: "discord".to_string(),
        timestamp: 0,
    }
}

pub(super) async fn build_agent() -> Result<Arc<Agent>> {
    build_agent_with_inference_url("http://127.0.0.1:1/v1/chat/completions").await
}

pub(super) async fn build_agent_with_inference_url(inference_url: &str) -> Result<Arc<Agent>> {
    ensure_http_llm_backend_for_tests();
    let config = AgentConfig {
        inference_url: inference_url.to_string(),
        model: "gpt-4o-mini".to_string(),
        api_key: None,
        max_tool_rounds: 1,
        ..AgentConfig::default()
    };
    Ok(Arc::new(Agent::from_config(config).await?))
}

pub(super) fn start_job_manager(agent: &Arc<Agent>) -> Arc<JobManager> {
    let runner: Arc<dyn TurnRunner> = agent.clone();
    let (manager, _completion_rx) = JobManager::start(runner, JobManagerConfig::default());
    manager
}

fn ensure_http_llm_backend_for_tests() {
    static CONFIG_HOME: OnceLock<PathBuf> = OnceLock::new();
    let path = CONFIG_HOME.get_or_init(|| {
        let root = std::env::temp_dir()
            .join("xiuxian-daochang-tests")
            .join("discord-runtime");
        let settings_dir = root.join("xiuxian-artisan-workshop");
        std::fs::create_dir_all(&settings_dir)
            .expect("create isolated config home for discord runtime tests");
        std::fs::write(
            settings_dir.join("xiuxian.toml"),
            "[agent]\nllm_backend = \"http\"\nagenda_validation_policy = \"never\"\n",
        )
        .expect("write isolated runtime settings for discord runtime tests");
        root
    });
    set_config_home_override(path.clone());
}
