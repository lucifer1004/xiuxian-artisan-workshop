//! Discord channel adapter with shared control-command authorization policy.

mod auth;
mod constructors;
mod policy;
mod policy_builders;
mod recipient_admin;
mod state;
mod trait_impl;

pub use policy::{DiscordCommandAdminRule, DiscordControlCommandPolicy, DiscordSlashCommandPolicy};
pub use policy_builders::build_discord_command_admin_rule;
pub use state::DiscordChannel;
