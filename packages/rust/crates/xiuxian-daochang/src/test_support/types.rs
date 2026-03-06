#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Dashboard,
    Json,
}

impl OutputFormat {
    #[must_use]
    pub const fn is_json(self) -> bool {
        matches!(self, Self::Json)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobStatusCommand {
    pub job_id: String,
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResumeContextCommand {
    Restore,
    Status,
    Drop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionFeedbackDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionFeedbackCommand {
    pub direction: SessionFeedbackDirection,
    pub format: OutputFormat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionInjectionAction {
    Status,
    Clear,
    SetXml(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionInjectionCommand {
    pub action: SessionInjectionAction,
    pub format: OutputFormat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionAdminAction {
    List,
    Set(Vec<String>),
    Add(Vec<String>),
    Remove(Vec<String>),
    Clear,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionAdminCommand {
    pub action: SessionAdminAction,
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionPartitionMode {
    Chat,
    ChatUser,
    User,
    ChatThreadUser,
}

impl SessionPartitionMode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Chat => "chat",
            Self::ChatUser => "chat_user",
            Self::User => "user",
            Self::ChatThreadUser => "chat_thread_user",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionPartitionCommand {
    pub mode: Option<SessionPartitionMode>,
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedSlashCommand {
    SessionStatus,
    SessionBudget,
    SessionMemory,
    SessionFeedback,
    JobStatus,
    JobsSummary,
    BackgroundSubmit,
}

impl ManagedSlashCommand {
    #[must_use]
    pub const fn scope(self) -> &'static str {
        match self {
            Self::SessionStatus => "session.status",
            Self::SessionBudget => "session.budget",
            Self::SessionMemory => "session.memory",
            Self::SessionFeedback => "session.feedback",
            Self::JobStatus => "job.status",
            Self::JobsSummary => "jobs.summary",
            Self::BackgroundSubmit => "background.submit",
        }
    }

    #[must_use]
    pub const fn canonical_command(self) -> &'static str {
        match self {
            Self::SessionStatus => "/session",
            Self::SessionBudget => "/session budget",
            Self::SessionMemory => "/session memory",
            Self::SessionFeedback => "/session feedback",
            Self::JobStatus => "/job",
            Self::JobsSummary => "/jobs",
            Self::BackgroundSubmit => "/bg",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedControlCommand {
    Reset,
    ResumeRestore,
    ResumeStatus,
    ResumeDrop,
    SessionAdmin,
    SessionInjection,
    SessionPartition,
}

impl ManagedControlCommand {
    #[must_use]
    pub const fn canonical_command(self) -> &'static str {
        match self {
            Self::Reset => "/reset",
            Self::ResumeRestore => "/resume",
            Self::ResumeStatus => "/resume status",
            Self::ResumeDrop => "/resume drop",
            Self::SessionAdmin => "/session admin",
            Self::SessionInjection => "/session inject",
            Self::SessionPartition => "/session partition",
        }
    }
}

pub(crate) fn map_output_format(is_json: bool) -> OutputFormat {
    if is_json {
        OutputFormat::Json
    } else {
        OutputFormat::Dashboard
    }
}
