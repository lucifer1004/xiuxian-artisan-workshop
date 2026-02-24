#[derive(Debug, Clone, Copy)]
pub(crate) struct PermissionHints<'a> {
    pub(crate) control_command_hint: &'a str,
    pub(crate) slash_command_hint: &'a str,
}

fn permission_lines(title: &str, reason: &str, command: &str, sender: &str, hint: &str) -> String {
    [
        title.to_string(),
        format!("- `reason`: `{reason}`"),
        format!("- `command`: `{command}`"),
        format!("- `sender`: `{sender}`"),
        format!("- `hint`: {hint}"),
    ]
    .join("\n")
}

pub(crate) fn format_control_command_admin_required(
    command: &str,
    sender: &str,
    hints: PermissionHints<'_>,
) -> String {
    permission_lines(
        "## Control Command Permission Denied",
        "admin_required",
        command,
        sender,
        hints.control_command_hint,
    )
}

pub(crate) fn format_slash_command_permission_required(
    command: &str,
    sender: &str,
    hints: PermissionHints<'_>,
) -> String {
    permission_lines(
        "## Slash Command Permission Denied",
        "slash_permission_required",
        command,
        sender,
        hints.slash_command_hint,
    )
}
