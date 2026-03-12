use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use base64::Engine;
use xiuxian_llm::llm::multimodal::Base64ImageSource;
use xiuxian_llm::llm::vision::{
    DeepseekRuntime, PreparedVisionImage, get_deepseek_runtime, infer_deepseek_ocr_truth,
};

use super::core::{
    OcrRequestAdmission, OcrTimeoutConfig, admit_deepseek_ocr_request_or_log,
    deepseek_ocr_max_dimension, deepseek_ocr_runtime_prewarmed, log_deepseek_ocr_runtime_status,
    mark_deepseek_ocr_runtime_prewarmed, resolve_deepseek_ocr_timeout,
};
use super::execution::{
    OcrExecutionOutcomeContext, execute_deepseek_ocr_blocking_task_for_runtime,
    handle_deepseek_ocr_execution_outcome,
};
use super::pipeline::{
    decode_deepseek_ocr_source_image_bytes, log_deepseek_ocr_truth_outcome,
    normalize_deepseek_ocr_truth_markdown, preprocess_deepseek_ocr_source_image,
};

const OCR_TRUTH_HEADER: &str =
    "[PHYSICAL_OCR_TRUTH]: The following is a high-fidelity Markdown reconstruction of the image.";
const OCR_TRUTH_FOOTER: &str =
    "[INSTRUCTION]: Use this truth to answer the user query and keep the answer grounded.";

pub(crate) fn build_ocr_truth_overlay_text(ocr_truth_markdown: &str) -> String {
    format!("{OCR_TRUTH_HEADER}\n\n{ocr_truth_markdown}\n\n{OCR_TRUTH_FOOTER}")
}

pub(crate) async fn infer_deepseek_ocr_truth_markdown(
    source: &Base64ImageSource,
) -> Option<String> {
    let request_started = Instant::now();
    let runtime = get_deepseek_runtime();
    if !log_deepseek_ocr_runtime_status(runtime.as_ref()) {
        return None;
    }

    let _process_guard = match admit_deepseek_ocr_request_or_log(runtime.as_ref()).await {
        OcrRequestAdmission::Allowed { guard } => guard,
        OcrRequestAdmission::Rejected => return None,
    };
    let max_dimension = deepseek_ocr_max_dimension();
    let decode_started = Instant::now();
    let image_bytes = decode_deepseek_ocr_source_image_bytes(source)?;
    let image_decode_ms = decode_started.elapsed().as_millis();
    let mut image_preprocess_ms = 0;
    if !deepseek_ocr_runtime_prewarmed() {
        tracing::info!(
            event = "agent.llm.vision.deepseek.ocr.prewarm_deferred",
            "DeepSeek OCR request path is running without blocking prewarm"
        );
    }

    let timeout_config = resolve_deepseek_ocr_timeout();
    let refine_started = Instant::now();

    let preprocess_started = Instant::now();
    let prepared = preprocess_deepseek_ocr_source_image(image_bytes, max_dimension)?;
    image_preprocess_ms = preprocess_started.elapsed().as_millis();

    let markdown = run_refinement_task(
        Arc::clone(&runtime),
        prepared,
        timeout_config,
        request_started,
        max_dimension,
    )
    .await?;

    let refine_ms = refine_started.elapsed().as_millis();
    mark_deepseek_ocr_runtime_prewarmed();

    let markdown = normalize_deepseek_ocr_truth_markdown(markdown);
    log_deepseek_ocr_truth_outcome(
        markdown.as_deref(),
        image_decode_ms,
        image_preprocess_ms,
        refine_ms,
        request_started.elapsed().as_millis(),
    );
    markdown
}

async fn run_refinement_task(
    runtime: Arc<DeepseekRuntime>,
    prepared: PreparedVisionImage,
    timeout_config: OcrTimeoutConfig,
    request_started: Instant,
    max_dimension: u32,
) -> Option<Option<String>> {
    let prepared_width = prepared.width;
    let prepared_height = prepared.height;

    // Create a stop signal that we can set if the outer task times out
    let stop_signal = Arc::new(AtomicBool::new(false));
    let stop_signal_for_task = Arc::clone(&stop_signal);

    handle_deepseek_ocr_execution_outcome(
        execute_deepseek_ocr_blocking_task_for_runtime(
            runtime.as_ref(),
            timeout_config.duration,
            stop_signal,
            move |signal| {
                Ok(infer_deepseek_ocr_truth(
                    runtime.as_ref(),
                    &prepared,
                    Some(signal),
                )?)
            },
        )
        .await,
        OcrExecutionOutcomeContext {
            request_started,
            timeout_config,
            max_dimension,
            prepared_width: Some(prepared_width),
            prepared_height: Some(prepared_height),
        },
    )
}
