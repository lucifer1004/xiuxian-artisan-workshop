pub(super) use crate::channels::managed_runtime::parsing::{
    FeedbackDirection, ResumeCommand, SessionFeedbackCommand,
    SessionPartitionCommand as SharedSessionPartitionCommand,
};
use crate::channels::managed_runtime::parsing::{
    OutputFormat, SessionPartitionModeToken, normalize_command_input, parse_background_prompt,
    parse_help_command, parse_job_status_command, parse_jobs_summary_command,
    parse_resume_context_command, parse_session_context_budget_command,
    parse_session_context_memory_command, parse_session_context_status_command,
    parse_session_feedback_command,
    parse_session_partition_command as parse_session_partition_shared,
    parse_session_partition_mode_token as parse_partition_mode_token,
    slice_original_command_suffix,
};

use super::super::super::session_partition::DiscordSessionPartition;

pub(super) type CommandOutputFormat = OutputFormat;
type SessionPartitionMode = DiscordSessionPartition;
pub(super) type SessionPartitionCommand = SharedSessionPartitionCommand<SessionPartitionMode>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SessionInjectionAction {
    Status,
    Clear,
    SetXml(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SessionInjectionCommand {
    pub(super) action: SessionInjectionAction,
    pub(super) format: CommandOutputFormat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SessionAdminAction {
    List,
    Set(Vec<String>),
    Add(Vec<String>),
    Remove(Vec<String>),
    Clear,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SessionAdminCommand {
    pub(super) action: SessionAdminAction,
    pub(super) format: CommandOutputFormat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ManagedCommand {
    Help(CommandOutputFormat),
    Reset,
    Resume(ResumeCommand),
    SessionStatus(CommandOutputFormat),
    SessionBudget(CommandOutputFormat),
    SessionMemory(CommandOutputFormat),
    SessionFeedback(SessionFeedbackCommand),
    SessionPartition(SessionPartitionCommand),
    SessionAdmin(SessionAdminCommand),
    SessionInjection(SessionInjectionCommand),
    JobStatus {
        job_id: String,
        format: CommandOutputFormat,
    },
    JobsSummary(CommandOutputFormat),
    BackgroundSubmit(String),
}

pub(super) fn parse_managed_command(input: &str) -> Option<ManagedCommand> {
    if let Some(format) = parse_help_command(input) {
        return Some(ManagedCommand::Help(format));
    }
    if crate::channels::managed_runtime::parsing::is_reset_context_command(input) {
        return Some(ManagedCommand::Reset);
    }
    if let Some(resume) = parse_resume_context_command(input) {
        return Some(ManagedCommand::Resume(resume));
    }
    if let Some(command) = parse_session_admin_command(input) {
        return Some(ManagedCommand::SessionAdmin(command));
    }
    if let Some(command) = parse_session_injection_command(input) {
        return Some(ManagedCommand::SessionInjection(command));
    }
    if let Some(command) = parse_session_partition_command(input) {
        return Some(ManagedCommand::SessionPartition(command));
    }
    if let Some(format) = parse_session_context_status_command(input) {
        return Some(ManagedCommand::SessionStatus(format));
    }
    if let Some(format) = parse_session_context_budget_command(input) {
        return Some(ManagedCommand::SessionBudget(format));
    }
    if let Some(format) = parse_session_context_memory_command(input) {
        return Some(ManagedCommand::SessionMemory(format));
    }
    if let Some(command) = parse_session_feedback_command(input) {
        return Some(ManagedCommand::SessionFeedback(command));
    }
    if let Some(command) = parse_job_status_command(input) {
        return Some(ManagedCommand::JobStatus {
            job_id: command.job_id,
            format: command.format,
        });
    }
    if let Some(format) = parse_jobs_summary_command(input) {
        return Some(ManagedCommand::JobsSummary(format));
    }
    if let Some(prompt) = parse_background_prompt(input) {
        return Some(ManagedCommand::BackgroundSubmit(prompt));
    }
    None
}

fn parse_session_partition_command(input: &str) -> Option<SessionPartitionCommand> {
    parse_session_partition_shared(input, parse_session_partition_mode)
}

fn parse_session_partition_mode(raw: &str) -> Option<SessionPartitionMode> {
    let token = parse_partition_mode_token(raw)?;
    match token {
        SessionPartitionModeToken::Chat | SessionPartitionModeToken::Channel => {
            Some(DiscordSessionPartition::ChannelOnly)
        }
        SessionPartitionModeToken::ChatUser
        | SessionPartitionModeToken::ChatThreadUser
        | SessionPartitionModeToken::GuildChannelUser => {
            Some(DiscordSessionPartition::GuildChannelUser)
        }
        SessionPartitionModeToken::User => Some(DiscordSessionPartition::UserOnly),
        SessionPartitionModeToken::GuildUser => Some(DiscordSessionPartition::GuildUser),
    }
}

fn parse_session_injection_command(input: &str) -> Option<SessionInjectionCommand> {
    let normalized = normalize_command_input(input);
    let lowered = normalized.to_ascii_lowercase();
    let prefixes = [
        "session inject",
        "window inject",
        "context inject",
        "session injection",
        "window injection",
        "context injection",
    ];

    let rest = prefixes.iter().find_map(|prefix| {
        lowered.strip_prefix(prefix).and_then(|suffix| {
            if suffix.trim().is_empty() {
                Some(String::new())
            } else {
                slice_original_command_suffix(normalized, suffix).map(ToString::to_string)
            }
        })
    })?;

    let tail = rest.trim();
    if tail.is_empty() {
        return Some(SessionInjectionCommand {
            action: SessionInjectionAction::Status,
            format: CommandOutputFormat::Dashboard,
        });
    }
    if tail.eq_ignore_ascii_case("json") {
        return Some(SessionInjectionCommand {
            action: SessionInjectionAction::Status,
            format: CommandOutputFormat::Json,
        });
    }
    if tail.eq_ignore_ascii_case("status") {
        return Some(SessionInjectionCommand {
            action: SessionInjectionAction::Status,
            format: CommandOutputFormat::Dashboard,
        });
    }
    if tail.eq_ignore_ascii_case("status json") {
        return Some(SessionInjectionCommand {
            action: SessionInjectionAction::Status,
            format: CommandOutputFormat::Json,
        });
    }
    if tail.eq_ignore_ascii_case("clear") {
        return Some(SessionInjectionCommand {
            action: SessionInjectionAction::Clear,
            format: CommandOutputFormat::Dashboard,
        });
    }
    if tail.eq_ignore_ascii_case("clear json") {
        return Some(SessionInjectionCommand {
            action: SessionInjectionAction::Clear,
            format: CommandOutputFormat::Json,
        });
    }
    if tail.eq_ignore_ascii_case("set") {
        return None;
    }
    let lowered_tail = tail.to_ascii_lowercase();
    if lowered_tail.starts_with("status ") || lowered_tail.starts_with("clear ") {
        return None;
    }

    let payload = if lowered_tail.starts_with("set ") {
        tail[4..].trim()
    } else {
        tail
    };
    if payload.is_empty() {
        return None;
    }
    Some(SessionInjectionCommand {
        action: SessionInjectionAction::SetXml(payload.to_string()),
        format: CommandOutputFormat::Dashboard,
    })
}

fn parse_session_admin_command(input: &str) -> Option<SessionAdminCommand> {
    let normalized = normalize_command_input(input);
    let mut parts = normalized.split_whitespace();
    let root = parts.next()?;
    if !root.eq_ignore_ascii_case("session")
        && !root.eq_ignore_ascii_case("window")
        && !root.eq_ignore_ascii_case("context")
    {
        return None;
    }
    let sub = parts.next()?;
    if !sub.eq_ignore_ascii_case("admin") {
        return None;
    }

    let tokens: Vec<&str> = parts.collect();
    if tokens.is_empty() {
        return Some(SessionAdminCommand {
            action: SessionAdminAction::List,
            format: CommandOutputFormat::Dashboard,
        });
    }
    if tokens.len() == 1 && tokens[0].eq_ignore_ascii_case("json") {
        return Some(SessionAdminCommand {
            action: SessionAdminAction::List,
            format: CommandOutputFormat::Json,
        });
    }

    let mut format = CommandOutputFormat::Dashboard;
    let args_end = if tokens
        .last()
        .is_some_and(|token| token.eq_ignore_ascii_case("json"))
    {
        format = CommandOutputFormat::Json;
        tokens.len().saturating_sub(1)
    } else {
        tokens.len()
    };
    if args_end == 0 {
        return None;
    }

    let command = tokens[0];
    let id_tokens = &tokens[1..args_end];
    let action = if command.eq_ignore_ascii_case("list") {
        if !id_tokens.is_empty() {
            return None;
        }
        SessionAdminAction::List
    } else if command.eq_ignore_ascii_case("clear") {
        if !id_tokens.is_empty() {
            return None;
        }
        SessionAdminAction::Clear
    } else if command.eq_ignore_ascii_case("set") {
        SessionAdminAction::Set(parse_admin_user_ids(id_tokens)?)
    } else if command.eq_ignore_ascii_case("add") {
        SessionAdminAction::Add(parse_admin_user_ids(id_tokens)?)
    } else if command.eq_ignore_ascii_case("remove")
        || command.eq_ignore_ascii_case("rm")
        || command.eq_ignore_ascii_case("del")
    {
        SessionAdminAction::Remove(parse_admin_user_ids(id_tokens)?)
    } else if command.eq_ignore_ascii_case("json") {
        return None;
    } else {
        SessionAdminAction::Set(parse_admin_user_ids(&tokens[..args_end])?)
    };

    Some(SessionAdminCommand { action, format })
}

fn parse_admin_user_ids(raw_tokens: &[&str]) -> Option<Vec<String>> {
    if raw_tokens.is_empty() {
        return None;
    }
    let values: Vec<String> = raw_tokens
        .iter()
        .flat_map(|token| token.split(','))
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(ToString::to_string)
        .collect();
    if values.is_empty() {
        return None;
    }
    Some(values)
}
