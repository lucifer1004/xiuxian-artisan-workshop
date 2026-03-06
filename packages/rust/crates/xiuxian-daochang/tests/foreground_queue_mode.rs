//! Foreground queue mode defaults and parsing behavior.

use xiuxian_daochang::{DiscordRuntimeConfig, ForegroundQueueMode};

#[test]
fn foreground_queue_mode_defaults_to_queue() {
    assert_eq!(ForegroundQueueMode::default(), ForegroundQueueMode::Queue);
    assert!(!ForegroundQueueMode::default().should_interrupt_on_new_message());
}

#[test]
fn foreground_queue_mode_parse_still_accepts_explicit_interrupt() {
    assert_eq!(
        ForegroundQueueMode::parse("queue"),
        Some(ForegroundQueueMode::Queue)
    );
    assert_eq!(
        ForegroundQueueMode::parse("interrupt"),
        Some(ForegroundQueueMode::Interrupt)
    );
}

#[test]
fn discord_runtime_defaults_to_queue_mode() {
    assert_eq!(
        DiscordRuntimeConfig::default().foreground_queue_mode,
        ForegroundQueueMode::Queue
    );
}
