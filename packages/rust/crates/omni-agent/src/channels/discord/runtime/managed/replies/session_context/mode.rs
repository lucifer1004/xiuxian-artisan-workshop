use crate::agent::SessionContextMode;

pub(super) fn format_context_mode(mode: SessionContextMode) -> &'static str {
    match mode {
        SessionContextMode::Bounded => "bounded",
        SessionContextMode::Unbounded => "unbounded",
    }
}
