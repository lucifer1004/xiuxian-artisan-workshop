#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputFormat {
    Dashboard,
    Json,
}

impl OutputFormat {
    pub(crate) fn is_json(self) -> bool {
        matches!(self, Self::Json)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResumeCommand {
    Restore,
    Status,
    Drop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FeedbackDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SessionFeedbackCommand {
    pub(crate) direction: FeedbackDirection,
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JobStatusCommand {
    pub(crate) job_id: String,
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionPartitionCommand<Mode> {
    pub(crate) mode: Option<Mode>,
    pub(crate) format: OutputFormat,
}

#[cfg_attr(test, allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionMentionMode {
    Require,
    Open,
    Inherit,
}

#[cfg_attr(test, allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SessionMentionCommand {
    pub(crate) mode: Option<SessionMentionMode>,
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionPartitionModeToken {
    Chat,
    ChatUser,
    User,
    ChatThreadUser,
    GuildChannelUser,
    Channel,
    GuildUser,
}

pub(crate) const fn session_partition_mode_name(mode: SessionPartitionModeToken) -> &'static str {
    match mode {
        SessionPartitionModeToken::Chat => "chat",
        SessionPartitionModeToken::ChatUser => "chat_user",
        SessionPartitionModeToken::User => "user",
        SessionPartitionModeToken::ChatThreadUser => "chat_thread_user",
        SessionPartitionModeToken::GuildChannelUser => "guild_channel_user",
        SessionPartitionModeToken::Channel => "channel",
        SessionPartitionModeToken::GuildUser => "guild_user",
    }
}

pub(crate) fn parse_session_partition_mode_token(raw: &str) -> Option<SessionPartitionModeToken> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "chat" | "on" | "enable" | "enabled" | "shared" | "group" => {
            Some(SessionPartitionModeToken::Chat)
        }
        "chat_user" | "off" | "disable" | "disabled" | "isolated" | "chat-user" | "chatuser"
        | "channel_user" | "channel-user" | "channeluser" => {
            Some(SessionPartitionModeToken::ChatUser)
        }
        "user" | "user_only" | "user-only" | "useronly" => Some(SessionPartitionModeToken::User),
        "chat_thread_user" | "chat-thread-user" | "chatthreaduser" | "topic_user"
        | "topic-user" | "topicuser" => Some(SessionPartitionModeToken::ChatThreadUser),
        "guild_channel_user" | "guild-channel-user" | "guildchanneluser" => {
            Some(SessionPartitionModeToken::GuildChannelUser)
        }
        "channel" | "channel_only" | "channel-only" | "channelonly" => {
            Some(SessionPartitionModeToken::Channel)
        }
        "guild_user" | "guild-user" | "guilduser" => Some(SessionPartitionModeToken::GuildUser),
        _ => None,
    }
}
