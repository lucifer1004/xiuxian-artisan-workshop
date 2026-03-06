use std::sync::Arc;

use crate::channels::traits::{Channel, ChannelMessage};

use super::super::super::super::parsing::CommandOutputFormat;
use super::super::super::super::replies::{format_slash_help, format_slash_help_json};
use super::super::super::events::{
    EVENT_DISCORD_COMMAND_SLASH_HELP_JSON_REPLIED, EVENT_DISCORD_COMMAND_SLASH_HELP_REPLIED,
};
use super::super::super::send::send_response;

pub(in super::super) async fn handle_help(
    channel: &Arc<dyn Channel>,
    msg: &ChannelMessage,
    format: CommandOutputFormat,
) {
    let (event, response) = if format.is_json() {
        (
            EVENT_DISCORD_COMMAND_SLASH_HELP_JSON_REPLIED,
            format_slash_help_json(),
        )
    } else {
        (
            EVENT_DISCORD_COMMAND_SLASH_HELP_REPLIED,
            format_slash_help(),
        )
    };
    send_response(channel, &msg.recipient, response, msg, event).await;
}
