use std::collections::VecDeque;
use std::sync::{Arc, Mutex, OnceLock, mpsc};
use std::time::Duration;

use super::super::super::super::preprocess::PreparedVisionImage;
use super::super::super::util::internal_error;
use super::super::env::{parse_env_u64, parse_env_usize};
use crate::llm::error::LlmResult;

use super::core::DeepseekEngine;

const DEFAULT_BATCH_WINDOW_MS: u64 = 50;
const DEFAULT_BATCH_MAX_SIZE: usize = 8;
const DEFAULT_WAIT_TIMEOUT_MS: u64 = 30_000;

static BATCH_LANE: OnceLock<Mutex<BatchLaneState>> = OnceLock::new();

struct BatchLaneState {
    queue: VecDeque<BatchTask>,
    draining: bool,
}

struct BatchTask {
    engine: Arc<DeepseekEngine>,
    prepared: PreparedVisionImage,
    result_tx: mpsc::Sender<LlmResult<Option<String>>>,
}

impl BatchLaneState {
    fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            draining: false,
        }
    }
}

pub(super) fn infer_with_batch_lane(
    engine: Arc<DeepseekEngine>,
    prepared: &PreparedVisionImage,
) -> LlmResult<Option<String>> {
    let wait_timeout = lane_wait_timeout();
    let (result_tx, result_rx) = mpsc::channel();
    let task = BatchTask {
        engine,
        prepared: prepared.clone(),
        result_tx,
    };

    let became_leader = enqueue(task);
    if became_leader {
        drain_lane();
    }

    match result_rx.recv_timeout(wait_timeout) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => Err(internal_error(format!(
            "deepseek OCR batch lane wait timed out after {}ms",
            wait_timeout.as_millis()
        ))),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(internal_error(
            "deepseek OCR batch lane result channel disconnected unexpectedly",
        )),
    }
}

fn enqueue(task: BatchTask) -> bool {
    let lane = BATCH_LANE.get_or_init(|| Mutex::new(BatchLaneState::new()));
    let mut guard = lane
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.queue.push_back(task);
    if guard.draining {
        false
    } else {
        guard.draining = true;
        true
    }
}

fn drain_lane() {
    let batch_window = lane_batch_window();
    let max_batch_size = lane_batch_max_size();
    if !batch_window.is_zero() {
        std::thread::sleep(batch_window);
    }

    loop {
        let batch = take_next_batch(max_batch_size);
        if batch.is_empty() {
            mark_draining_finished();
            return;
        }
        for task in batch {
            let result = task.engine.infer_markdown(&task.prepared);
            let _ = task.result_tx.send(result);
        }
    }
}

fn take_next_batch(max_batch_size: usize) -> Vec<BatchTask> {
    let lane = BATCH_LANE.get_or_init(|| Mutex::new(BatchLaneState::new()));
    let mut guard = lane
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let to_take = guard.queue.len().min(max_batch_size.max(1));
    let mut batch = Vec::with_capacity(to_take);
    for _ in 0..to_take {
        if let Some(task) = guard.queue.pop_front() {
            batch.push(task);
        }
    }
    batch
}

fn mark_draining_finished() {
    let lane = BATCH_LANE.get_or_init(|| Mutex::new(BatchLaneState::new()));
    let mut guard = lane
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.draining = false;
}

fn lane_batch_window() -> Duration {
    Duration::from_millis(
        parse_env_u64("XIUXIAN_VISION_OCR_BATCH_WINDOW_MS").unwrap_or(DEFAULT_BATCH_WINDOW_MS),
    )
}

fn lane_batch_max_size() -> usize {
    parse_env_usize("XIUXIAN_VISION_OCR_BATCH_MAX_SIZE")
        .unwrap_or(DEFAULT_BATCH_MAX_SIZE)
        .max(1)
}

fn lane_wait_timeout() -> Duration {
    Duration::from_millis(
        parse_env_u64("XIUXIAN_VISION_OCR_INFLIGHT_WAIT_TIMEOUT_MS")
            .unwrap_or(DEFAULT_WAIT_TIMEOUT_MS)
            .max(1),
    )
}
