use serde::Deserialize;
use serenity::all::{
    ChannelId, CommandDataOption, CommandDataOptionValue, GuildId, Interaction, MessageId, RoleId,
    UserId,
};

#[derive(Debug)]
pub(super) struct DiscordIngressPayload {
    pub(super) event_id: String,
    pub(super) content: String,
    pub(super) channel_id: ChannelId,
    pub(super) guild_id: Option<GuildId>,
    pub(super) author_id: UserId,
    pub(super) author_username: Option<String>,
    pub(super) author_role_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct DiscordIngressMessagePayload {
    pub(super) id: MessageId,
    pub(super) content: String,
    pub(super) channel_id: ChannelId,
    #[serde(default)]
    pub(super) guild_id: Option<GuildId>,
    pub(super) author: DiscordIngressAuthorPayload,
    #[serde(default)]
    pub(super) member: Option<DiscordIngressMemberPayload>,
}

#[derive(Debug, Deserialize)]
pub(super) struct DiscordIngressAuthorPayload {
    pub(super) id: UserId,
    #[serde(default)]
    pub(super) username: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct DiscordIngressMemberPayload {
    #[serde(default)]
    pub(super) roles: Vec<RoleId>,
}

pub(super) fn parse_discord_ingress_payload(
    event: &serde_json::Value,
) -> Option<DiscordIngressPayload> {
    parse_discord_ingress_message(event)
        .map(|message| DiscordIngressPayload {
            event_id: message.id.to_string(),
            content: message.content,
            channel_id: message.channel_id,
            guild_id: message.guild_id,
            author_id: message.author.id,
            author_username: message.author.username,
            author_role_ids: message
                .member
                .as_ref()
                .map(|member| {
                    member
                        .roles
                        .iter()
                        .map(|role| role.get().to_string())
                        .collect()
                })
                .unwrap_or_default(),
        })
        .or_else(|| parse_discord_command_interaction(event))
}

fn parse_discord_ingress_message(
    event: &serde_json::Value,
) -> Option<DiscordIngressMessagePayload> {
    serde_json::from_value(event.clone()).ok()
}

fn parse_discord_command_interaction(event: &serde_json::Value) -> Option<DiscordIngressPayload> {
    let interaction: Interaction = serde_json::from_value(event.clone()).ok()?;
    let command = interaction.command()?;
    let content = render_interaction_command(&command.data.name, &command.data.options)?;
    if content.trim().is_empty() {
        return None;
    }

    let username = command.user.name.trim().to_string();
    let author_role_ids = command
        .member
        .as_ref()
        .map(|member| {
            member
                .roles
                .iter()
                .map(|role| role.get().to_string())
                .collect()
        })
        .unwrap_or_default();
    Some(DiscordIngressPayload {
        event_id: command.id.to_string(),
        content,
        channel_id: command.channel_id,
        guild_id: command.guild_id,
        author_id: command.user.id,
        author_username: (!username.is_empty()).then_some(username),
        author_role_ids,
    })
}

fn render_interaction_command(command_name: &str, options: &[CommandDataOption]) -> Option<String> {
    let command_name = command_name.trim();
    if command_name.is_empty() {
        return None;
    }

    let mut tokens = vec![format!("/{command_name}")];
    let mut args = Vec::new();
    for option in options {
        flatten_option(option, &mut args);
    }
    if !args.is_empty() {
        tokens.push(args.join(" "));
    }

    Some(tokens.join(" "))
}

fn flatten_option(option: &CommandDataOption, args: &mut Vec<String>) {
    match &option.value {
        CommandDataOptionValue::SubCommand(options)
        | CommandDataOptionValue::SubCommandGroup(options) => {
            let name = option.name.trim();
            if !name.is_empty() {
                args.push(name.to_string());
            }
            for child in options {
                flatten_option(child, args);
            }
        }
        CommandDataOptionValue::Autocomplete { value, .. }
        | CommandDataOptionValue::String(value) => push_non_empty(value, args),
        CommandDataOptionValue::Boolean(value) => args.push(value.to_string()),
        CommandDataOptionValue::Integer(value) => args.push(value.to_string()),
        CommandDataOptionValue::Number(value) => args.push(value.to_string()),
        CommandDataOptionValue::Attachment(value) => args.push(value.get().to_string()),
        CommandDataOptionValue::Channel(value) => args.push(value.get().to_string()),
        CommandDataOptionValue::Mentionable(value) => args.push(value.get().to_string()),
        CommandDataOptionValue::Role(value) => args.push(value.get().to_string()),
        CommandDataOptionValue::User(value) => args.push(value.get().to_string()),
        _ => {}
    }
}

fn push_non_empty(value: &str, args: &mut Vec<String>) {
    let trimmed = value.trim();
    if !trimmed.is_empty() {
        args.push(trimmed.to_string());
    }
}
