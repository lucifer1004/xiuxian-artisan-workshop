//! Unix Domain Socket server for receiving events from Python Agent
//!
//! Listens on /tmp/omni-omega.sock for JSON events in omni-events format:
//! {"source": "omega", "topic": "omega/mission/start", "payload": {...}, "timestamp": "..."}

use anyhow::Context;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Received event from Python
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SocketEvent {
    /// Event source identifier.
    pub source: String,
    /// Event topic used for routing.
    pub topic: String,
    /// Event payload content.
    pub payload: Value,
    /// Event timestamp in string form.
    pub timestamp: String,
}

/// Event callback for received events
pub type EventCallback = Box<dyn Fn(SocketEvent) + Send + 'static>;

/// Unix Domain Socket client for receiving events from an external socket.
#[derive(Debug, Clone)]
pub struct SocketClient;

impl SocketClient {
    /// Connect to an existing Unix socket and forward events to a channel.
    ///
    /// # Errors
    ///
    /// Returns an error when the Unix socket cannot be connected.
    pub fn connect(
        socket_path: &str,
        tx: std::sync::mpsc::Sender<SocketEvent>,
    ) -> anyhow::Result<thread::JoinHandle<()>> {
        let stream = UnixStream::connect(socket_path)
            .with_context(|| format!("failed to connect to socket at {socket_path}"))?;
        let socket_path = socket_path.to_string();

        let handle = thread::spawn(move || {
            let mut reader = BufReader::new(stream);
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => {
                        info!("Socket client reached EOF on {socket_path}");
                        break;
                    }
                    Ok(_) => {
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }
                        match serde_json::from_str::<SocketEvent>(line) {
                            Ok(event) => {
                                if tx.send(event).is_err() {
                                    warn!("Socket client receiver dropped for {socket_path}");
                                    break;
                                }
                            }
                            Err(error) => {
                                warn!("Failed to parse socket event from {socket_path}: {error}");
                            }
                        }
                    }
                    Err(error) => {
                        error!("Socket client read error on {socket_path}: {error}");
                        break;
                    }
                }
            }
        });

        Ok(handle)
    }
}

/// Unix Domain Socket server for receiving Python events
#[derive(Clone)]
pub struct SocketServer {
    socket_path: String,
    running: Arc<AtomicBool>,
    event_callback: Arc<Mutex<Option<EventCallback>>>,
}

impl fmt::Debug for SocketServer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SocketServer")
            .field("socket_path", &self.socket_path)
            .field("running", &self.running.load(Ordering::SeqCst))
            .finish_non_exhaustive()
    }
}

impl SocketServer {
    /// Create a new socket server
    #[must_use]
    pub fn new(socket_path: &str) -> Self {
        Self {
            socket_path: socket_path.to_string(),
            running: Arc::new(AtomicBool::new(false)),
            event_callback: Arc::new(Mutex::new(None)),
        }
    }

    /// Set callback for received events
    pub fn set_event_callback(&self, callback: EventCallback) {
        if let Ok(mut cb) = self.event_callback.lock() {
            *cb = Some(callback);
        }
    }

    /// Start the server in a background thread
    ///
    /// # Errors
    ///
    /// Returns an error when the socket path cannot be bound.
    pub fn start(&self) -> anyhow::Result<thread::JoinHandle<()>> {
        let socket_path = Path::new(&self.socket_path);

        // Remove existing socket file
        if socket_path.exists() {
            std::fs::remove_file(socket_path)?;
        }

        // Create listener
        let listener = UnixListener::bind(socket_path)?;
        let listener_clone = listener.try_clone()?;
        listener.set_nonblocking(true)?;

        self.running.store(true, Ordering::SeqCst);

        let running = self.running.clone();
        let callback = self.event_callback.clone();

        // Start background thread
        let handle = thread::spawn(move || {
            Self::run_loop(&listener_clone, &running, &callback);
        });

        info!("Socket server started on {}", self.socket_path);
        Ok(handle)
    }

    /// Main server loop
    fn run_loop(
        listener: &UnixListener,
        running: &Arc<AtomicBool>,
        callback: &Arc<Mutex<Option<EventCallback>>>,
    ) {
        let mut connections = Vec::new();

        while running.load(Ordering::SeqCst) {
            // Check for new connections
            match listener.accept() {
                Ok((stream, _addr)) => {
                    stream.set_nonblocking(false).ok();
                    connections.push(BufReader::new(stream));
                    info!("New connection from Python agent");
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No pending connections, continue
                }
                Err(e) => {
                    error!("Accept error: {e}");
                }
            }

            // Process existing connections
            let mut dead_connections = Vec::new();
            for (i, conn) in connections.iter_mut().enumerate() {
                let mut line = String::new();
                match conn.read_line(&mut line) {
                    Ok(0) => {
                        // Connection closed
                        dead_connections.push(i);
                    }
                    Ok(_) => {
                        // Parse event
                        let line = line.trim();
                        if !line.is_empty() {
                            if let Ok(event) = serde_json::from_str::<SocketEvent>(line) {
                                info!("Received event: {} from {}", event.topic, event.source);
                                if let Ok(cb) = callback.lock()
                                    && let Some(ref callback) = *cb
                                {
                                    callback(event.clone());
                                }
                            } else {
                                warn!("Failed to parse event: {line}");
                            }
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // No data available
                    }
                    Err(e) => {
                        error!("Read error: {e}");
                        dead_connections.push(i);
                    }
                }
            }

            // Remove dead connections
            for i in dead_connections.into_iter().rev() {
                connections.swap_remove(i);
            }

            // Sleep briefly to avoid busy loop
            thread::sleep(Duration::from_millis(10));
        }

        info!("Socket server stopped");
    }

    /// Stop the server
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);

        // Clean up socket file
        let socket_path = Path::new(&self.socket_path);
        if socket_path.exists() {
            std::fs::remove_file(socket_path).ok();
        }

        info!("Socket server stopped and cleaned up");
    }

    /// Check if running
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

/// Send an event through Unix socket (for testing)
///
/// # Errors
///
/// Returns an error when the socket cannot be opened or the event cannot be serialized.
pub fn send_event(socket_path: &str, event: &SocketEvent) -> anyhow::Result<()> {
    let mut stream = UnixStream::connect(socket_path)?;

    let json = serde_json::to_string(event)?;
    stream.write_all(json.as_bytes())?;
    stream.write_all(b"\n")?;

    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/socket.rs"]
mod tests;
