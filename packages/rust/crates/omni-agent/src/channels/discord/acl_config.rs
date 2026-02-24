use std::collections::HashMap;

use crate::config::{
    DiscordAclAllowSettings, DiscordAclControlSettings, DiscordAclPrincipalSettings,
    DiscordAclSettings, DiscordAclSlashSettings, RuntimeSettings,
};

use super::channel::{DiscordCommandAdminRule, build_discord_command_admin_rule};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiscordAclOverrides {
    pub allowed_users: Vec<String>,
    pub allowed_guilds: Vec<String>,
    pub admin_users: Option<Vec<String>>,
    pub control_command_allow_from: Option<Vec<String>>,
    pub control_command_rules: Vec<DiscordCommandAdminRule>,
    pub slash_command_allow_from: Option<Vec<String>>,
    pub slash_session_status_allow_from: Option<Vec<String>>,
    pub slash_session_budget_allow_from: Option<Vec<String>>,
    pub slash_session_memory_allow_from: Option<Vec<String>>,
    pub slash_session_feedback_allow_from: Option<Vec<String>>,
    pub slash_job_allow_from: Option<Vec<String>>,
    pub slash_jobs_allow_from: Option<Vec<String>>,
    pub slash_bg_allow_from: Option<Vec<String>>,
}

/// Build Discord runtime ACL overrides from settings.
///
/// # Errors
/// Returns an error when ACL command-rule parsing fails.
pub fn build_discord_acl_overrides(
    settings: &RuntimeSettings,
) -> anyhow::Result<DiscordAclOverrides> {
    let acl = &settings.discord.acl;
    let role_aliases = normalize_role_aliases(acl);

    let allowed_users = acl
        .allow
        .as_ref()
        .and_then(|allow| principal_list_from_allow(allow, &role_aliases))
        .unwrap_or_default();
    let allowed_guilds = acl
        .allow
        .as_ref()
        .and_then(guilds_list_from_allow)
        .unwrap_or_default();
    let admin_users = acl
        .admin
        .as_ref()
        .and_then(|principal| collect_principals(principal, &role_aliases));
    let control_command_allow_from = acl
        .control
        .as_ref()
        .and_then(|control| control.allow_from.as_ref())
        .and_then(|allow_from| collect_principals(allow_from, &role_aliases));
    let control_command_rules = acl
        .control
        .as_ref()
        .map(|control| control_rules(control, &role_aliases))
        .transpose()?
        .unwrap_or_default();

    let (
        slash_command_allow_from,
        slash_session_status_allow_from,
        slash_session_budget_allow_from,
        slash_session_memory_allow_from,
        slash_session_feedback_allow_from,
        slash_job_status_allow_from,
        slash_jobs_summary_allow_from,
        slash_bg_allow_from,
    ) = slash_overrides(acl.slash.as_ref(), &role_aliases);

    Ok(DiscordAclOverrides {
        allowed_users,
        allowed_guilds,
        admin_users,
        control_command_allow_from,
        control_command_rules,
        slash_command_allow_from,
        slash_session_status_allow_from,
        slash_session_budget_allow_from,
        slash_session_memory_allow_from,
        slash_session_feedback_allow_from,
        slash_job_allow_from: slash_job_status_allow_from,
        slash_jobs_allow_from: slash_jobs_summary_allow_from,
        slash_bg_allow_from,
    })
}

fn normalize_role_aliases(acl: &DiscordAclSettings) -> HashMap<String, String> {
    let mut normalized = HashMap::new();
    let Some(role_aliases) = acl.role_aliases.as_ref() else {
        return normalized;
    };
    for (alias, raw_role_value) in role_aliases {
        let key = alias.trim().to_ascii_lowercase();
        if key.is_empty() {
            continue;
        }
        let Some(role_id) = parse_role_id(raw_role_value) else {
            tracing::warn!(
                alias = %key,
                value = %raw_role_value,
                "discord acl role_aliases entry ignored: invalid role id"
            );
            continue;
        };
        normalized.insert(key, format!("role:{role_id}"));
    }
    normalized
}

fn parse_role_id(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(rest) = trimmed.strip_prefix("role:") {
        return parse_role_id(rest);
    }
    if let Some(rest) = trimmed
        .strip_prefix("<@&")
        .and_then(|value| value.strip_suffix('>'))
    {
        return parse_role_id(rest);
    }
    if trimmed.chars().all(|ch| ch.is_ascii_digit()) {
        return Some(trimmed.to_string());
    }
    None
}

