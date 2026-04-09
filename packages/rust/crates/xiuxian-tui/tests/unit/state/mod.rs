//! Comprehensive tests for `xiuxian-tui` state management.

use std::{error::Error, io, path::PathBuf, time::Duration};
use tempfile::TempDir;

use super::{AppState, ExecutionState, LogWindow, PanelType, ReceivedEvent, TaskItem, TaskStatus};
use crate::socket::{SocketEvent, send_event};

type TestResult = Result<(), Box<dyn Error>>;

fn must_some<T>(value: Option<T>, context: &str) -> T {
    match value {
        Some(inner) => inner,
        None => panic!("{context}"),
    }
}

fn socket_fixture(name: &str) -> Result<(TempDir, PathBuf, String), io::Error> {
    let temp_dir = TempDir::new()?;
    let socket_path = temp_dir.path().join(name);
    let socket_str = socket_path.to_str().map(str::to_owned).ok_or_else(|| {
        io::Error::other(format!(
            "socket path should be valid UTF-8: {socket_path:?}"
        ))
    })?;
    Ok((temp_dir, socket_path, socket_str))
}

/// Test: Basic state creation
#[test]
fn test_state_creation() {
    let state = AppState::new("Test App".to_string());
    assert_eq!(state.title(), "Test App");
    assert!(!state.should_quit());
    assert!(state.app().is_some());
    assert!(!state.is_socket_running());
}

/// Test: Empty state creation
#[test]
fn test_empty_state() {
    let state = AppState::empty();
    assert_eq!(state.title(), "Omni TUI");
    assert!(state.app().is_none());
    assert!(!state.should_quit());
}

/// Test: Status message operations
#[test]
fn test_status_message() {
    let mut state = AppState::new("Test".to_string());
    assert_eq!(state.status_message(), None);

    state.set_status("Test message");
    assert_eq!(state.status_message(), Some("Test message"));
}

/// Test: Quit functionality
#[test]
fn test_quit() {
    let mut state = AppState::new("Test".to_string());
    assert!(!state.should_quit());

    state.quit();
    assert!(state.should_quit());
}

/// Test: Panel addition
#[test]
fn test_panel_addition() {
    let mut state = AppState::new("Test".to_string());
    assert_eq!(
        must_some(state.app(), "panel collection should exist")
            .panels()
            .len(),
        0
    );

    state.add_result("Test Panel", "Test Content");
    assert_eq!(
        must_some(state.app(), "panel collection should exist")
            .panels()
            .len(),
        1
    );
}

/// Test: Socket server integration
#[test]
fn test_socket_server_integration() -> TestResult {
    let (_temp_dir, socket_path, socket_str) = socket_fixture("integration.sock")?;

    let mut state = AppState::new("Test".to_string());
    assert!(!state.is_socket_running());

    state.start_socket_server(socket_str.as_str())?;
    assert!(state.is_socket_running());
    assert!(socket_path.exists());

    state.stop_socket_server();
    assert!(!state.is_socket_running());
    Ok(())
}

/// Test: Received events storage
#[test]
fn test_received_events_storage() {
    let state = AppState::new("Test".to_string());
    assert!(state.received_events().is_empty());
}

/// Test: Socket event handling
#[test]
fn test_socket_event_handling() -> TestResult {
    let (_temp_dir, _socket_path, socket_str) = socket_fixture("events.sock")?;

    let mut state = AppState::new("Test".to_string());
    state.start_socket_server(socket_str.as_str())?;

    let event = SocketEvent {
        source: "omega".to_string(),
        topic: "omega/mission/start".to_string(),
        payload: serde_json::json!({"goal": "test goal"}),
        timestamp: "2026-01-31T12:00:00Z".to_string(),
    };

    send_event(socket_str.as_str(), &event)?;

    std::thread::sleep(Duration::from_millis(100));

    state.stop_socket_server();

    let events = state.received_events();
    assert!(!events.is_empty());
    Ok(())
}

/// Test: Multiple mission events
#[test]
fn test_mission_events() -> TestResult {
    let (_temp_dir, _socket_path, socket_str) = socket_fixture("missions.sock")?;

    let mut state = AppState::new("Test".to_string());
    state.start_socket_server(socket_str.as_str())?;

    for (i, &(source, topic, _)) in [
        ("omega", "omega/mission/start", "Mission 1"),
        ("omega", "omega/semantic/scan", "Scanning..."),
        ("omega", "omega/mission/complete", "Done"),
    ]
    .iter()
    .enumerate()
    {
        let event = SocketEvent {
            source: source.to_string(),
            topic: topic.to_string(),
            payload: serde_json::json!({"index": i}),
            timestamp: "2026-01-31T12:00:00Z".to_string(),
        };

        send_event(socket_str.as_str(), &event)?;

        std::thread::sleep(Duration::from_millis(20));
    }

    std::thread::sleep(Duration::from_millis(200));

    state.stop_socket_server();

    let received = state.received_events();
    assert!(received.len() >= 3);
    Ok(())
}

/// Test: `AppState` Default implementation
#[test]
fn test_state_default() {
    let state = AppState::default();
    assert_eq!(state.title(), "Omni TUI");
    assert!(!state.should_quit());
}

