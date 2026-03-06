use super::{DEFAULT_PULSE_CHANNEL, PulseEmitter, SwarmEvent};
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;

mod publish;

const DEFAULT_EVENT_QUEUE_CAPACITY: usize = 2048;

/// Non-blocking pulse emitter backed by Valkey Pub/Sub.
///
/// Emission path is always asynchronous:
/// - caller serializes + enqueue (`try_send`)
/// - dedicated background task publishes to Valkey
#[derive(Debug)]
pub struct ValkeyPulseEmitter {
    channel: Arc<str>,
    queue_tx: mpsc::Sender<Arc<str>>,
    sample_counter: AtomicU64,
    dropped_events: AtomicU64,
}

impl ValkeyPulseEmitter {
    /// Creates a pulse emitter that publishes to [`DEFAULT_PULSE_CHANNEL`].
    #[must_use]
    pub fn new(redis_url: String) -> Self {
        Self::with_channel(redis_url, DEFAULT_PULSE_CHANNEL.to_string())
    }

    /// Creates a pulse emitter with explicit channel name.
    #[must_use]
    pub fn with_channel(redis_url: String, channel: String) -> Self {
        let (queue_tx, queue_rx) = mpsc::channel(DEFAULT_EVENT_QUEUE_CAPACITY);
        let redis_url = Arc::<str>::from(redis_url);
        let channel = Arc::<str>::from(channel);
        std::mem::drop(tokio::spawn(publish::run_publish_loop(
            redis_url,
            Arc::clone(&channel),
            queue_rx,
        )));
        Self {
            channel,
            queue_tx,
            sample_counter: AtomicU64::new(0),
            dropped_events: AtomicU64::new(0),
        }
    }

    /// Returns total number of events dropped by local backpressure sampling/full queue.
    #[must_use]
    pub fn dropped_events(&self) -> u64 {
        self.dropped_events.load(Ordering::Relaxed)
    }

    /// Returns target Pub/Sub channel name.
    #[must_use]
    pub fn channel(&self) -> &str {
        &self.channel
    }

    fn should_sample_event(&self) -> bool {
        let sample_rate = self.current_sample_rate();
        if sample_rate <= 1 {
            return false;
        }
        let slot = self.sample_counter.fetch_add(1, Ordering::Relaxed);
        !slot.is_multiple_of(sample_rate)
    }

    fn current_sample_rate(&self) -> u64 {
        let free_capacity = self.queue_tx.capacity();
        if free_capacity < 64 {
            8
        } else if free_capacity < 128 {
            4
        } else if free_capacity < 256 {
            2
        } else {
            1
        }
    }
}

#[async_trait]
impl PulseEmitter for ValkeyPulseEmitter {
    async fn emit_pulse(&self, event: SwarmEvent) -> Result<(), String> {
        if self.should_sample_event() {
            return Ok(());
        }
        let payload = serde_json::to_string(&event).map_err(|error| error.to_string())?;
        match self.queue_tx.try_send(Arc::<str>::from(payload)) {
            Ok(()) => Ok(()),
            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                self.dropped_events.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                Err("swarm pulse emitter is closed".to_string())
            }
        }
    }
}
