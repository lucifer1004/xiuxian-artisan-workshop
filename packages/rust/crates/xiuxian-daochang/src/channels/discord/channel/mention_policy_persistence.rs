use std::fs;
use std::path::Path;

use anyhow::Context;
use toml::{Table, Value};

pub(super) fn persist_recipient_require_mention_to_user_settings(
    user_settings_path: &Path,
    recipient: &str,
    require_mention: Option<bool>,
) -> anyhow::Result<()> {
    let normalized_recipient = recipient.trim();
    if normalized_recipient.is_empty() {
        return Err(anyhow::anyhow!(
            "recipient-scoped mention policy persistence requires non-empty recipient"
        ));
    }

    let mut root = load_settings_toml(user_settings_path)?;
    let Some(root_table) = root.as_table_mut() else {
        return Err(anyhow::anyhow!(
            "invalid user settings toml: root must be a table"
        ));
    };

    let changed = apply_recipient_override(root_table, normalized_recipient, require_mention);
    if !changed {
        return Ok(());
    }

    if let Some(parent) = user_settings_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create user settings parent dir: {}",
                parent.display()
            )
        })?;
    }
    let serialized = toml::to_string_pretty(&root)
        .context("failed to serialize user settings toml for discord mention persistence")?;
    fs::write(user_settings_path, serialized).with_context(|| {
        format!(
            "failed to write user settings toml: {}",
            user_settings_path.display()
        )
    })?;
    Ok(())
}

fn apply_recipient_override(
    root_table: &mut Table,
    recipient: &str,
    require_mention: Option<bool>,
) -> bool {
    if let Some(value) = require_mention {
        let discord = ensure_child_table(root_table, "discord");
        let channels = ensure_child_table(discord, "channels");
        let channel = ensure_child_table(channels, recipient);
        channel.insert("require_mention".to_string(), Value::Boolean(value));
        return true;
    }

    let Some(discord) = root_table.get_mut("discord").and_then(Value::as_table_mut) else {
        return false;
    };
    let Some(channels) = discord.get_mut("channels").and_then(Value::as_table_mut) else {
        return false;
    };
    let Some(channel_value) = channels.get_mut(recipient) else {
        return false;
    };
    let Some(channel_table) = channel_value.as_table_mut() else {
        return false;
    };
    let changed = channel_table.remove("require_mention").is_some();
    if changed && channel_table.is_empty() {
        channels.remove(recipient);
    }
    if channels.is_empty() {
        discord.remove("channels");
    }
    if discord.is_empty() {
        root_table.remove("discord");
    }
    changed
}

fn ensure_child_table<'a>(parent: &'a mut Table, key: &str) -> &'a mut Table {
    let value = parent
        .entry(key.to_string())
        .or_insert_with(|| Value::Table(Table::new()));
    if !value.is_table() {
        *value = Value::Table(Table::new());
    }
    if let Value::Table(table) = value {
        table
    } else {
        unreachable!("table value should be initialized");
    }
}

fn load_settings_toml(path: &Path) -> anyhow::Result<Value> {
    if !path.exists() {
        return Ok(Value::Table(Table::new()));
    }
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read user settings toml: {}", path.display()))?;
    if raw.trim().is_empty() {
        return Ok(Value::Table(Table::new()));
    }
    let parsed = toml::from_str::<Value>(&raw)
        .with_context(|| format!("failed to parse user settings toml: {}", path.display()))?;
    Ok(parsed)
}