/// Test: Panel type enum
#[test]
fn test_panel_types() {
    assert_eq!(PanelType::Result, PanelType::Result);
    assert_eq!(PanelType::Log, PanelType::Log);
    assert_eq!(PanelType::Error, PanelType::Error);
}

/// Test: `ReceivedEvent` clone and debug
#[test]
fn test_received_event_traits() {
    let event = ReceivedEvent {
        source: "test".to_string(),
        topic: "test/topic".to_string(),
        payload: serde_json::json!({"key": "value"}),
        timestamp: "2026-01-31T12:00:00Z".to_string(),
    };

    let cloned = event.clone();
    assert_eq!(cloned.source, event.source);

    let debug_str = format!("{event:?}");
    assert!(debug_str.contains("test"));
}

/// Test: Event processing with tick
#[test]
fn test_event_processing_tick() -> TestResult {
    let (_temp_dir, _socket_path, socket_str) = socket_fixture("tick.sock")?;

    let mut state = AppState::new("Test".to_string());
    state.start_socket_server(socket_str.as_str())?;

    let event = SocketEvent {
        source: "test".to_string(),
        topic: "test/event".to_string(),
        payload: serde_json::json!({"test": true}),
        timestamp: "2026-01-31T12:00:00Z".to_string(),
    };

    send_event(socket_str.as_str(), &event)?;

    std::thread::sleep(Duration::from_millis(100));

    state.on_tick();
    state.stop_socket_server();
    Ok(())
}

/// Test: Large number of events
#[test]
fn test_many_events() -> TestResult {
    let (_temp_dir, _socket_path, socket_str) = socket_fixture("many.sock")?;

    let mut state = AppState::new("Test".to_string());
    state.start_socket_server(socket_str.as_str())?;

    for i in 0..20 {
        let event = SocketEvent {
            source: "test".to_string(),
            topic: format!("test/event/{i}"),
            payload: serde_json::json!({"index": i}),
            timestamp: format!("2026-01-31T12:00:{i:02}Z"),
        };

        send_event(socket_str.as_str(), &event)?;
    }

    std::thread::sleep(Duration::from_millis(300));

    state.stop_socket_server();

    let events = state.received_events();
    assert!(
        events.len() >= 19,
        "Expected ~20 events, got {}",
        events.len()
    );
    Ok(())
}

/// Test: Stop server when not running
#[test]
fn test_stop_when_not_running() {
    let mut state = AppState::new("Test".to_string());
    state.stop_socket_server();
    assert!(!state.is_socket_running());
}

/// Test: Event with special characters
#[test]
fn test_special_characters() -> TestResult {
    let (_temp_dir, _socket_path, socket_str) = socket_fixture("special.sock")?;

    let mut state = AppState::new("Test".to_string());
    state.start_socket_server(socket_str.as_str())?;

    let event = SocketEvent {
        source: "test".to_string(),
        topic: "test/special".to_string(),
        payload: serde_json::json!({"text": "Hello 世界 🌍"}),
        timestamp: "2026-01-31T12:00:00Z".to_string(),
    };

    send_event(socket_str.as_str(), &event)?;

    std::thread::sleep(Duration::from_millis(100));

    state.stop_socket_server();

    let events = state.received_events();
    assert_eq!(events.len(), 1);
    assert!(
        must_some(
            events[0].payload["text"].as_str(),
            "special-character payload should be a string"
        )
        .contains("世界")
    );
    Ok(())
}

#[test]
fn test_task_item_creation() {
    let task = TaskItem::new(
        "t1".to_string(),
        "Test task".to_string(),
        "echo test".to_string(),
    );
    assert_eq!(task.id, "t1");
    assert_eq!(task.status, TaskStatus::Pending);
    assert_eq!(task.status_symbol(), "○");
}

#[test]
fn test_task_status_colors() {
    let mut pending = TaskItem::new("t1".to_string(), "Test".to_string(), "cmd".to_string());
    let mut running = TaskItem::new("t2".to_string(), "Test".to_string(), "cmd".to_string());
    let mut success = TaskItem::new("t3".to_string(), "Test".to_string(), "cmd".to_string());

    pending.status = TaskStatus::Pending;
    running.status = TaskStatus::Running;
    success.status = TaskStatus::Success;

    assert_ne!(pending.status_color(), running.status_color());
    assert_ne!(running.status_color(), success.status_color());
}

#[test]
fn test_execution_state() {
    let mut state = ExecutionState::new();
    assert!(state.tasks.is_empty());

    state.add_task(TaskItem::new(
        "t1".to_string(),
        "Task 1".to_string(),
        "cmd1".to_string(),
    ));
    state.add_task(TaskItem::new(
        "t2".to_string(),
        "Task 2".to_string(),
        "cmd2".to_string(),
    ));

    assert_eq!(state.tasks.len(), 2);
    assert!(state.find_task("t1").is_some());
    assert!(state.find_task("unknown").is_none());

    state.update_task_status("t1", TaskStatus::Running);
    let t1 = must_some(state.find_task("t1"), "task t1 should exist");
    assert_eq!(t1.status, TaskStatus::Running);
}

#[test]
fn test_log_window_bounded() {
    let mut window = LogWindow::new(5);
    for i in 0..10 {
        window.add_line("info", &format!("Line {i}"), "");
    }
    assert_eq!(window.len(), 5);
    assert!(window.get_lines_owned()[0].contains("Line 5"));
}
