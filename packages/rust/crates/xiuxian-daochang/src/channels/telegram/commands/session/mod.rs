mod admin;
mod injection;

use crate::channels::managed_runtime::parsing::{
    FeedbackDirection as SharedSessionFeedbackDirection, OutputFormat as SharedSessionOutputFormat,
    ResumeCommand as SharedResumeContextCommand,
    SessionFeedbackCommand as SharedSessionFeedbackCommand,
    SessionPartitionCommand as SharedSessionPartitionCommand, SessionPartitionModeToken,
    is_reset_context_command as is_reset_context_command_shared,
    is_stop_command as is_stop_command_shared, parse_resume_context_command as parse_resume_shared,
    parse_session_context_budget_command as parse_session_budget_shared,
    parse_session_context_memory_command as parse_session_memory_shared,
    parse_session_context_status_command as parse_session_status_shared,
    parse_session_feedback_command as parse_session_feedback_shared,
    parse_session_partition_command as parse_session_partition_shared,
    parse_session_partition_mode_token as parse_partition_mode_token,
};

pub(crate) use admin::parse_session_admin_command;
pub(crate) use injection::parse_session_injection_command;

pub(crate) type SessionOutputFormat = SharedSessionOutputFormat;
pub(crate) type ResumeContextCommand = SharedResumeContextCommand;
pub(crate) type SessionFeedbackDirection = SharedSessionFeedbackDirection;
pub(crate) type SessionFeedbackCommand = SharedSessionFeedbackCommand;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionPartitionMode {
    Chat,
    ChatUser,
    User,
    ChatThreadUser,
}

impl SessionPartitionMode {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Chat => "chat",
            Self::ChatUser => "chat_user",
            Self::User => "user",
            Self::ChatThreadUser => "chat_thread_user",
        }
    }
}

pub(crate) type SessionPartitionCommand = SharedSessionPartitionCommand<SessionPartitionMode>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SessionAdminAction {
    List,
    Set(Vec<String>),
    Add(Vec<String>),
    Remove(Vec<String>),
    Clear,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionAdminCommand {
    pub(crate) action: SessionAdminAction,
    pub(crate) format: SessionOutputFormat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SessionInjectionAction {
    Status,
    Clear,
    SetXml(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionInjectionCommand {
    pub(crate) action: SessionInjectionAction,
    pub(crate) format: SessionOutputFormat,
}

/// Parse session status command and return output format.
pub fn parse_session_context_status_command(input: &str) -> Option<SessionOutputFormat> {
    parse_session_status_shared(input)
}

/// Parse session budget command and return output format.
pub fn parse_session_context_budget_command(input: &str) -> Option<SessionOutputFormat> {
    parse_session_budget_shared(input)
}

/// Parse session memory command and return output format.
pub fn parse_session_context_memory_command(input: &str) -> Option<SessionOutputFormat> {
    parse_session_memory_shared(input)
}

/// Parse session partition command:
/// - `/session partition` (status)
/// - `/session scope` (status alias)
/// - `/session partition json`
/// - `/session scope json`
/// - `/session partition on|off`
/// - `/session scope on|off`
/// - `/session partition chat|chat_user|user|chat_thread_user [json]`
/// - `/session scope chat|chat_user|user|chat_thread_user [json]`
pub fn parse_session_partition_command(input: &str) -> Option<SessionPartitionCommand> {
    parse_session_partition_shared(input, parse_session_partition_mode)
}

/// Parse session recall-feedback command:
/// - `/session feedback up|down [json]`
/// - `/window feedback up|down [json]`
/// - `/context feedback up|down [json]`
/// - `/feedback up|down [json]`
pub fn parse_session_feedback_command(input: &str) -> Option<SessionFeedbackCommand> {
    parse_session_feedback_shared(input)
}

/// Parse `/reset`, `/clear`, `reset`, or `clear`.
pub fn is_reset_context_command(input: &str) -> bool {
    is_reset_context_command_shared(input)
}

/// Parse `/stop`, `/cancel`, `stop`, `cancel`, or `interrupt`.
pub fn is_stop_command(input: &str) -> bool {
    is_stop_command_shared(input)
}

/// Parse `/resume` or `resume`, with optional `/resume status`.
pub fn parse_resume_context_command(input: &str) -> Option<ResumeContextCommand> {
    parse_resume_shared(input)
}

fn parse_session_partition_mode(raw: &str) -> Option<SessionPartitionMode> {
    let token = parse_partition_mode_token(raw)?;
    match token {
        SessionPartitionModeToken::Chat => Some(SessionPartitionMode::Chat),
        SessionPartitionModeToken::ChatUser => Some(SessionPartitionMode::ChatUser),
        SessionPartitionModeToken::User => Some(SessionPartitionMode::User),
        SessionPartitionModeToken::ChatThreadUser => Some(SessionPartitionMode::ChatThreadUser),
        SessionPartitionModeToken::GuildChannelUser
        | SessionPartitionModeToken::Channel
        | SessionPartitionModeToken::GuildUser => None,
    }
}
