use std::collections::HashSet;

use crate::session::ChatMessage;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ToolMessageIntegrityReport {
    pub incomplete_assistants: usize,
    pub linked_tools: usize,
    pub orphan_tools: usize,
    pub empty_tool_call_assistants: usize,
}

impl ToolMessageIntegrityReport {
    #[must_use]
    pub(crate) fn dropped_total(self) -> usize {
        self.incomplete_assistants
            .saturating_add(self.linked_tools)
            .saturating_add(self.orphan_tools)
            .saturating_add(self.empty_tool_call_assistants)
    }
}

#[derive(Debug)]
struct PendingAssistant {
    message_index: usize,
    required_ids: HashSet<String>,
    satisfied_ids: HashSet<String>,
    linked_tool_message_indices: Vec<usize>,
}

fn drop_pending_assistant(
    pending: PendingAssistant,
    report: &mut ToolMessageIntegrityReport,
    dropped_indices: &mut HashSet<usize>,
) {
    if dropped_indices.insert(pending.message_index) {
        report.incomplete_assistants = report.incomplete_assistants.saturating_add(1);
    }
    for linked_idx in pending.linked_tool_message_indices {
        if dropped_indices.insert(linked_idx) {
            report.linked_tools = report.linked_tools.saturating_add(1);
        }
    }
}

fn collect_required_ids<P: HygienePolicy + ?Sized>(
    tool_calls: &[crate::session::ToolCallOut],
    policy: &P,
) -> HashSet<String> {
    tool_calls
        .iter()
        .filter_map(|call| {
            policy
                .normalize_tool_call_id(call.id.as_str())
                .map(ToString::to_string)
        })
        .collect()
}

/// Provider-specific sanitation policy for outbound chat transcripts.
pub(crate) trait HygienePolicy {
    /// Gives the policy a chance to reshape messages before chain repair.
    fn preprocess_messages(&self, messages: Vec<ChatMessage>) -> Vec<ChatMessage> {
        messages
    }

    /// Normalizes raw tool-call identifiers into provider-stable protocol IDs.
    fn normalize_tool_call_id<'a>(&self, raw_id: &'a str) -> Option<&'a str> {
        raw_id
            .split('|')
            .next()
            .map(str::trim)
            .filter(|id| !id.is_empty())
    }

    /// Gives the policy a chance to adjust the repaired transcript and report.
    fn postprocess_messages(
        &self,
        messages: Vec<ChatMessage>,
        report: ToolMessageIntegrityReport,
    ) -> (Vec<ChatMessage>, ToolMessageIntegrityReport) {
        (messages, report)
    }

    /// Repairs or discards inconsistent messages before provider dispatch.
    fn sanitize_messages(
        &self,
        messages: Vec<ChatMessage>,
    ) -> (Vec<ChatMessage>, ToolMessageIntegrityReport) {
        let messages = self.preprocess_messages(messages);
        let (messages, report) = sanitize_tool_message_chain(messages, self);
        self.postprocess_messages(messages, report)
    }
}

/// OpenAI-compatible transcript hygiene policy.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OpenAiHygienePolicy;

impl HygienePolicy for OpenAiHygienePolicy {}

/// Ensure LLM message history preserves tool-call integrity.
///
/// OpenAI-compatible providers require each assistant message that includes
/// `tool_calls` to be followed by matching `tool` messages with corresponding
/// `tool_call_id` values. This function removes incomplete chains and orphaned
/// tool messages before dispatching requests.
pub(crate) fn enforce_tool_message_integrity(
    messages: Vec<ChatMessage>,
) -> (Vec<ChatMessage>, ToolMessageIntegrityReport) {
    OpenAiHygienePolicy.sanitize_messages(messages)
}

fn sanitize_tool_message_chain<P: HygienePolicy + ?Sized>(
    messages: Vec<ChatMessage>,
    policy: &P,
) -> (Vec<ChatMessage>, ToolMessageIntegrityReport) {
    if messages.is_empty() {
        return (messages, ToolMessageIntegrityReport::default());
    }

    let mut report = ToolMessageIntegrityReport::default();
    let mut kept = Vec::with_capacity(messages.len());
    let mut dropped_indices: HashSet<usize> = HashSet::new();
    let mut active_pending: Option<PendingAssistant> = None;

    for message in messages {
        let idx = kept.len();
        kept.push(message);
        let message_ref = &kept[idx];

        if let Some(mut pending) = active_pending.take() {
            if message_ref.role == "tool" {
                let Some(tool_call_id) = message_ref.tool_call_id.as_deref() else {
                    dropped_indices.insert(idx);
                    report.orphan_tools = report.orphan_tools.saturating_add(1);
                    active_pending = Some(pending);
                    continue;
                };
                let Some(normalized_id) = policy.normalize_tool_call_id(tool_call_id) else {
                    dropped_indices.insert(idx);
                    report.orphan_tools = report.orphan_tools.saturating_add(1);
                    active_pending = Some(pending);
                    continue;
                };
                if !pending.required_ids.contains(normalized_id)
                    || pending.satisfied_ids.contains(normalized_id)
                {
                    dropped_indices.insert(idx);
                    report.orphan_tools = report.orphan_tools.saturating_add(1);
                    active_pending = Some(pending);
                    continue;
                }

                pending.satisfied_ids.insert(normalized_id.to_string());
                pending.linked_tool_message_indices.push(idx);
                if pending.required_ids.len() != pending.satisfied_ids.len() {
                    active_pending = Some(pending);
                }
                continue;
            }

            drop_pending_assistant(pending, &mut report, &mut dropped_indices);
        }

        if message_ref.role == "assistant" {
            let Some(tool_calls) = message_ref.tool_calls.as_ref() else {
                continue;
            };
            if tool_calls.is_empty() {
                continue;
            }

            let required_ids = collect_required_ids(tool_calls, policy);

            if required_ids.is_empty() {
                dropped_indices.insert(idx);
                report.empty_tool_call_assistants =
                    report.empty_tool_call_assistants.saturating_add(1);
                continue;
            }

            active_pending = Some(PendingAssistant {
                message_index: idx,
                required_ids,
                satisfied_ids: HashSet::new(),
                linked_tool_message_indices: Vec::new(),
            });
            continue;
        }

        if message_ref.role == "tool" {
            dropped_indices.insert(idx);
            report.orphan_tools = report.orphan_tools.saturating_add(1);
        }
    }

    if let Some(pending) = active_pending.take() {
        drop_pending_assistant(pending, &mut report, &mut dropped_indices);
    }

    if dropped_indices.is_empty() {
        return (kept, report);
    }

    let mut sanitized = Vec::with_capacity(kept.len().saturating_sub(dropped_indices.len()));
    for (idx, message) in kept.into_iter().enumerate() {
        if !dropped_indices.contains(&idx) {
            sanitized.push(message);
        }
    }
    (sanitized, report)
}