fn resolve_role_principal(
    raw_role: &str,
    role_aliases: &HashMap<String, String>,
) -> Option<String> {
    let role = raw_role.trim();
    if role.is_empty() {
        return None;
    }
    if let Some(role_id) = parse_role_id(role) {
        return Some(format!("role:{role_id}"));
    }

    let alias_key = role
        .strip_prefix("role:")
        .map_or(role, str::trim)
        .to_ascii_lowercase();
    if let Some(role_principal) = role_aliases.get(&alias_key) {
        return Some(role_principal.clone());
    }

    tracing::warn!(
        role = %role,
        "discord acl role entry ignored: role id or alias not found"
    );
    None
}

fn resolve_principal_entry(
    raw_entry: &str,
    role_aliases: &HashMap<String, String>,
) -> Option<String> {
    let entry = raw_entry.trim();
    if entry.is_empty() {
        return None;
    }
    if entry.starts_with("role:") || entry.starts_with("<@&") {
        return resolve_role_principal(entry, role_aliases);
    }
    Some(entry.to_string())
}

fn collect_principals(
    principal: &DiscordAclPrincipalSettings,
    role_aliases: &HashMap<String, String>,
) -> Option<Vec<String>> {
    let configured = principal.users.is_some() || principal.roles.is_some();
    if !configured {
        return None;
    }

    let mut resolved: Vec<String> = principal
        .users
        .clone()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|entry| resolve_principal_entry(&entry, role_aliases))
        .collect();
    resolved.extend(
        principal
            .roles
            .clone()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|entry| resolve_role_principal(&entry, role_aliases)),
    );

    Some(resolved)
}

fn principal_list_from_allow(
    allow: &DiscordAclAllowSettings,
    role_aliases: &HashMap<String, String>,
) -> Option<Vec<String>> {
    let principal = DiscordAclPrincipalSettings {
        users: allow.users.clone(),
        roles: allow.roles.clone(),
    };
    collect_principals(&principal, role_aliases)
}

fn guilds_list_from_allow(allow: &DiscordAclAllowSettings) -> Option<Vec<String>> {
    let guilds = allow.guilds.as_ref()?;
    Some(
        guilds
            .iter()
            .map(|entry| entry.trim().to_string())
            .filter(|entry| !entry.is_empty())
            .collect(),
    )
}

fn control_rules(
    control: &DiscordAclControlSettings,
    role_aliases: &HashMap<String, String>,
) -> anyhow::Result<Vec<DiscordCommandAdminRule>> {
    let Some(rules) = control.rules.as_ref() else {
        return Ok(Vec::new());
    };
    let mut parsed_rules = Vec::new();
    for (index, rule) in rules.iter().enumerate() {
        let commands: Vec<String> = rule
            .commands
            .iter()
            .map(|command| command.trim().to_string())
            .filter(|command| !command.is_empty())
            .collect();
        if commands.is_empty() {
            tracing::warn!("discord acl control rule ignored: empty commands");
            continue;
        }
        let Some(principals) = collect_principals(&rule.allow, role_aliases) else {
            tracing::warn!(
                commands = %commands.join(","),
                "discord acl control rule ignored: no allow principals configured"
            );
            continue;
        };
        if principals.is_empty() {
            tracing::warn!(
                commands = %commands.join(","),
                "discord acl control rule ignored: allow principals resolved to empty set"
            );
            continue;
        }
        let parsed_rule =
            build_discord_command_admin_rule(commands, principals).map_err(|error| {
                anyhow::anyhow!("discord.acl.control.rules[{index}].commands: {error}")
            })?;
        parsed_rules.push(parsed_rule);
    }
    Ok(parsed_rules)
}

#[allow(clippy::type_complexity)]
fn slash_overrides(
    slash: Option<&DiscordAclSlashSettings>,
    role_aliases: &HashMap<String, String>,
) -> (
    Option<Vec<String>>,
    Option<Vec<String>>,
    Option<Vec<String>>,
    Option<Vec<String>>,
    Option<Vec<String>>,
    Option<Vec<String>>,
    Option<Vec<String>>,
    Option<Vec<String>>,
) {
    let Some(slash) = slash else {
        return (None, None, None, None, None, None, None, None);
    };

    (
        slash
            .global
            .as_ref()
            .and_then(|principal| collect_principals(principal, role_aliases)),
        slash
            .session_status
            .as_ref()
            .and_then(|principal| collect_principals(principal, role_aliases)),
        slash
            .session_budget
            .as_ref()
            .and_then(|principal| collect_principals(principal, role_aliases)),
        slash
            .session_memory
            .as_ref()
            .and_then(|principal| collect_principals(principal, role_aliases)),
        slash
            .session_feedback
            .as_ref()
            .and_then(|principal| collect_principals(principal, role_aliases)),
        slash
            .job_status
            .as_ref()
            .and_then(|principal| collect_principals(principal, role_aliases)),
        slash
            .jobs_summary
            .as_ref()
            .and_then(|principal| collect_principals(principal, role_aliases)),
        slash
            .background_submit
            .as_ref()
            .and_then(|principal| collect_principals(principal, role_aliases)),
    )
}
