use super::super::session_partition::DiscordSessionPartition;

const DISCORD_DEFAULT_INBOUND_QUEUE_CAPACITY: usize = 512;
const DISCORD_DEFAULT_TURN_TIMEOUT_SECS: u64 = 120;
const DISCORD_DEFAULT_FOREGROUND_MAX_IN_FLIGHT_MESSAGES: usize = 16;

/// Runtime configuration for Discord ingress/dispatch loop.
#[derive(Debug, Clone)]
pub struct DiscordRuntimeConfig {
    pub session_partition: DiscordSessionPartition,
    pub inbound_queue_capacity: usize,
    pub turn_timeout_secs: u64,
    pub foreground_max_in_flight_messages: usize,
}

impl DiscordRuntimeConfig {
    #[must_use]
    pub fn from_env() -> Self {
        Self::default()
    }
}

impl Default for DiscordRuntimeConfig {
    fn default() -> Self {
        let inbound_queue_capacity = std::env::var("OMNI_AGENT_DISCORD_INBOUND_QUEUE_CAPACITY")
            .ok()
            .and_then(|raw| raw.trim().parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(DISCORD_DEFAULT_INBOUND_QUEUE_CAPACITY);
        let turn_timeout_secs = std::env::var("OMNI_AGENT_DISCORD_TURN_TIMEOUT_SECS")
            .ok()
            .and_then(|raw| raw.trim().parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(DISCORD_DEFAULT_TURN_TIMEOUT_SECS);
        let foreground_max_in_flight_messages =
            std::env::var("OMNI_AGENT_DISCORD_FOREGROUND_MAX_IN_FLIGHT_MESSAGES")
                .ok()
                .and_then(|raw| raw.trim().parse::<usize>().ok())
                .filter(|value| *value > 0)
                .unwrap_or(DISCORD_DEFAULT_FOREGROUND_MAX_IN_FLIGHT_MESSAGES);
        Self {
            session_partition: DiscordSessionPartition::from_env(),
            inbound_queue_capacity,
            turn_timeout_secs,
            foreground_max_in_flight_messages,
        }
    }
}
