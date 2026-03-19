use std::{
    convert::TryFrom,
    env,
    io::{self, Write as _},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
    time::Instant,
};

use anyhow::{Context, Result, anyhow, ensure};
use candle_core::{DType, Device, Tensor, shape::D};
use candle_nn::VarBuilder;
use image::GenericImageView;
use image::{DynamicImage, Rgb, RgbImage, imageops};
use rand::rngs::StdRng;
use rayon::prelude::*;
use tokenizers::Tokenizer;
use tracing::{info, trace};

use crate::{
    config::{DeepseekOcrConfig, ProjectorConfig, load_ocr_config},
    quant_snapshot::{LinearSpec, QuantizedSnapshot, SnapshotLinear, SnapshotLoadPlan},
    quantization::{
        QuantModule, QuantizationOutcome, QuantizationState, backend_label, run_quantized_matmul,
    },
    transformer::{
        cache::{DynamicCache, KvCacheChunk, PromptCacheGuard},
        model::{DeepseekLanguageModel, LanguageModelOutput},
        weights::{DeferredMoeLoadSource, qualified_name},
    },
    vision::{
        ClipDebugTrace, ClipVisionModel, DeferredClipLoadSource, PreprocessParams,
        Qwen2VisionEncoder, Qwen2VisionInput, SamBackbone, SamDebugTrace,
        dynamic_preprocess_with_params, resample::resize_bicubic,
    },
};
use deepseek_ocr_core::{
    benchmark::Timer,
    inference::{
        DecodeOutcome, DecodeParameters, ModelKind, ModelLoadArgs, OcrEngine, VisionSettings,
        normalize_text,
    },
    sampling::{
        TokenSelectionParams, init_rng, select_token_id, select_token_id_from_logits_values,
    },
};

use crate::debug::{
    debug_logits_config_from_env, debug_logits_json_path_from_env, logits_top2_at_step,
    write_debug_logits_json,
};

/// Callback invoked as tokens are generated.
type ProgressCallback<'a> = Option<&'a dyn Fn(usize, &[i64])>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LowPrecisionLoadPolicy {
    pub preload_language_f32_aux: bool,
    pub preload_vision_f32_aux: bool,
    pub preload_linear_weight_f32: bool,
    pub promote_language_input_f32: bool,
    pub lazy_moe_experts: bool,
    pub lazy_clip_transformer_layers: bool,
}

impl Default for LowPrecisionLoadPolicy {
    fn default() -> Self {
        Self {
            preload_language_f32_aux: true,
            preload_vision_f32_aux: true,
            preload_linear_weight_f32: true,
            promote_language_input_f32: true,
            lazy_moe_experts: false,
            lazy_clip_transformer_layers: false,
        }
    }
}

static LOW_PRECISION_LOAD_POLICY: OnceLock<Mutex<LowPrecisionLoadPolicy>> = OnceLock::new();

fn low_precision_load_policy_slot() -> &'static Mutex<LowPrecisionLoadPolicy> {
    LOW_PRECISION_LOAD_POLICY.get_or_init(|| Mutex::new(LowPrecisionLoadPolicy::default()))
}

fn stage_trace_enabled() -> bool {
    env::var("XIUXIAN_VISION_STAGE_TRACE_STDERR")
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
}

fn emit_stage_trace(stage: &str, fields: &[(&str, String)]) {
    if !stage_trace_enabled() {
        return;
    }
    let mut line = format!("[VISION STAGE] {stage}");
    for (key, value) in fields {
        line.push(' ');
        line.push_str(key);
        line.push('=');
        line.push_str(value);
    }
    eprintln!("{line}");
    let _ = io::stderr().flush();
}

fn empty_output_trace_enabled() -> bool {
    env::var("XIUXIAN_VISION_TRACE_EMPTY_OUTPUT")
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
}

fn trace_empty_output(decoded: &str, normalized: &str, generated_tokens: &[i64]) {
    if !empty_output_trace_enabled() || !normalized.is_empty() {
        return;
    }
    let token_preview = generated_tokens
        .iter()
        .take(4)
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let decoded_preview = decoded
        .chars()
        .take(64)
        .collect::<String>()
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    eprintln!(
        "[VISION EMPTY OUTPUT] response_tokens={} decoded_chars={} normalized_chars={} token_preview=[{}] decoded_preview={}",
        generated_tokens.len(),
        decoded.chars().count(),
        normalized.chars().count(),
        token_preview,
        decoded_preview,
    );
    let _ = io::stderr().flush();
}

fn decoded_first_token_is_visible(decoded: &str) -> bool {
    !normalize_text(decoded).is_empty()
}

fn decoded_first_token_is_single_digit(decoded: &str) -> bool {
    let normalized = normalize_text(decoded);
    let mut chars = normalized.chars();
    matches!(
        (chars.next(), chars.next()),
        (Some(ch), None) if ch.is_ascii_digit()
    )
}

fn prompt_requests_single_visible_digit(prompt: &str) -> bool {
    let prompt = prompt.to_ascii_lowercase();
    prompt.contains("exactly one visible digit")
        || prompt.contains("return one visible digit")
        || prompt.contains("single visible digit")
}

fn prompt_requested_visible_first_token(prompt: &str) -> Option<String> {
    let prompt_lower = prompt.to_ascii_lowercase();
    let (start, marker_kind) = [
        ("return only the visible word ", false),
        ("return only the visible phrase ", true),
    ]
    .into_iter()
    .find_map(|(marker, is_phrase)| {
        prompt_lower
            .find(marker)
            .map(|start| (start + marker.len(), is_phrase))
    })?;
    let tail = prompt.get(start..)?.trim();
    let end = tail
        .to_ascii_lowercase()
        .find(" from the image")
        .unwrap_or(tail.len());
    let candidate = tail.get(..end)?.trim();
    if candidate.is_empty() {
        return None;
    }
    if !marker_kind && candidate.chars().any(char::is_whitespace) {
        return None;
    }
    let normalized = normalize_text(candidate);
    let first = normalized.split_whitespace().next()?;
    let anchor: String = first
        .chars()
        .skip_while(|ch| !ch.is_ascii_alphanumeric())
        .take_while(|ch| ch.is_ascii_alphanumeric())
        .collect();
    if normalized.is_empty() || anchor.is_empty() {
        return None;
    }
    Some(anchor.to_ascii_lowercase())
}

fn has_selectable_first_token_logits(logits: &[f32]) -> bool {
    logits
        .iter()
        .any(|value| value.is_finite() && *value > f32::NEG_INFINITY)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FirstTokenCandidateClass {
    Invisible,
    Visible,
    Preferred,
}

fn select_first_visible_token_id_from_logits_with<P, F>(
    logits: &[f32],
    params: &P,
    context: &[i64],
    eos_token_id: Option<i64>,
    rng: &mut StdRng,
    mut classify: F,
) -> Result<i64>
where
    P: TokenSelectionParams,
    F: FnMut(i64) -> FirstTokenCandidateClass,
{
    ensure!(!logits.is_empty(), "logits tensor is empty");
    let mut filtered = logits.to_vec();
    let mut deferred_eos = None;
    let mut deferred_visible = None;
    while has_selectable_first_token_logits(&filtered) {
        let candidate = select_token_id_from_logits_values(&filtered, params, context, rng)?;
        if Some(candidate) == eos_token_id {
            deferred_eos.get_or_insert(candidate);
        } else {
            match classify(candidate) {
                FirstTokenCandidateClass::Preferred => return Ok(candidate),
                FirstTokenCandidateClass::Visible => {
                    deferred_visible.get_or_insert(candidate);
                }
                FirstTokenCandidateClass::Invisible => {}
            }
        }
        let index = usize::try_from(candidate)
            .context("first-token candidate id out of range for filtering")?;
        if index >= filtered.len() {
            break;
        }
        filtered[index] = f32::NEG_INFINITY;
    }
    if let Some(token_id) = deferred_visible {
        return Ok(token_id);
    }
    if let Some(token_id) = deferred_eos {
        return Ok(token_id);
    }
    select_token_id_from_logits_values(logits, params, context, rng)
}

fn select_first_visible_token_id<P: TokenSelectionParams>(
    tokenizer: &Tokenizer,
    logits: &Tensor,
    params: &P,
    context: &[i64],
    eos_token_id: Option<i64>,
    rng: &mut StdRng,
) -> Result<i64> {
    let logits = logits
        .to_dtype(DType::F32)?
        .to_vec1::<f32>()
        .context("failed to extract logits for first-token filtering")?;
    let prefer_digit = params.prefer_digit_first_token();
    let preferred_visible_text = params
        .preferred_first_visible_text()
        .map(str::to_ascii_lowercase);
    select_first_visible_token_id_from_logits_with(
        &logits,
        params,
        context,
        eos_token_id,
        rng,
        |token_id| {
            let Ok(token_id_u32) = u32::try_from(token_id) else {
                return FirstTokenCandidateClass::Invisible;
            };
            let decoded = tokenizer.decode(&[token_id_u32], true).unwrap_or_default();
            if !decoded_first_token_is_visible(&decoded) {
                return FirstTokenCandidateClass::Invisible;
            }
            let normalized = normalize_text(&decoded);
            if let Some(target) = preferred_visible_text.as_deref()
                && normalized.eq_ignore_ascii_case(target)
            {
                return FirstTokenCandidateClass::Preferred;
            }
            if prefer_digit && decoded_first_token_is_single_digit(&decoded) {
                return FirstTokenCandidateClass::Preferred;
            }
            FirstTokenCandidateClass::Visible
        },
    )
}

pub(crate) fn current_low_precision_load_policy() -> LowPrecisionLoadPolicy {
    *low_precision_load_policy_slot()
        .lock()
        .expect("low precision load policy mutex poisoned")
}

pub(crate) fn with_low_precision_load_policy<T>(
    policy: LowPrecisionLoadPolicy,
    f: impl FnOnce() -> Result<T>,
) -> Result<T> {
    let _guard = LowPrecisionLoadPolicyGuard::enter(policy);
    f()
}

struct LowPrecisionLoadPolicyGuard {
    previous: LowPrecisionLoadPolicy,
}

impl LowPrecisionLoadPolicyGuard {
    fn enter(policy: LowPrecisionLoadPolicy) -> Self {
        let mut slot = low_precision_load_policy_slot()
            .lock()
            .expect("low precision load policy mutex poisoned");
        let previous = *slot;
        *slot = policy;
        drop(slot);
        Self { previous }
    }
}

impl Drop for LowPrecisionLoadPolicyGuard {
    fn drop(&mut self) {
        if let Ok(mut slot) = low_precision_load_policy_slot().lock() {
            *slot = self.previous;
        }
    }
}

fn cast_dtype(tensor: &Tensor, dtype: DType, context_msg: &'static str) -> Result<Tensor> {
    if tensor.dtype() == dtype {
        Ok(tensor.clone())
    } else {
        tensor.to_dtype(dtype).context(context_msg)
    }
}

fn cast_dtype_owned(tensor: Tensor, dtype: DType, context_msg: &'static str) -> Result<Tensor> {
    if tensor.dtype() == dtype {
        Ok(tensor)
    } else {
        tensor.to_dtype(dtype).context(context_msg)
    }
}

fn select_f32<'a, T>(dtype: DType, native: &'a T, f32: Option<&'a T>) -> &'a T {
    if dtype == DType::F32 {
        f32.unwrap_or(native)
    } else {
        native
    }
}

fn low_precision_compute_dtype(dtype: DType) -> DType {
    if matches!(dtype, DType::F16 | DType::BF16) {
        DType::F32
    } else {
        dtype
    }
}

fn language_input_compute_dtype(dtype: DType) -> DType {
    if current_low_precision_load_policy().promote_language_input_f32 {
        low_precision_compute_dtype(dtype)
    } else {
        dtype
    }
}

fn cache_store_dtype(model_dtype: DType, requested_dtype: DType) -> DType {
    if matches!(model_dtype, DType::F16 | DType::BF16) {
        DType::F32
    } else {
        requested_dtype
    }
}

pub fn load_model(args: ModelLoadArgs<'_>) -> Result<Box<dyn OcrEngine>> {
    let ModelLoadArgs {
        kind,
        config_path,
        weights_path,
        snapshot_path,
        device,
        dtype,
    } = args;
    match kind {
        ModelKind::Deepseek => {
            let model =
                DeepseekOcrModel::load(config_path, weights_path, snapshot_path, device, dtype)?;
            Ok(Box::new(model))
        }
        ModelKind::PaddleOcrVl => Err(anyhow!(
            "ModelKind::PaddleOcrVl cannot be loaded by the Deepseek engine"
        )),
        ModelKind::DotsOcr => Err(anyhow!(
            "ModelKind::DotsOcr cannot be loaded by the Deepseek engine"
        )),
        ModelKind::GlmOcr => Err(anyhow!(
            "ModelKind::GlmOcr cannot be loaded by the Deepseek engine"
        )),
    }
}

pub fn load_model_with_low_precision_policy(
    args: ModelLoadArgs<'_>,
    policy: LowPrecisionLoadPolicy,
) -> Result<Box<dyn OcrEngine>> {
    let _guard = LowPrecisionLoadPolicyGuard::enter(policy);
    load_model(args)
}

pub const DEFAULT_WEIGHTS_PATH: &str = "DeepSeek-OCR/model-00001-of-000001.safetensors";

/// Vision inputs associated with a single batch element.
#[derive(Clone, Copy)]
pub struct VisionInput<'a> {
    pub global: &'a Tensor,
    pub patches: Option<&'a Tensor>,
    pub crop_shape: Option<(usize, usize)>,
}

/// Owned buffers backing a [`VisionInput`].
pub struct OwnedVisionInput {
    pub global: Tensor,
    pub patches: Option<Tensor>,
    pub crop_shape: Option<(usize, usize)>,
}

impl OwnedVisionInput {
    pub fn as_ref(&self) -> VisionInput<'_> {
        VisionInput {
            global: &self.global,
            patches: self.patches.as_ref(),
            crop_shape: self.crop_shape,
        }
    }
}

#[derive(Clone, Debug)]
pub struct VisionProjectionOutputs {
    pub global_pre: Tensor,
    pub local_pre: Option<Tensor>,
    pub global_post: Tensor,
    pub local_post: Option<Tensor>,
    pub global_tokens: Tensor,
    pub local_tokens: Option<Tensor>,
    pub fused_tokens: Tensor,
}

struct VisionProcessArtifacts {
    fused_tokens: Tensor,
    global_pre: Tensor,
    local_pre: Option<Tensor>,
    global_post: Tensor,
    local_post: Option<Tensor>,
    global_tokens: Tensor,
    local_tokens: Option<Tensor>,
}

#[cfg_attr(not(test), allow(dead_code))]
pub struct VisionDebugFeatures {
    pub global_clip: Tensor,
    pub global_sam: Tensor,
    pub local_clip: Option<Tensor>,
    pub local_sam: Option<Tensor>,
    pub global_clip_trace: ClipDebugTrace,
    pub local_clip_trace: Option<ClipDebugTrace>,
    pub global_sam_trace: SamDebugTrace,
    pub local_sam_trace: Option<SamDebugTrace>,
}

/// Options controlling autoregressive generation.
pub struct GenerateOptions<'a> {
    pub attention_mask: Option<&'a Tensor>,
    pub position_ids: Option<&'a Tensor>,
    pub images_seq_mask: Option<&'a Tensor>,
    pub image_inputs: Option<&'a [Option<VisionInput<'a>>]>,
    pub image_embeddings: Option<&'a [Tensor]>,
    pub max_new_tokens: usize,
    pub eos_token_id: Option<i64>,
    pub progress_callback: ProgressCallback<'a>,
    pub use_cache: bool,
    pub temperature: f64,
    pub top_p: Option<f64>,
    pub top_k: Option<usize>,
    pub repetition_penalty: f32,
    pub no_repeat_ngram_size: Option<usize>,
    pub do_sample: bool,
    pub seed: Option<u64>,
    pub prefer_digit_first_token: bool,
    pub preferred_first_visible_text: Option<String>,
}

impl<'a> GenerateOptions<'a> {
    pub fn new(max_new_tokens: usize) -> Self {
        Self {
            attention_mask: None,
            position_ids: None,
            images_seq_mask: None,
            image_inputs: None,
            image_embeddings: None,
            max_new_tokens,
            eos_token_id: None,
            progress_callback: None,
            use_cache: true,
            temperature: 1.0,
            top_p: None,
            top_k: None,
            repetition_penalty: 1.0,
            no_repeat_ngram_size: None,
            do_sample: false,
            seed: None,
            prefer_digit_first_token: false,
            preferred_first_visible_text: None,
        }
    }
}

impl<'a> TokenSelectionParams for GenerateOptions<'a> {
    fn do_sample(&self) -> bool {
        self.do_sample
    }

    fn temperature(&self) -> f64 {
        self.temperature
    }

    fn top_p(&self) -> Option<f64> {
        self.top_p
    }

    fn top_k(&self) -> Option<usize> {
        self.top_k
    }

    fn repetition_penalty(&self) -> f32 {
        self.repetition_penalty
    }

    fn no_repeat_ngram_size(&self) -> Option<usize> {
        self.no_repeat_ngram_size
    }

    fn prefer_digit_first_token(&self) -> bool {
        self.prefer_digit_first_token
    }

    fn preferred_first_visible_text(&self) -> Option<&str> {
        self.preferred_first_visible_text.as_deref()
    }
}

struct ImageProjector {
    input_dim: usize,
    hidden: usize,
    weight: Option<Tensor>,
    qmatmul: Option<std::sync::Arc<candle_core::quantized::QMatMul>>,
    bias: Option<Tensor>,
    image_newline: Tensor,
    view_separator: Tensor,
    weight_label: String,
}

impl ImageProjector {
    fn load(
        vb: &VarBuilder,
        cfg: &ProjectorConfig,
        snapshot: Option<&QuantizedSnapshot>,
    ) -> Result<Self> {
        let input_dim = cfg
            .input_dim
            .with_context(|| "projector input_dim missing from config")?;
        ensure!(
            cfg.projector_type == "linear",
            "unsupported projector_type `{}`",
            cfg.projector_type
        );

        let model_vb = vb.pp("model");
        let projector_vb = model_vb.pp("projector");
        let layers_vb = projector_vb.pp("layers");
        let weight_label = qualified_name(&layers_vb, "weight");

        let mut weight = Some(
            layers_vb
                .get((cfg.n_embed, input_dim), "weight")
                .with_context(|| "missing projector weight tensor")?
                .contiguous()?,
        );
        // Optional bias
        let mut bias = if layers_vb.contains_tensor("bias") {
            Some(
                layers_vb
                    .get(cfg.n_embed, "bias")
                    .with_context(|| "missing projector bias tensor")?
                    .contiguous()?,
            )
        } else {
            None
        };
        let image_newline = model_vb
            .get(cfg.n_embed, "image_newline")
            .or_else(|_| {
                // Some OCR2 weight snapshots don't include `image_newline`.
                // The token is only used as a formatting separator in prompt assembly.
                // Keep inference working by defaulting to a zero vector.
                // (This is only relevant for the OCR2 variant.)
                Tensor::zeros(cfg.n_embed, DType::F32, model_vb.device())
            })?
            .contiguous()?;
        let view_separator = model_vb
            .get(cfg.n_embed, "view_seperator")
            .with_context(|| "missing projector view_seperator tensor")?
            .contiguous()?;

        // Try runtime quantization if enabled for projector
        use candle_core::quantized::QMatMul;
        use tracing::trace;
        let quant = QuantizationState::global();
        let mut qmatmul: Option<std::sync::Arc<QMatMul>> = None;
        let device = layers_vb.device();
        if let Some(snapshot) = snapshot {
            let mut plan = SnapshotLoadPlan::default();
            plan.push(LinearSpec::new(
                weight_label.clone(),
                cfg.n_embed,
                input_dim,
            ));
            let mut hits = plan.execute(Some(snapshot), device, None)?;
            if let Some(hit) = hits.as_mut().and_then(|map| map.remove(&weight_label)) {
                match hit {
                    SnapshotLinear::Quantized {
                        qmatmul: qm,
                        bias: snap_bias,
                    } => {
                        let path = if device.is_cuda() || device.is_metal() {
                            "kernel_upcast"
                        } else {
                            "kernel"
                        };
                        trace!(
                            module = "projector",
                            in_dim = input_dim,
                            out_dim = cfg.n_embed,
                            backend = backend_label(device),
                            path,
                            container = snapshot.container_label(),
                            source = "snapshot",
                            action = "quantized",
                            "quant-linear"
                        );
                        quant
                            .record_attempt(QuantModule::Projector, QuantizationOutcome::Quantized);
                        if let Some(bias_tensor) = snap_bias {
                            bias = Some(bias_tensor);
                        }
                        qmatmul = Some(qm);
                        weight = None;
                    }
                    SnapshotLinear::Float {
                        weight: snap_weight,
                        bias: snap_bias,
                    } => {
                        trace!(
                            module = "projector",
                            in_dim = input_dim,
                            out_dim = cfg.n_embed,
                            backend = backend_label(device),
                            path = "snapshot-float",
                            container = snapshot.container_label(),
                            source = "snapshot",
                            action = "float",
                            "quant-linear"
                        );
                        quant.record_attempt(QuantModule::Projector, QuantizationOutcome::Fallback);
                        if let Some(bias_tensor) = snap_bias {
                            bias = Some(bias_tensor);
                        }
                        weight = Some(snap_weight);
                    }
                }
            }
        }

        let weight = weight;

        Ok(Self {
            input_dim,
            hidden: cfg.n_embed,
            weight,
            qmatmul,
            bias,
            image_newline,
            view_separator,
            weight_label,
        })
    }

    fn project(&self, input: &Tensor) -> Result<Tensor> {
        let dims = input.shape().dims();
        ensure!(
            !dims.is_empty(),
            "projector input must have rank >= 1, received {:?}",
            dims
        );
        let last_dim = *dims.last().expect("at least one dim");
        ensure!(
            last_dim == self.input_dim,
            "projector expected input dim {}, got {}",
            self.input_dim,
            last_dim
        );
        let leading = dims[..dims.len() - 1].iter().product::<usize>();
        let flat = input.reshape((leading, self.input_dim))?.contiguous()?;
        let mut proj = if let Some(qm) = &self.qmatmul {
            run_quantized_matmul(&self.weight_label, qm, &flat)?
        } else {
            let weight = self
                .weight
                .as_ref()
                .context("projector float weight missing for non-quantized path")?;
            let weight_t = weight.transpose(0, 1)?;
            if flat.dtype() != weight_t.dtype() {
                let x = flat.to_dtype(weight_t.dtype())?;
                let mut out = x.matmul(&weight_t)?;
                if out.dtype() != flat.dtype() {
                    out = out.to_dtype(flat.dtype())?;
                }
                out
            } else if matches!(flat.dtype(), DType::F16 | DType::BF16) {
                // Dtype-sensitive path: keep projector matmul in f32 for low-precision inputs.
                let x = flat.to_dtype(DType::F32)?;
                let w = weight_t.to_dtype(DType::F32)?;
                x.matmul(&w)?.to_dtype(flat.dtype())?
            } else {
                flat.matmul(&weight_t)?
            }
        };
        if let Some(bias) = &self.bias {
            let bias = cast_dtype(bias, proj.dtype(), "failed to match projector bias dtype")?;
            proj = proj.broadcast_add(&bias.reshape((1, self.hidden))?)?;
        }
        proj.reshape(
            dims[..dims.len() - 1]
                .iter()
                .copied()
                .chain(std::iter::once(self.hidden))
                .collect::<Vec<_>>(),
        )
        .context("failed to reshape projector output")
    }

    fn adapt_tokens(&self, tensor: &Tensor, dtype: DType, device: &Device) -> Result<Tensor> {
        let tensor = tensor.to_device(device)?;
        cast_dtype(&tensor, dtype, "failed to cast image embeddings")
    }

    fn placeholders(&self, count: usize, dtype: DType, device: &Device) -> Result<Tensor> {
        if count == 0 {
            return Ok(Tensor::zeros((0, self.hidden), dtype, device)?);
        }
        let newline = self.adapt_tokens(&self.image_newline, dtype, device)?;
        let mut tokens = newline
            .unsqueeze(0)?
            .expand((count, self.hidden))?
            .contiguous()?;
        let separator = self
            .adapt_tokens(&self.view_separator, dtype, device)?
            .unsqueeze(0)?;
        tokens = tokens.slice_assign(&[count - 1..count, 0..self.hidden], &separator)?;
        Ok(tokens)
    }

    fn image_newline_token(&self, dtype: DType, device: &Device) -> Result<Tensor> {
        self.adapt_tokens(&self.image_newline, dtype, device)
    }

    fn view_separator_token(&self, dtype: DType, device: &Device) -> Result<Tensor> {
        self.adapt_tokens(&self.view_separator, dtype, device)
    }

    fn hidden_size(&self) -> usize {
        self.hidden
    }

    fn input_dim(&self) -> usize {
        self.input_dim
    }
}

/// High-level multimodal container that will eventually wrap the vision towers, projector, and
/// language model. For now it wires the language stack so we can exercise text-only inference.
pub struct DeepseekOcrModel {
    cfg: Arc<DeepseekOcrConfig>,
    language: DeepseekLanguageModel,
    projector_cfg: Arc<ProjectorConfig>,
    projector: ImageProjector,
    projector_f32: Option<ImageProjector>,
    variant: OcrVariant,
    vision: VisionBackend,
    vision_f32: Option<Box<VisionModules>>,
    vision_ocr2_f32: Option<Box<Qwen2VisionEncoder>>,
    device: Device,
    dtype: DType,
    weights_path: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OcrVariant {
    Ocr1,
    Ocr2,
}

enum VisionBackend {
    Ocr1(Box<VisionModules>),
    Ocr2(Box<Qwen2VisionEncoder>),
}

impl VisionBackend {
    fn sam_backbone(&self) -> &SamBackbone {
        match self {
            VisionBackend::Ocr1(vision) => &vision.sam,
            VisionBackend::Ocr2(vision) => vision.sam_backbone(),
        }
    }
}

struct VisionModules {
    sam: SamBackbone,
    clip: ClipVisionModel,
}

struct VisionContext<'a> {
    projector: &'a ImageProjector,
    vision: &'a VisionModules,
    device: &'a Device,
    dtype: DType,
    parallel: bool,
}

fn prepare_image_tensor_for_device(
    tensor: &Tensor,
    device: &Device,
    dtype: DType,
) -> Result<Tensor> {
    let mut image = if tensor.rank() == 3 {
        tensor.unsqueeze(0)?
    } else {
        tensor.clone()
    };
    ensure!(
        image.rank() == 4,
        "image tensor must have rank 4 (batch, channels, height, width)"
    );
    if !image.device().same_device(device) {
        image = image.to_device(device)?;
    }
    if image.dtype() != dtype {
        image = image.to_dtype(dtype)?;
    }
    Ok(image.contiguous()?)
}

impl<'a> VisionContext<'a> {
    fn new_with_dtype(
        model: &'a DeepseekOcrModel,
        vision: &'a VisionModules,
        projector: &'a ImageProjector,
        dtype: DType,
    ) -> Self {
        let parallel = matches!(model.device(), Device::Cpu);
        Self {
            projector,
            vision,
            device: model.device(),
            dtype,
            parallel,
        }
    }

    fn hidden_size(&self) -> usize {
        self.projector.hidden_size()
    }

    fn device(&self) -> &'a Device {
        self.device
    }

    fn parallel_enabled(&self) -> bool {
        self.parallel
    }

    fn prepare_image_tensor(&self, tensor: &Tensor) -> Result<Tensor> {
        prepare_image_tensor_for_device(tensor, self.device, self.dtype)
    }

    fn append_row_breaks(&self, grid: Tensor, newline: &Tensor) -> Result<Tensor> {
        let (rows, cols, hidden) = grid
            .shape()
            .dims3()
            .context("grid must be [rows, cols, hidden]")?;
        let grid3 = grid.reshape((rows, cols, hidden))?;
        let newline = newline
            .reshape((1, 1, hidden))?
            .expand((rows, 1, hidden))?
            .contiguous()?;
        let with_breaks = Tensor::cat(&[grid3, newline], 1)?;
        Ok(with_breaks.reshape((rows * (cols + 1), hidden))?)
    }

    fn build_clip_sam_tokens(&self, clip: &Tensor, sam: &Tensor) -> Result<Tensor> {
        let (batch, clip_seq, clip_hidden) = clip
            .shape()
            .dims3()
            .context("clip output must be [batch, seq, hidden]")?;
        ensure!(clip_seq > 0, "clip output missing sequence dimension");
        let clip_tokens = clip
            .narrow(D::Minus2, 1, clip_seq - 1)?
            .contiguous()
            .context("clip token slice not contiguous")?;
        let (sam_batch, sam_channels, sam_h, sam_w) = sam
            .shape()
            .dims4()
            .context("sam output must be [batch, channels, height, width]")?;
        ensure!(
            sam_batch == batch,
            "sam batch {} does not match clip batch {}",
            sam_batch,
            batch
        );
        let sam_tokens = sam
            .reshape((batch, sam_channels, sam_h * sam_w))?
            .transpose(1, 2)?
            .contiguous()
            .context("sam token transpose not contiguous")?;
        let (_, sam_seq, sam_hidden) = sam_tokens
            .shape()
            .dims3()
            .context("sam tokens reshape failed")?;
        let (_, clip_seq_trimmed, _) = clip_tokens
            .shape()
            .dims3()
            .context("clip token slice should be 3D")?;
        ensure!(
            clip_seq_trimmed == sam_seq,
            "clip tokens ({clip_seq_trimmed}) do not match sam tokens ({sam_seq})"
        );
        ensure!(
            clip_hidden + sam_hidden == self.projector.input_dim(),
            "combined hidden dims {}+{} do not match projector input {}",
            clip_hidden,
            sam_hidden,
            self.projector.input_dim()
        );
        let combined = Tensor::cat(&[clip_tokens, sam_tokens], D::Minus1)?;
        Ok(combined)
    }

    fn newline_for_projected(&self, projected: &Tensor, newline: &Tensor) -> Result<Tensor> {
        cast_dtype(newline, projected.dtype(), "newline dtype cast failed")
    }

    fn format_global_tokens(&self, projected: &Tensor, newline: &Tensor) -> Result<Tensor> {
        let newline = self.newline_for_projected(projected, newline)?;
        let (batch, seq, hidden) = projected
            .shape()
            .dims3()
            .context("projected global tokens must be 3D")?;
        ensure!(batch == 1, "global view expects batch size 1, got {batch}");
        let side = (seq as f64).sqrt() as usize;
        ensure!(
            side * side == seq,
            "global token count {} is not a perfect square",
            seq
        );
        let grid = projected
            .get(0)?
            .reshape((side, side, hidden))?
            .contiguous()
            .context("global grid reshape not contiguous")?;
        self.append_row_breaks(grid, &newline)
    }

    fn format_local_tokens(
        &self,
        projected: &Tensor,
        crop_shape: (usize, usize),
        newline: &Tensor,
    ) -> Result<Tensor> {
        let newline = self.newline_for_projected(projected, newline)?;
        let (patches, seq, hidden) = projected
            .shape()
            .dims3()
            .context("projected local tokens must be 3D")?;
        let (width_crops, height_crops) = crop_shape;
        ensure!(
            patches == width_crops * height_crops,
            "patch count {} does not match crop grid {}x{}",
            patches,
            width_crops,
            height_crops
        );
        let side = (seq as f64).sqrt() as usize;
        ensure!(
            side * side == seq,
            "local token count {} is not a perfect square",
            seq
        );
        let grid = projected
            .reshape((height_crops, width_crops, side, side, hidden))?
            .permute((0, 2, 1, 3, 4))?
            .reshape((height_crops * side, width_crops * side, hidden))?
            .contiguous()
            .context("local grid reshape not contiguous")?;
        self.append_row_breaks(grid, &newline)
    }

    fn process_input_full(&self, input: &VisionInput<'_>) -> Result<VisionProcessArtifacts> {
        let newline = self
            .projector
            .image_newline_token(self.dtype, self.device)
            .context("failed to adapt image_newline token")?;
        if self.parallel {
            let newline_for_global = newline.clone();
            let newline_for_local = newline.clone();
            let (global_res, local_res) = rayon::join(
                || self.compute_global(input, &newline_for_global),
                || self.compute_local(input, &newline_for_local),
            );
            let (global_pre, global_post, global_tokens) = global_res?;
            let (local_pre_opt, local_post_opt, local_tokens_opt) = local_res?;
            self.assemble_artifacts(
                global_pre,
                global_post,
                global_tokens,
                local_pre_opt,
                local_post_opt,
                local_tokens_opt,
            )
        } else {
            let (global_pre, global_post, global_tokens) = self.compute_global(input, &newline)?;
            let (local_pre_opt, local_post_opt, local_tokens_opt) =
                self.compute_local(input, &newline)?;
            self.assemble_artifacts(
                global_pre,
                global_post,
                global_tokens,
                local_pre_opt,
                local_post_opt,
                local_tokens_opt,
            )
        }
    }

    fn process_input(&self, input: &VisionInput<'_>) -> Result<Tensor> {
        let vision_stage_started = Instant::now();
        emit_stage_trace(
            "vision.process_input.started",
            &[
                (
                    "elapsed_ms",
                    vision_stage_started.elapsed().as_millis().to_string(),
                ),
                ("has_patches", input.patches.is_some().to_string()),
                (
                    "crop_shape",
                    input
                        .crop_shape
                        .map(|(w, h)| format!("{w}x{h}"))
                        .unwrap_or_else(|| "none".to_string()),
                ),
            ],
        );
        let artifacts = self.process_input_full(input)?;
        emit_stage_trace(
            "vision.process_input.completed",
            &[
                (
                    "elapsed_ms",
                    vision_stage_started.elapsed().as_millis().to_string(),
                ),
                (
                    "fused_tokens",
                    artifacts
                        .fused_tokens
                        .shape()
                        .dims()
                        .first()
                        .copied()
                        .unwrap_or(0)
                        .to_string(),
                ),
            ],
        );
        Ok(artifacts.fused_tokens)
    }

    fn compute_global(
        &self,
        input: &VisionInput<'_>,
        newline: &Tensor,
    ) -> Result<(Tensor, Tensor, Tensor)> {
        let global_stage_started = Instant::now();
        emit_stage_trace(
            "vision.compute_global.started",
            &[(
                "elapsed_ms",
                global_stage_started.elapsed().as_millis().to_string(),
            )],
        );
        let global = self
            .prepare_image_tensor(input.global)
            .context("invalid global image tensor")?;
        let (global_batch, global_channels, global_height, global_width) =
            global.shape().dims4().context("global tensor must be 4D")?;
        emit_stage_trace(
            "vision.compute_global.prepare_image_tensor.completed",
            &[
                (
                    "elapsed_ms",
                    global_stage_started.elapsed().as_millis().to_string(),
                ),
                ("batch", global_batch.to_string()),
                ("channels", global_channels.to_string()),
                ("height", global_height.to_string()),
                ("width", global_width.to_string()),
            ],
        );
        let sam_global = self
            .vision
            .sam
            .forward(&global)
            .context("sam forward (global)")?;
        emit_stage_trace(
            "vision.compute_global.sam.completed",
            &[(
                "elapsed_ms",
                global_stage_started.elapsed().as_millis().to_string(),
            )],
        );
        let clip_global = self
            .vision
            .clip
            .forward(&global, Some(&sam_global))
            .context("clip forward (global)")?;
        emit_stage_trace(
            "vision.compute_global.clip.completed",
            &[(
                "elapsed_ms",
                global_stage_started.elapsed().as_millis().to_string(),
            )],
        );
        let global_pre = self
            .build_clip_sam_tokens(&clip_global, &sam_global)
            .context("concat global clip+sam tokens")?
            .contiguous()
            .context("global pre tokens not contiguous")?;
        let global_post = self
            .projector
            .project(&global_pre)
            .context("project global features")?
            .contiguous()
            .context("global post tokens not contiguous")?;
        emit_stage_trace(
            "vision.compute_global.projector.completed",
            &[(
                "elapsed_ms",
                global_stage_started.elapsed().as_millis().to_string(),
            )],
        );
        let global_tokens = self
            .format_global_tokens(&global_post, newline)
            .context("format global tokens")?
            .contiguous()
            .context("global tokens not contiguous")?;
        emit_stage_trace(
            "vision.compute_global.completed",
            &[
                (
                    "elapsed_ms",
                    global_stage_started.elapsed().as_millis().to_string(),
                ),
                (
                    "token_rows",
                    global_tokens
                        .shape()
                        .dims()
                        .first()
                        .copied()
                        .unwrap_or(0)
                        .to_string(),
                ),
            ],
        );
        Ok((global_pre, global_post, global_tokens))
    }

    fn compute_local(
        &self,
        input: &VisionInput<'_>,
        newline: &Tensor,
    ) -> Result<(Option<Tensor>, Option<Tensor>, Option<Tensor>)> {
        let local_stage_started = Instant::now();
        emit_stage_trace(
            "vision.compute_local.started",
            &[
                (
                    "elapsed_ms",
                    local_stage_started.elapsed().as_millis().to_string(),
                ),
                ("has_patches", input.patches.is_some().to_string()),
            ],
        );
        if let Some(patches) = input.patches {
            let crop_shape = input
                .crop_shape
                .context("crop_shape required when patches are provided")?;
            let patches = self
                .prepare_image_tensor(patches)
                .context("invalid patch tensor")?;
            let (patch_batch, _c, _h, _w) = patches
                .shape()
                .dims4()
                .context("patch tensor must be 4D (batch, channels, height, width)")?;
            emit_stage_trace(
                "vision.compute_local.prepare_image_tensor.completed",
                &[
                    (
                        "elapsed_ms",
                        local_stage_started.elapsed().as_millis().to_string(),
                    ),
                    ("patch_batch", patch_batch.to_string()),
                    ("crop_width", crop_shape.0.to_string()),
                    ("crop_height", crop_shape.1.to_string()),
                ],
            );
            if patch_batch > 0 {
                if self.parallel {
                    let chunks = patches.chunk(patch_batch, 0)?;
                    let local_results: Result<Vec<(Tensor, Tensor)>> = chunks
                        .into_par_iter()
                        .map(|chunk| self.process_patch_chunk(chunk))
                        .collect();
                    let (local_pre_list, local_post_list): (Vec<_>, Vec<_>) = local_results?
                        .into_iter()
                        .unzip::<Tensor, Tensor, Vec<_>, Vec<_>>();

                    let local_pre_refs: Vec<_> = local_pre_list.iter().collect();
                    let local_post_refs: Vec<_> = local_post_list.iter().collect();
                    let local_pre = Tensor::cat(&local_pre_refs, 0)?
                        .contiguous()
                        .context("local pre tokens not contiguous")?;
                    let local_post = Tensor::cat(&local_post_refs, 0)?
                        .contiguous()
                        .context("local post tokens not contiguous")?;
                    let local_tokens = self
                        .format_local_tokens(&local_post, crop_shape, newline)
                        .context("format local tokens")?
                        .contiguous()
                        .context("local tokens not contiguous")?;
                    emit_stage_trace(
                        "vision.compute_local.completed",
                        &[
                            (
                                "elapsed_ms",
                                local_stage_started.elapsed().as_millis().to_string(),
                            ),
                            (
                                "token_rows",
                                local_tokens
                                    .shape()
                                    .dims()
                                    .first()
                                    .copied()
                                    .unwrap_or(0)
                                    .to_string(),
                            ),
                        ],
                    );
                    return Ok((Some(local_pre), Some(local_post), Some(local_tokens)));
                } else {
                    let patches = patches
                        .contiguous()
                        .context("local patch tensor not contiguous")?;
                    let (local_pre, local_post) = self.process_patch_batch(&patches)?;
                    let local_tokens = self
                        .format_local_tokens(&local_post, crop_shape, newline)
                        .context("format local tokens")?
                        .contiguous()
                        .context("local tokens not contiguous")?;
                    emit_stage_trace(
                        "vision.compute_local.completed",
                        &[
                            (
                                "elapsed_ms",
                                local_stage_started.elapsed().as_millis().to_string(),
                            ),
                            (
                                "token_rows",
                                local_tokens
                                    .shape()
                                    .dims()
                                    .first()
                                    .copied()
                                    .unwrap_or(0)
                                    .to_string(),
                            ),
                        ],
                    );
                    return Ok((Some(local_pre), Some(local_post), Some(local_tokens)));
                }
            }
        }
        emit_stage_trace(
            "vision.compute_local.completed",
            &[
                (
                    "elapsed_ms",
                    local_stage_started.elapsed().as_millis().to_string(),
                ),
                ("token_rows", "0".to_string()),
            ],
        );
        Ok((None, None, None))
    }

    fn process_patch_chunk(&self, chunk: Tensor) -> Result<(Tensor, Tensor)> {
        let chunk = chunk
            .contiguous()
            .context("local patch chunk not contiguous")?;
        self.process_patch_batch(&chunk)
    }

    fn process_patch_batch(&self, batch: &Tensor) -> Result<(Tensor, Tensor)> {
        let patch_stage_started = Instant::now();
        emit_stage_trace(
            "vision.process_patch_batch.started",
            &[(
                "elapsed_ms",
                patch_stage_started.elapsed().as_millis().to_string(),
            )],
        );
        let sam_local = self
            .vision
            .sam
            .forward(batch)
            .context("sam forward (local)")?;
        emit_stage_trace(
            "vision.process_patch_batch.sam.completed",
            &[(
                "elapsed_ms",
                patch_stage_started.elapsed().as_millis().to_string(),
            )],
        );
        let clip_local = self
            .vision
            .clip
            .forward(batch, Some(&sam_local))
            .context("clip forward (local)")?;
        emit_stage_trace(
            "vision.process_patch_batch.clip.completed",
            &[(
                "elapsed_ms",
                patch_stage_started.elapsed().as_millis().to_string(),
            )],
        );
        let local_pre = self
            .build_clip_sam_tokens(&clip_local, &sam_local)
            .context("concat local clip+sam tokens")?
            .contiguous()
            .context("local pre tokens not contiguous")?;
        let local_post = self
            .projector
            .project(&local_pre)
            .context("project local features")?
            .contiguous()
            .context("local post tokens not contiguous")?;
        emit_stage_trace(
            "vision.process_patch_batch.projector.completed",
            &[(
                "elapsed_ms",
                patch_stage_started.elapsed().as_millis().to_string(),
            )],
        );
        Ok((local_pre, local_post))
    }

    fn assemble_artifacts(
        &self,
        global_pre: Tensor,
        global_post: Tensor,
        global_tokens: Tensor,
        local_pre_opt: Option<Tensor>,
        local_post_opt: Option<Tensor>,
        local_tokens_opt: Option<Tensor>,
    ) -> Result<VisionProcessArtifacts> {
        let target_dtype = global_tokens.dtype();
        let mut segments = Vec::new();
        if let Some(local_tokens) = local_tokens_opt.clone() {
            let local_tokens =
                cast_dtype_owned(local_tokens, target_dtype, "local tokens dtype cast failed")?;
            segments.push(local_tokens);
        }
        segments.push(global_tokens.clone());
        let view_separator = self
            .projector
            .view_separator_token(self.dtype, self.device)
            .context("failed to adapt view separator token")?
            .reshape((1, self.projector.hidden_size()))?
            .contiguous()
            .context("view separator not contiguous")?;
        let view_separator = cast_dtype_owned(
            view_separator,
            target_dtype,
            "view separator dtype cast failed",
        )?;
        segments.push(view_separator);
        let fused_tokens = Tensor::cat(&segments, 0)
            .context("failed to concatenate image segments")?
            .contiguous()
            .context("fused tokens not contiguous")?;

        Ok(VisionProcessArtifacts {
            fused_tokens,
            global_pre,
            local_pre: local_pre_opt,
            global_post,
            local_post: local_post_opt,
            global_tokens,
            local_tokens: local_tokens_opt,
        })
    }
}

impl DeepseekOcrModel {
    fn vision_modules(&self) -> Option<&VisionModules> {
        match &self.vision {
            VisionBackend::Ocr1(vision) => Some(vision.as_ref()),
            VisionBackend::Ocr2(_) => None,
        }
    }

    fn vision_modules_f32(&self) -> Option<&VisionModules> {
        self.vision_f32.as_deref()
    }

    fn projector_for_dtype(&self, dtype: DType) -> &ImageProjector {
        select_f32(dtype, &self.projector, self.projector_f32.as_ref())
    }

    /// Load the OCR model from disk, pulling configuration and language-model weights.
    ///
    /// The vision/projector paths are stubbed for now; they will be filled in once the Candle
    /// kernels land. `device` controls where tensors are allocated (CPU/GPU).
    pub fn load(
        config_path: Option<&Path>,
        weights_path: Option<&Path>,
        snapshot_path: Option<&Path>,
        device: Device,
        dtype: DType,
    ) -> Result<Self> {
        let load_started_at = Instant::now();
        let config_path_label = config_path
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "<default>".to_string());
        let weights_path_label = weights_path
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| DEFAULT_WEIGHTS_PATH.to_string());
        let snapshot_path_label = snapshot_path
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "<none>".to_string());
        emit_stage_trace(
            "deepseek.load.entered",
            &[
                ("config_path", config_path_label.clone()),
                ("weights_path", weights_path_label.clone()),
                ("snapshot_path", snapshot_path_label),
                ("device", format!("{device:?}")),
                ("dtype", format!("{dtype:?}")),
            ],
        );
        let cfg = Arc::new(load_ocr_config(config_path)?);
        emit_stage_trace(
            "deepseek.load.config.completed",
            &[(
                "elapsed_ms",
                load_started_at.elapsed().as_millis().to_string(),
            )],
        );
        info!(
            elapsed_ms = load_started_at.elapsed().as_millis(),
            config_path = config_path.map(|path| path.display().to_string()),
            "deepseek load stage completed: config"
        );
        let variant = detect_ocr_variant(&cfg);
        let language_cfg = Arc::new(cfg.resolved_language_config()?);
        let snapshot = if let Some(path) = snapshot_path {
            info!(
                path = %path.display(),
                "snapshot requested via model registry"
            );
            let snapshot = QuantizedSnapshot::load(path)
                .with_context(|| format!("failed to load snapshot from {}", path.display()))?;
            // No conflict checks: snapshot dtype is the single source of truth.
            let snap = Arc::new(snapshot);
            let hdr = snap.header();
            info!(
                container = "dsq",
                dtype = %hdr.default_qdtype,
                tensors = hdr.tensor_count,
                backend = %hdr.backend,
                "quant snapshot overview"
            );
            info!(
                elapsed_ms = load_started_at.elapsed().as_millis(),
                snapshot_path = %path.display(),
                "deepseek load stage completed: snapshot"
            );
            Some(snap)
        } else {
            // No snapshot provided: run with float weights (runtime quantization path removed).
            info!(
                elapsed_ms = load_started_at.elapsed().as_millis(),
                "deepseek load stage completed: snapshot_skip"
            );
            None
        };
        let resolved_weights = weights_path
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_WEIGHTS_PATH));
        emit_stage_trace(
            "deepseek.load.weights_mmap.started",
            &[
                (
                    "elapsed_ms",
                    load_started_at.elapsed().as_millis().to_string(),
                ),
                ("weights_path", weights_path_label),
            ],
        );
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[resolved_weights.as_path()], dtype, &device)
        }
        .with_context(|| format!("failed to mmap weights at {}", resolved_weights.display()))?;
        emit_stage_trace(
            "deepseek.load.weights_mmap.completed",
            &[(
                "elapsed_ms",
                load_started_at.elapsed().as_millis().to_string(),
            )],
        );
        info!(
            elapsed_ms = load_started_at.elapsed().as_millis(),
            weights_path = %resolved_weights.display(),
            device = ?device,
            dtype = ?dtype,
            "deepseek load stage completed: weights_mmap"
        );
        let low_precision_policy = current_low_precision_load_policy();
        let deferred_moe_source = DeferredMoeLoadSource::new(
            vec![resolved_weights.clone()],
            dtype,
            &device,
            low_precision_policy,
        );
        emit_stage_trace(
            "deepseek.load.deferred_moe_source.completed",
            &[(
                "elapsed_ms",
                load_started_at.elapsed().as_millis().to_string(),
            )],
        );
        emit_stage_trace(
            "deepseek.load.language.started",
            &[(
                "elapsed_ms",
                load_started_at.elapsed().as_millis().to_string(),
            )],
        );
        let mut language = DeepseekLanguageModel::load_with_snapshot_and_deferred_source(
            language_cfg.clone(),
            &vb,
            snapshot.as_deref(),
            Some(&deferred_moe_source),
        )
        .context("failed to load language model")?;
        info!(
            elapsed_ms = load_started_at.elapsed().as_millis(),
            variant = ?variant,
            "deepseek load stage completed: language"
        );
        let low_precision = matches!(dtype, DType::F16 | DType::BF16);
        info!(
            low_precision,
            preload_language_f32_aux = low_precision_policy.preload_language_f32_aux,
            preload_vision_f32_aux = low_precision_policy.preload_vision_f32_aux,
            preload_linear_weight_f32 = low_precision_policy.preload_linear_weight_f32,
            lazy_moe_experts = low_precision_policy.lazy_moe_experts,
            lazy_clip_transformer_layers = low_precision_policy.lazy_clip_transformer_layers,
            "deepseek load policy resolved"
        );
        if matches!(variant, OcrVariant::Ocr1) && low_precision_policy.lazy_clip_transformer_layers
        {
            info!(
                elapsed_ms = load_started_at.elapsed().as_millis(),
                "deepseek load stage enabled: lazy_clip_transformer_layers"
            );
        }
        let clip_deferred_source = if matches!(variant, OcrVariant::Ocr1)
            && low_precision_policy.lazy_clip_transformer_layers
        {
            Some(DeferredClipLoadSource::new(
                vec![resolved_weights.clone()],
                dtype,
                &device,
                "model.vision_model.transformer",
                low_precision_policy,
            ))
        } else {
            None
        };
        let clip_f32_deferred_source = if matches!(variant, OcrVariant::Ocr1)
            && low_precision_policy.lazy_clip_transformer_layers
        {
            Some(DeferredClipLoadSource::new(
                vec![resolved_weights.clone()],
                DType::F32,
                &device,
                "model.vision_model.transformer",
                low_precision_policy,
            ))
        } else {
            None
        };
        if low_precision && low_precision_policy.preload_language_f32_aux {
            let vb_f32_lang = unsafe {
                VarBuilder::from_mmaped_safetensors(
                    &[resolved_weights.as_path()],
                    DType::F32,
                    &device,
                )
            }
            .with_context(|| {
                format!(
                    "failed to mmap f32 language weights at {}",
                    resolved_weights.display()
                )
            })?;

            let norm_f32 = vb_f32_lang
                .pp("model")
                .pp("norm")
                .get(language_cfg.hidden_size, "weight")
                .context("failed to load f32 final layernorm weight")?
                .to_dtype(DType::F32)?
                .contiguous()?;
            let lm_head_f32 = vb_f32_lang
                .pp("lm_head")
                .get(
                    (language_cfg.vocab_size, language_cfg.hidden_size),
                    "weight",
                )
                .context("failed to load f32 lm_head weight")?
                .to_dtype(DType::F32)?
                .contiguous()?;
            language.set_output_weights_f32(norm_f32, lm_head_f32);
            info!(
                elapsed_ms = load_started_at.elapsed().as_millis(),
                "deepseek load stage completed: language_f32_aux"
            );
        } else if low_precision {
            info!(
                elapsed_ms = load_started_at.elapsed().as_millis(),
                "deepseek load stage skipped: language_f32_aux"
            );
        }
        let projector_cfg = Arc::new(
            cfg.resolved_projector_config()
                .context("projector configuration missing")?,
        );
        ensure!(
            projector_cfg.n_embed == language.config().hidden_size,
            "projector n_embed {} mismatches language hidden size {}",
            projector_cfg.n_embed,
            language.config().hidden_size
        );
        let projector = ImageProjector::load(&vb, projector_cfg.as_ref(), snapshot.as_deref())
            .context("failed to load image projector")?;
        info!(
            elapsed_ms = load_started_at.elapsed().as_millis(),
            "deepseek load stage completed: projector"
        );
        let low_precision = matches!(dtype, DType::F16 | DType::BF16);
        let (projector_f32, vision_f32, vision_ocr2_f32) = if low_precision
            && low_precision_policy.preload_vision_f32_aux
        {
            let vb_f32 = unsafe {
                VarBuilder::from_mmaped_safetensors(
                    &[resolved_weights.as_path()],
                    DType::F32,
                    &device,
                )
            }
            .with_context(|| {
                format!(
                    "failed to mmap f32 weights at {}",
                    resolved_weights.display()
                )
            })?;
            let projector_f32 =
                ImageProjector::load(&vb_f32, projector_cfg.as_ref(), snapshot.as_deref())
                    .context("failed to load f32 image projector")?;
            let (vision_f32, vision_ocr2_f32) = match variant {
                OcrVariant::Ocr1 => {
                    info!(
                        elapsed_ms = load_started_at.elapsed().as_millis(),
                        "deepseek load stage started: vision_f32_sam"
                    );
                    let sam = SamBackbone::new(cfg.as_ref(), &vb_f32.pp("model").pp("sam_model"))
                        .context("failed to load f32 SAM backbone")?;
                    info!(
                        elapsed_ms = load_started_at.elapsed().as_millis(),
                        "deepseek load stage completed: vision_f32_sam"
                    );
                    info!(
                        elapsed_ms = load_started_at.elapsed().as_millis(),
                        "deepseek load stage started: vision_f32_clip"
                    );
                    let clip = ClipVisionModel::load_with_deferred_source(
                        cfg.as_ref(),
                        &vb_f32.pp("model").pp("vision_model"),
                        clip_f32_deferred_source.as_ref(),
                    )
                    .context("failed to load f32 CLIP vision model")?;
                    info!(
                        elapsed_ms = load_started_at.elapsed().as_millis(),
                        "deepseek load stage completed: vision_f32_clip"
                    );
                    (Some(Box::new(VisionModules { sam, clip })), None)
                }
                OcrVariant::Ocr2 => {
                    info!(
                        elapsed_ms = load_started_at.elapsed().as_millis(),
                        "deepseek load stage started: vision_f32_ocr2_encoder"
                    );
                    let vision = Qwen2VisionEncoder::new(cfg.as_ref(), &vb_f32)
                        .context("failed to load f32 Qwen2 vision encoder")?;
                    info!(
                        elapsed_ms = load_started_at.elapsed().as_millis(),
                        "deepseek load stage completed: vision_f32_ocr2_encoder"
                    );
                    (None, Some(Box::new(vision)))
                }
            };
            info!(
                elapsed_ms = load_started_at.elapsed().as_millis(),
                "deepseek load stage completed: low_precision_f32_aux"
            );
            (Some(projector_f32), vision_f32, vision_ocr2_f32)
        } else if low_precision {
            info!(
                elapsed_ms = load_started_at.elapsed().as_millis(),
                "deepseek load stage skipped: low_precision_f32_aux"
            );
            (None, None, None)
        } else {
            (None, None, None)
        };
        let vision = match variant {
            OcrVariant::Ocr1 => {
                info!(
                    elapsed_ms = load_started_at.elapsed().as_millis(),
                    "deepseek load stage started: vision_sam"
                );
                let sam = SamBackbone::new(cfg.as_ref(), &vb.pp("model").pp("sam_model"))
                    .context("failed to load SAM backbone")?;
                info!(
                    elapsed_ms = load_started_at.elapsed().as_millis(),
                    "deepseek load stage completed: vision_sam"
                );
                info!(
                    elapsed_ms = load_started_at.elapsed().as_millis(),
                    "deepseek load stage started: vision_clip"
                );
                let clip = ClipVisionModel::load_with_deferred_source(
                    cfg.as_ref(),
                    &vb.pp("model").pp("vision_model"),
                    clip_deferred_source.as_ref(),
                )
                .context("failed to load CLIP vision model")?;
                info!(
                    elapsed_ms = load_started_at.elapsed().as_millis(),
                    "deepseek load stage completed: vision_clip"
                );
                VisionBackend::Ocr1(Box::new(VisionModules { sam, clip }))
            }
            OcrVariant::Ocr2 => {
                info!(
                    elapsed_ms = load_started_at.elapsed().as_millis(),
                    "deepseek load stage started: vision_ocr2_encoder"
                );
                let vision = Qwen2VisionEncoder::new(cfg.as_ref(), &vb)
                    .context("failed to load Qwen2 vision encoder")?;
                info!(
                    elapsed_ms = load_started_at.elapsed().as_millis(),
                    "deepseek load stage completed: vision_ocr2_encoder"
                );
                VisionBackend::Ocr2(Box::new(vision))
            }
        };
        info!(
            elapsed_ms = load_started_at.elapsed().as_millis(),
            variant = ?variant,
            "deepseek load stage completed: vision"
        );
        // Log quantization summary after all quantizable modules (language + projector) are loaded.
        QuantizationState::global().log_summary(&device);
        info!(
            elapsed_ms = load_started_at.elapsed().as_millis(),
            "deepseek load stage completed: quant_summary"
        );
        info!(
            elapsed_ms = load_started_at.elapsed().as_millis(),
            variant = ?variant,
            weights_path = %resolved_weights.display(),
            device = ?device,
            dtype = ?dtype,
            "deepseek load stage completed: model_ready"
        );

        Ok(Self {
            cfg,
            language,
            projector_cfg,
            projector,
            projector_f32,
            variant,
            vision,
            vision_f32,
            vision_ocr2_f32,
            device,
            dtype,
            weights_path: resolved_weights,
        })
    }

    /// Access the currently loaded configuration.
    pub fn config(&self) -> &DeepseekOcrConfig {
        self.cfg.as_ref()
    }

    /// Device backing the allocated tensors.
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// DType the model was loaded with.
    pub fn dtype(&self) -> DType {
        self.dtype
    }

    /// Path the weights were loaded from (useful for logging).
    pub fn weights_path(&self) -> &Path {
        &self.weights_path
    }

    /// Borrow the language-only component.
    pub fn language_model(&self) -> &DeepseekLanguageModel {
        &self.language
    }

    /// Whether flash attention is enabled for the underlying decoder.
    pub fn flash_attention_enabled(&self) -> bool {
        self.language.flash_attention_enabled()
    }

    /// Access the projector configuration.
    pub fn projector_config(&self) -> &ProjectorConfig {
        self.projector_cfg.as_ref()
    }

    /// Construct a fresh dynamic cache sized for this model.
    pub fn new_cache(&self) -> DynamicCache {
        let layers = self.language.transformer_weights().layers.len();
        DynamicCache::with_num_layers(layers)
    }

    /// Construct a fresh dynamic cache sized for this model, matching the model dtype.
    pub fn new_cache_for_dtype(&self, dtype: DType) -> Result<DynamicCache> {
        let layers = self.language.transformer_weights().layers.len();
        let mut cache = DynamicCache::with_num_layers(layers);
        // Pre-seed a zero-length cache entry per layer so cache dtype is deterministic.
        // Low-precision models keep cache storage in f32 to reduce accumulation drift.
        let store_dtype = cache_store_dtype(self.dtype, dtype);
        for layer in 0..layers {
            let heads = self.language.config().num_attention_heads;
            let head_dim = self.language.config().hidden_size / heads;
            let v_head_dim = self.language.config().v_head_dim.unwrap_or(head_dim);
            let device = self.device.clone();
            let key_t =
                Tensor::zeros((1, heads, head_dim, 0), store_dtype, &device)?.contiguous()?;
            let value =
                Tensor::zeros((1, heads, 0, v_head_dim), store_dtype, &device)?.contiguous()?;
            cache.append(layer, KvCacheChunk::new(key_t, value)?)?;
        }
        Ok(cache)
    }

    /// Helper to guard prompt-scoped cache state.
    pub fn prompt_guard<'a>(&'a self, cache: &'a mut DynamicCache) -> PromptCacheGuard<'a> {
        self.language.prompt_guard(cache)
    }

    #[doc(hidden)]
    pub fn sam_backbone(&self) -> &SamBackbone {
        self.vision.sam_backbone()
    }

    /// Forward pass through the multimodal stack, applying optional image-token injection.
    #[allow(clippy::too_many_arguments)]
    pub fn forward<'a>(
        &self,
        input_ids: Option<&Tensor>,
        inputs_embeds: Option<&Tensor>,
        attention_mask: Option<&Tensor>,
        position_ids: Option<&Tensor>,
        images_seq_mask: Option<&Tensor>,
        vision_inputs: Option<&'a [Option<VisionInput<'a>>]>,
        image_embeddings: Option<&'a [Tensor]>,
        cache: Option<&mut DynamicCache>,
        use_cache: bool,
    ) -> Result<LanguageModelOutput> {
        ensure!(
            input_ids.is_some() ^ inputs_embeds.is_some(),
            "provide exactly one of input_ids or inputs_embeds"
        );
        ensure!(
            !use_cache || cache.is_some(),
            "use_cache=true requires a mutable DynamicCache"
        );
        if vision_inputs.is_some() || image_embeddings.is_some() {
            ensure!(
                images_seq_mask.is_some(),
                "image masks required when providing image inputs or embeddings"
            );
        }

        let mut embeddings = match inputs_embeds {
            Some(t) => t.clone(),
            None => {
                let ids = input_ids.expect("input_ids validity checked above");
                self.language.embed_tokens(ids)?
            }
        };

        let computed_embeddings = if image_embeddings.is_none() {
            if let Some(inputs) = vision_inputs {
                Some(self.compute_image_embeddings(inputs)?)
            } else {
                None
            }
        } else {
            None
        };

        let image_embeddings_slice = image_embeddings
            .map(Some)
            .unwrap_or_else(|| computed_embeddings.as_deref());

        if let Some(mask) = images_seq_mask {
            embeddings = self.inject_image_tokens(embeddings, mask, image_embeddings_slice)?;
        }

        let language_input_dtype = language_input_compute_dtype(self.dtype);
        embeddings = cast_dtype_owned(
            embeddings,
            language_input_dtype,
            "failed to cast language input embeddings",
        )?;

        let lm_out = self.language.forward(
            None,
            Some(&embeddings),
            attention_mask,
            position_ids,
            cache,
            use_cache,
        )?;

        Ok(lm_out)
    }

    /// Convenience wrapper around the language-model forward path without image tokens.
    pub fn forward_language(
        &self,
        input_ids: Option<&Tensor>,
        inputs_embeds: Option<&Tensor>,
        attention_mask: Option<&Tensor>,
        position_ids: Option<&Tensor>,
        cache: Option<&mut DynamicCache>,
        use_cache: bool,
    ) -> Result<LanguageModelOutput> {
        self.forward(
            input_ids,
            inputs_embeds,
            attention_mask,
            position_ids,
            None,
            None,
            None,
            cache,
            use_cache,
        )
    }

    pub fn compute_image_embeddings(
        &self,
        inputs: &[Option<VisionInput<'_>>],
    ) -> Result<Vec<Tensor>> {
        match &self.vision {
            VisionBackend::Ocr1(_vision) => {
                let compute_dtype = low_precision_compute_dtype(self.dtype);
                let vision_native = self.vision_modules().context("vision modules missing")?;
                let vision = select_f32(compute_dtype, vision_native, self.vision_modules_f32());
                let projector = self.projector_for_dtype(compute_dtype);
                let ctx = VisionContext::new_with_dtype(self, vision, projector, compute_dtype);
                let hidden = ctx.hidden_size();
                let device = ctx.device();
                if ctx.parallel_enabled() {
                    inputs
                        .par_iter()
                        .map(|input| {
                            if let Some(vision_input) = input {
                                ctx.process_input(vision_input)
                            } else {
                                Tensor::zeros((0, hidden), compute_dtype, device)
                                    .map_err(Into::into)
                            }
                        })
                        .collect::<Result<Vec<_>>>()
                } else {
                    inputs
                        .iter()
                        .map(|input| {
                            if let Some(vision_input) = input {
                                ctx.process_input(vision_input)
                            } else {
                                Tensor::zeros((0, hidden), compute_dtype, device)
                                    .map_err(Into::into)
                            }
                        })
                        .collect::<Result<Vec<_>>>()
                }
            }
            VisionBackend::Ocr2(vision) => {
                let hidden = self.projector.hidden_size();
                let compute_dtype = low_precision_compute_dtype(self.dtype());
                let device = self.device();
                let parallel = matches!(self.device(), Device::Cpu);
                let vision = select_f32(
                    compute_dtype,
                    vision.as_ref(),
                    self.vision_ocr2_f32.as_deref(),
                );

                let encode = |vision_input: &VisionInput<'_>| -> Result<Tensor> {
                    let global = prepare_image_tensor_for_device(
                        vision_input.global,
                        device,
                        compute_dtype,
                    )?;
                    let patches = if let Some(patches) = vision_input.patches {
                        Some(prepare_image_tensor_for_device(
                            patches,
                            device,
                            compute_dtype,
                        )?)
                    } else {
                        None
                    };
                    let qwen_input = Qwen2VisionInput {
                        global: &global,
                        patches: patches.as_ref(),
                        crop_shape: vision_input.crop_shape,
                    };
                    let encoded = vision.encode(qwen_input)?;
                    cast_dtype_owned(encoded, compute_dtype, "qwen2 encoded dtype cast failed")
                };

                if parallel {
                    inputs
                        .par_iter()
                        .map(|input| {
                            if let Some(vision_input) = input {
                                encode(vision_input)
                            } else {
                                Tensor::zeros((0, hidden), compute_dtype, device)
                                    .map_err(Into::into)
                            }
                        })
                        .collect()
                } else {
                    inputs
                        .iter()
                        .map(|input| {
                            if let Some(vision_input) = input {
                                encode(vision_input)
                            } else {
                                Tensor::zeros((0, hidden), compute_dtype, device)
                                    .map_err(Into::into)
                            }
                        })
                        .collect()
                }
            }
        }
    }

    pub fn compute_vision_projection(
        &self,
        input: &VisionInput<'_>,
    ) -> Result<VisionProjectionOutputs> {
        let artifacts = self.process_vision_input_full(input)?;
        let VisionProcessArtifacts {
            fused_tokens,
            global_pre,
            local_pre,
            global_post,
            local_post,
            global_tokens,
            local_tokens,
        } = artifacts;

        let (batch, _, _) = global_pre
            .shape()
            .dims3()
            .context("global pre tokens must be 3D")?;
        ensure!(
            batch == 1,
            "global pre tokens expect batch size 1, got {batch}"
        );
        let global_pre_flat = global_pre
            .get(0)?
            .contiguous()
            .context("global pre flat not contiguous")?;

        let (post_batch, _, _) = global_post
            .shape()
            .dims3()
            .context("global post tokens must be 3D")?;
        ensure!(
            post_batch == 1,
            "global post tokens expect batch size 1, got {post_batch}"
        );
        let global_post_flat = global_post
            .get(0)?
            .contiguous()
            .context("global post flat not contiguous")?;

        let local_pre_flat = if let Some(local_pre) = local_pre {
            let (patches, seq, hidden) = local_pre
                .shape()
                .dims3()
                .context("local pre tokens must be 3D")?;
            Some(
                local_pre
                    .reshape((patches * seq, hidden))?
                    .contiguous()
                    .context("local pre flat not contiguous")?,
            )
        } else {
            None
        };

        let local_post_flat = if let Some(local_post) = local_post {
            let (patches, seq, hidden) = local_post
                .shape()
                .dims3()
                .context("local post tokens must be 3D")?;
            Some(
                local_post
                    .reshape((patches * seq, hidden))?
                    .contiguous()
                    .context("local post flat not contiguous")?,
            )
        } else {
            None
        };

        Ok(VisionProjectionOutputs {
            global_pre: global_pre_flat,
            local_pre: local_pre_flat,
            global_post: global_post_flat,
            local_post: local_post_flat,
            global_tokens,
            local_tokens,
            fused_tokens,
        })
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn compute_vision_debug_features(
        &self,
        input: &VisionInput<'_>,
    ) -> Result<VisionDebugFeatures> {
        let vision_dtype = self.dtype;
        let vision_modules = match &self.vision {
            VisionBackend::Ocr1(vision) => vision,
            VisionBackend::Ocr2(_) => {
                anyhow::bail!("vision debug features are not available for OCR2 backends")
            }
        };
        let global = self
            .prepare_image_tensor(input.global)
            .context("invalid global image tensor")?;
        let (sam_global_raw, sam_trace_global_raw) = vision_modules
            .sam
            .forward_with_trace(&global)
            .context("sam forward (global)")?;
        let sam_global = sam_global_raw
            .contiguous()
            .context("sam global not contiguous")?;
        let sam_global =
            cast_dtype_owned(sam_global, vision_dtype, "sam global dtype cast failed")?;
        let clip_trace_global_raw = vision_modules
            .clip
            .forward_with_trace(&global, Some(&sam_global))
            .context("clip forward (global)")?;
        let clip_global = clip_trace_global_raw
            .output
            .contiguous()
            .context("clip global output not contiguous")?;

        let clip_global_layers = clip_trace_global_raw
            .layer_outputs
            .into_iter()
            .enumerate()
            .map(|(idx, tensor)| {
                tensor
                    .contiguous()
                    .with_context(|| format!("clip global layer {idx} not contiguous"))
            })
            .collect::<Result<Vec<_>>>()?;
        let clip_trace_global = ClipDebugTrace {
            embeddings: clip_trace_global_raw
                .embeddings
                .contiguous()
                .context("clip global embeddings not contiguous")?,
            pre_layernorm: clip_trace_global_raw
                .pre_layernorm
                .contiguous()
                .context("clip global pre_layernorm not contiguous")?,
            layer_outputs: clip_global_layers,
            output: clip_global.clone(),
        };

        let sam_trace_global = SamDebugTrace {
            patch_embed: sam_trace_global_raw
                .patch_embed
                .contiguous()
                .context("sam global patch_embed not contiguous")?,
            pos_added: match sam_trace_global_raw.pos_added {
                Some(tensor) => Some(
                    tensor
                        .contiguous()
                        .context("sam global pos_added not contiguous")?,
                ),
                None => None,
            },
            block_outputs: sam_trace_global_raw
                .block_outputs
                .into_iter()
                .enumerate()
                .map(|(idx, tensor)| {
                    tensor
                        .contiguous()
                        .with_context(|| format!("sam global block {idx} not contiguous"))
                })
                .collect::<Result<Vec<_>>>()?,
            neck_conv1: sam_trace_global_raw
                .neck_conv1
                .contiguous()
                .context("sam global neck conv1 not contiguous")?,
            neck_norm1: sam_trace_global_raw
                .neck_norm1
                .contiguous()
                .context("sam global neck norm1 not contiguous")?,
            neck_conv2: sam_trace_global_raw
                .neck_conv2
                .contiguous()
                .context("sam global neck conv2 not contiguous")?,
            neck_norm2: sam_trace_global_raw
                .neck_norm2
                .contiguous()
                .context("sam global neck norm2 not contiguous")?,
            net2: sam_trace_global_raw
                .net2
                .contiguous()
                .context("sam global net2 not contiguous")?,
            net3: sam_trace_global_raw
                .net3
                .contiguous()
                .context("sam global net3 not contiguous")?,
        };

        let mut local_clip_opt = None;
        let mut local_sam_opt = None;
        let mut local_clip_trace_opt = None;
        let mut local_sam_trace_opt = None;

        if let Some(patches) = input.patches {
            let patches = self
                .prepare_image_tensor(patches)
                .context("invalid patch tensor")?;
            let (patch_batch, _c, _h, _w) =
                patches.shape().dims4().context("patch tensor must be 4D")?;
            if patch_batch > 0 {
                let (sam_local_raw, sam_trace_local_raw) = vision_modules
                    .sam
                    .forward_with_trace(&patches)
                    .context("sam forward (local)")?;
                let sam_local = sam_local_raw
                    .contiguous()
                    .context("sam local not contiguous")?;
                let sam_local =
                    cast_dtype_owned(sam_local, vision_dtype, "sam local dtype cast failed")?;
                let clip_trace_local_raw = vision_modules
                    .clip
                    .forward_with_trace(&patches, Some(&sam_local))
                    .context("clip forward (local)")?;
                let clip_local = clip_trace_local_raw
                    .output
                    .contiguous()
                    .context("clip local output not contiguous")?;

                let clip_local_layers = clip_trace_local_raw
                    .layer_outputs
                    .into_iter()
                    .enumerate()
                    .map(|(idx, tensor)| {
                        tensor
                            .contiguous()
                            .with_context(|| format!("clip local layer {idx} not contiguous"))
                    })
                    .collect::<Result<Vec<_>>>()?;
                let clip_trace_local = ClipDebugTrace {
                    embeddings: clip_trace_local_raw
                        .embeddings
                        .contiguous()
                        .context("clip local embeddings not contiguous")?,
                    pre_layernorm: clip_trace_local_raw
                        .pre_layernorm
                        .contiguous()
                        .context("clip local pre_layernorm not contiguous")?,
                    layer_outputs: clip_local_layers,
                    output: clip_local.clone(),
                };

                let sam_trace_local = SamDebugTrace {
                    patch_embed: sam_trace_local_raw
                        .patch_embed
                        .contiguous()
                        .context("sam local patch_embed not contiguous")?,
                    pos_added: match sam_trace_local_raw.pos_added {
                        Some(tensor) => Some(
                            tensor
                                .contiguous()
                                .context("sam local pos_added not contiguous")?,
                        ),
                        None => None,
                    },
                    block_outputs: sam_trace_local_raw
                        .block_outputs
                        .into_iter()
                        .enumerate()
                        .map(|(idx, tensor)| {
                            tensor
                                .contiguous()
                                .with_context(|| format!("sam local block {idx} not contiguous"))
                        })
                        .collect::<Result<Vec<_>>>()?,
                    neck_conv1: sam_trace_local_raw
                        .neck_conv1
                        .contiguous()
                        .context("sam local neck conv1 not contiguous")?,
                    neck_norm1: sam_trace_local_raw
                        .neck_norm1
                        .contiguous()
                        .context("sam local neck norm1 not contiguous")?,
                    neck_conv2: sam_trace_local_raw
                        .neck_conv2
                        .contiguous()
                        .context("sam local neck conv2 not contiguous")?,
                    neck_norm2: sam_trace_local_raw
                        .neck_norm2
                        .contiguous()
                        .context("sam local neck norm2 not contiguous")?,
                    net2: sam_trace_local_raw
                        .net2
                        .contiguous()
                        .context("sam local net2 not contiguous")?,
                    net3: sam_trace_local_raw
                        .net3
                        .contiguous()
                        .context("sam local net3 not contiguous")?,
                };

                local_clip_opt = Some(clip_local);
                local_sam_opt = Some(sam_local);
                local_clip_trace_opt = Some(clip_trace_local);
                local_sam_trace_opt = Some(sam_trace_local);
            }
        }

        Ok(VisionDebugFeatures {
            global_clip: clip_global,
            global_sam: sam_global,
            local_clip: local_clip_opt,
            local_sam: local_sam_opt,
            global_clip_trace: clip_trace_global,
            local_clip_trace: local_clip_trace_opt,
            global_sam_trace: sam_trace_global,
            local_sam_trace: local_sam_trace_opt,
        })
    }

    fn process_vision_input_full(&self, input: &VisionInput<'_>) -> Result<VisionProcessArtifacts> {
        match &self.vision {
            VisionBackend::Ocr1(vision) => {
                let dtype = low_precision_compute_dtype(self.dtype);
                let vision = select_f32(dtype, vision.as_ref(), self.vision_modules_f32());
                let projector = self.projector_for_dtype(dtype);
                VisionContext::new_with_dtype(self, vision, projector, dtype)
                    .process_input_full(input)
            }
            VisionBackend::Ocr2(_) => {
                anyhow::bail!("vision projection is not available for OCR2 backends")
            }
        }
    }

    fn prepare_image_tensor(&self, tensor: &Tensor) -> Result<Tensor> {
        prepare_image_tensor_for_device(tensor, self.device(), self.dtype())
    }

    /// Construct normalized tensors for a single multimodal example.
    pub fn prepare_vision_input_from_image(
        &self,
        image: &DynamicImage,
        base_size: u32,
        image_size: u32,
        crop_mode: bool,
    ) -> Result<OwnedVisionInput> {
        let global_size = if crop_mode { base_size } else { image_size };
        let global_view = build_global_view(image, global_size);
        let vision_dtype = low_precision_compute_dtype(self.dtype);
        let global = image_to_tensor(&global_view, self.device(), vision_dtype)?
            .unsqueeze(0)?
            .contiguous()?;

        let (patches, crop_shape) = if crop_mode {
            let params = match self.variant {
                OcrVariant::Ocr1 => PreprocessParams::ocr1(base_size, image_size),
                OcrVariant::Ocr2 => PreprocessParams::ocr2(base_size, image_size),
            };
            let preprocess = dynamic_preprocess_with_params(image, &params, false);
            let crop = (preprocess.grid().0 as usize, preprocess.grid().1 as usize);
            let tiles = preprocess.tiles;
            if tiles.is_empty() {
                (None, Some(crop))
            } else {
                tracing::info!("Preparing {} image crops for vision input", tiles.len());
                let device = self.device().clone();
                let dtype = vision_dtype;
                let tensors: Vec<Tensor> = if matches!(self.device(), Device::Cpu) {
                    tiles
                        .into_par_iter()
                        .map(|tile| image_to_tensor(&tile, &device, dtype))
                        .collect::<Result<Vec<_>>>()?
                } else {
                    tiles
                        .into_iter()
                        .map(|tile| image_to_tensor(&tile, &device, dtype))
                        .collect::<Result<Vec<_>>>()?
                };
                let stacked = Tensor::stack(&tensors, 0)?.contiguous()?;
                (Some(stacked), Some(crop))
            }
        } else {
            (None, None)
        };

        Ok(OwnedVisionInput {
            global,
            patches,
            crop_shape,
        })
    }

    fn inject_image_tokens(
        &self,
        embeddings: Tensor,
        mask: &Tensor,
        image_embeddings: Option<&[Tensor]>,
    ) -> Result<Tensor> {
        let (batch, seq_len, hidden) = embeddings.shape().dims3()?;
        if let Some(tokens) = image_embeddings {
            ensure!(
                tokens.len() == batch,
                "image_embeddings batch {} does not match embeddings batch {batch}",
                tokens.len()
            );
        }
        let mask = cast_dtype(mask, DType::U8, "images_seq_mask dtype cast failed")?;
        let (mask_batch, mask_seq) = mask
            .shape()
            .dims2()
            .context("images_seq_mask must have shape [batch, seq_len]")?;
        ensure!(
            mask_batch == batch && mask_seq == seq_len,
            "images_seq_mask shape ({mask_batch}, {mask_seq}) does not match embeddings ({batch}, {seq_len})"
        );

        let dtype = embeddings.dtype();
        let device = embeddings.device();
        let mut rows = Vec::with_capacity(batch);
        for b in 0..batch {
            let row = embeddings
                .get(b)?
                .reshape((seq_len, hidden))?
                .contiguous()?;
            let mask_row = mask.get(b)?.reshape((seq_len,))?;
            let mask_vec = mask_row.to_vec1::<u8>()?;
            let positions: Vec<usize> = mask_vec
                .iter()
                .enumerate()
                .filter_map(|(idx, &flag)| (flag != 0).then_some(idx))
                .collect();
            if positions.is_empty() {
                rows.push(row);
                continue;
            }
            let replacements = if let Some(tokens) = image_embeddings {
                let per_batch = tokens
                    .get(b)
                    .context("image_embeddings missing entry for batch row")?;
                let adapted = cast_dtype(per_batch, dtype, "failed to cast image embeddings")?;
                let adapted = if adapted.device().same_device(device) {
                    adapted
                } else {
                    adapted.to_device(device)?
                }
                .contiguous()?;
                let (count, embed_dim) = adapted
                    .shape()
                    .dims2()
                    .context("image embeddings must have shape [tokens, hidden]")?;
                ensure!(
                    count == positions.len(),
                    "image embeddings provide {} tokens but mask requires {}",
                    count,
                    positions.len()
                );
                ensure!(
                    embed_dim == hidden,
                    "image embedding hidden dim {} does not match language hidden size {}",
                    embed_dim,
                    hidden
                );
                adapted
            } else {
                self.projector
                    .placeholders(positions.len(), dtype, device)?
            };
            let replacements = replacements.contiguous()?;

            let replacements_full = Tensor::zeros((seq_len, hidden), dtype, device)?;
            let positions_i64: Vec<i64> = positions.iter().map(|&p| p as i64).collect();
            let idx_tensor = Tensor::from_vec(positions_i64, (positions.len(),), device)?
                .to_dtype(DType::I64)?;
            let idx_matrix = idx_tensor
                .reshape((positions.len(), 1))?
                .expand((positions.len(), hidden))?
                .contiguous()?;
            replacements_full.scatter_add_set(&idx_matrix, &replacements, 0)?;

            let mask_float = mask_row
                .to_dtype(dtype)?
                .reshape((seq_len, 1))?
                .contiguous()?;
            let ones = Tensor::ones((seq_len, 1), dtype, device)?;
            let keep = ones.sub(&mask_float)?;
            let updated = row.broadcast_mul(&keep)?.add(&replacements_full)?;
            rows.push(updated);
        }
        Ok(Tensor::stack(&rows, 0)?)
    }

    #[doc(hidden)]
    pub fn inject_image_tokens_for_tests(
        &self,
        embeddings: Tensor,
        mask: &Tensor,
        image_embeddings: Option<&[Tensor]>,
    ) -> Result<Tensor> {
        self.inject_image_tokens(embeddings, mask, image_embeddings)
    }

    /// Greedy autoregressive generation for the multimodal model.
    pub fn generate(
        &self,
        tokenizer: &Tokenizer,
        input_ids: &Tensor,
        options: GenerateOptions<'_>,
    ) -> Result<Tensor> {
        let total_timer = Timer::new("decode.generate");
        ensure!(
            input_ids.rank() == 2,
            "generate expects input_ids with shape [batch, seq]"
        );
        let (batch, seq_len) = input_ids.shape().dims2()?;
        ensure!(
            batch == 1,
            "generate currently supports batch size 1 (got {batch})"
        );
        if !options.use_cache {
            total_timer.finish(|event| {
                event.add_field("mode", "no_cache");
                event.add_field("prompt_tokens", seq_len as u64);
                event.add_field("max_new_tokens", options.max_new_tokens as u64);
            });
            return self.generate_without_cache(tokenizer, input_ids, options);
        }
        let progress_callback = options.progress_callback;
        if options.max_new_tokens == 0 {
            total_timer.finish(|event| {
                event.add_field("prompt_tokens", seq_len as u64);
                event.add_field("max_new_tokens", 0u64);
                event.add_field("generated_tokens", 0u64);
            });
            return self.empty_generation();
        }

        let mut context_tokens = {
            let rows = cast_dtype(
                input_ids,
                DType::I64,
                "failed to cast prompt tokens for generation",
            )?
            .to_vec2::<i64>()
            .context("failed to extract prompt tokens for generation")?;
            rows.into_iter()
                .next()
                .context("input_ids must have batch dimension 1")?
        };
        ensure!(
            context_tokens.len() == seq_len,
            "prompt token count {} mismatches seq_len {}",
            context_tokens.len(),
            seq_len
        );
        let mut rng = init_rng(options.seed);

        let mut cache = self.new_cache_for_dtype(self.dtype)?;
        // Zero-length seeded cache entries are only used to lock dtype; clear them before decode.
        cache.clear();
        let mut guard = self.prompt_guard(&mut cache);

        let prefill_timer = Timer::new("decode.prefill");
        let prefill = self.forward(
            Some(input_ids),
            None,
            options.attention_mask,
            options.position_ids,
            options.images_seq_mask,
            options.image_inputs,
            options.image_embeddings,
            Some(guard.cache()),
            true,
        )?;
        prefill_timer.finish(|event| {
            event.add_field("prompt_tokens", seq_len as u64);
            event.add_field("has_image_mask", options.images_seq_mask.is_some());
            event.add_field("use_cache", true);
        });
        let logits = prefill
            .logits
            .get(0)
            .context("prefill logits missing batch dimension")?;
        let last_logits = logits
            .get(seq_len - 1)
            .context("prefill logits missing final timestep")?;
        let mut current = select_first_visible_token_id(
            tokenizer,
            &last_logits,
            &options,
            &context_tokens,
            options.eos_token_id,
            &mut rng,
        )?;

        // Debug-only: capture top-2 logits at a specific generated token step.
        // step=0 corresponds to the first generated token (selected from prompt prefill logits).
        if let (Some(cfg), Some(path)) = (
            debug_logits_config_from_env(),
            debug_logits_json_path_from_env(),
        ) {
            if cfg.step == 0 {
                let info = logits_top2_at_step(0, &last_logits)?;
                write_debug_logits_json(&path, &info, current)?;
            }
        }
        if let Some(eos) = options.eos_token_id
            && current == eos
        {
            total_timer.finish(|event| {
                event.add_field("prompt_tokens", seq_len as u64);
                event.add_field("generated_tokens", 0u64);
                event.add_field("max_new_tokens", options.max_new_tokens as u64);
                event.add_field("terminated_on_prefill", true);
            });
            return self.empty_generation();
        }

        let mut generated = Vec::with_capacity(options.max_new_tokens);
        let decode_timer = Timer::new("decode.iterative");
        let decode_input_dtype = language_input_compute_dtype(self.dtype);
        for step in 0..options.max_new_tokens {
            context_tokens.push(current);
            generated.push(current);
            if let Some(cb) = progress_callback {
                cb(generated.len(), &generated);
            }
            if step + 1 == options.max_new_tokens {
                break;
            }
            let token_index = usize::try_from(current)
                .context("token id out of range while preparing decode embedding")?;
            let mut decode_inputs = self
                .language
                .token_embedding_for_id(token_index)
                .context("failed to gather embedding for decode token")?
                .unsqueeze(0)?
                .unsqueeze(0)?;
            decode_inputs = cast_dtype_owned(
                decode_inputs,
                decode_input_dtype,
                "failed to cast decode input embedding",
            )?;
            let decode = self.forward(
                None,
                Some(&decode_inputs),
                None,
                None,
                None,
                None,
                None,
                Some(guard.cache()),
                true,
            )?;
            let next_logits = decode
                .logits
                .get(0)
                .context("decode logits missing batch dimension")?
                .get(0)
                .context("decode logits missing timestep")?;
            current = select_token_id(&next_logits, &options, &context_tokens, &mut rng)?;

            // step=N selects token N from logits given prompt + first N tokens.
            if let (Some(cfg), Some(path)) = (
                debug_logits_config_from_env(),
                debug_logits_json_path_from_env(),
            ) {
                if cfg.step == step + 1 {
                    let info = logits_top2_at_step(cfg.step, &next_logits)?;
                    write_debug_logits_json(&path, &info, current)?;
                }
            }
            if let Some(eos) = options.eos_token_id
                && current == eos
            {
                break;
            }
        }
        let len = generated.len();
        decode_timer.finish(|event| {
            event.add_field("steps", len as u64);
            event.add_field("max_new_tokens", options.max_new_tokens as u64);
        });
        total_timer.finish(|event| {
            event.add_field("prompt_tokens", seq_len as u64);
            event.add_field("generated_tokens", len as u64);
            event.add_field("max_new_tokens", options.max_new_tokens as u64);
            event.add_field("terminated_on_prefill", false);
            event.add_field("use_cache", true);
        });
        Ok(Tensor::from_vec(generated, (1, len), self.device())?.to_dtype(DType::I64)?)
    }

    fn generate_without_cache(
        &self,
        tokenizer: &Tokenizer,
        input_ids: &Tensor,
        options: GenerateOptions<'_>,
    ) -> Result<Tensor> {
        let total_timer = Timer::new("decode.generate_no_cache");
        ensure!(
            input_ids.rank() == 2,
            "generate expects input_ids with shape [batch, seq]"
        );
        let (batch, seq_len) = input_ids.shape().dims2()?;
        ensure!(
            batch == 1,
            "generate without cache currently supports batch size 1 (got {batch})"
        );
        if options.max_new_tokens == 0 {
            total_timer.finish(|event| {
                event.add_field("prompt_tokens", seq_len as u64);
                event.add_field("generated_tokens", 0u64);
                event.add_field("max_new_tokens", 0u64);
                event.add_field("use_cache", false);
            });
            return self.empty_generation();
        }
        ensure!(
            options.position_ids.is_none(),
            "generate without cache requires position_ids to be computed internally"
        );

        let token_rows = input_ids
            .to_dtype(DType::I64)?
            .to_vec2::<i64>()
            .context("failed to extract input_ids for no-cache generation")?;
        let mut tokens = token_rows
            .into_iter()
            .next()
            .context("input_ids must have batch dimension 1")?;
        ensure!(
            tokens.len() == seq_len,
            "token vector length {} does not match seq_len {}",
            tokens.len(),
            seq_len
        );
        let mut rng = init_rng(options.seed);

        let mut attention_vec = if let Some(mask) = options.attention_mask {
            let rows = mask
                .to_dtype(DType::I64)?
                .to_vec2::<i64>()
                .context("failed to materialize attention mask for no-cache generation")?;
            let row = rows
                .into_iter()
                .next()
                .context("attention mask must have batch dimension 1")?;
            ensure!(
                row.len() == tokens.len(),
                "attention mask length {} does not match token count {}",
                row.len(),
                tokens.len()
            );
            Some(row)
        } else {
            None
        };

        let mut image_mask_vec = if let Some(mask) = options.images_seq_mask {
            let rows = mask
                .to_dtype(DType::U8)?
                .to_vec2::<u8>()
                .context("failed to materialize image mask for no-cache generation")?;
            let row = rows
                .into_iter()
                .next()
                .context("images_seq_mask must have batch dimension 1")?;
            ensure!(
                row.len() == tokens.len(),
                "images_seq_mask length {} does not match token count {}",
                row.len(),
                tokens.len()
            );
            Some(row)
        } else {
            None
        };

        let mut _owned_embeddings: Option<Vec<Tensor>> = None;
        let image_embeddings_stage_started = Instant::now();
        let image_embeddings_slice: Option<&[Tensor]> =
            if let Some(slice) = options.image_embeddings {
                info!(
                    elapsed_ms = image_embeddings_stage_started.elapsed().as_millis(),
                    image_count = slice.len(),
                    "decode no-cache stage reused: image_embeddings"
                );
                emit_stage_trace(
                    "decode.no_cache.image_embeddings.reused",
                    &[
                        (
                            "elapsed_ms",
                            image_embeddings_stage_started
                                .elapsed()
                                .as_millis()
                                .to_string(),
                        ),
                        ("image_count", slice.len().to_string()),
                    ],
                );
                Some(slice)
            } else if let Some(inputs) = options.image_inputs {
                info!(
                    elapsed_ms = image_embeddings_stage_started.elapsed().as_millis(),
                    image_count = inputs.len(),
                    "decode no-cache stage started: image_embeddings"
                );
                emit_stage_trace(
                    "decode.no_cache.image_embeddings.started",
                    &[
                        (
                            "elapsed_ms",
                            image_embeddings_stage_started
                                .elapsed()
                                .as_millis()
                                .to_string(),
                        ),
                        ("image_count", inputs.len().to_string()),
                    ],
                );
                let computed = self.compute_image_embeddings(inputs)?;
                info!(
                    elapsed_ms = image_embeddings_stage_started.elapsed().as_millis(),
                    image_count = computed.len(),
                    "decode no-cache stage completed: image_embeddings"
                );
                emit_stage_trace(
                    "decode.no_cache.image_embeddings.completed",
                    &[
                        (
                            "elapsed_ms",
                            image_embeddings_stage_started
                                .elapsed()
                                .as_millis()
                                .to_string(),
                        ),
                        ("image_count", computed.len().to_string()),
                    ],
                );
                _owned_embeddings = Some(computed);
                _owned_embeddings.as_deref()
            } else {
                info!(
                    elapsed_ms = image_embeddings_stage_started.elapsed().as_millis(),
                    "decode no-cache stage skipped: image_embeddings"
                );
                emit_stage_trace(
                    "decode.no_cache.image_embeddings.skipped",
                    &[(
                        "elapsed_ms",
                        image_embeddings_stage_started
                            .elapsed()
                            .as_millis()
                            .to_string(),
                    )],
                );
                None
            };
        let forward_image_inputs = if image_embeddings_slice.is_some() {
            None
        } else {
            options.image_inputs
        };

        let to_tensor_i64 = |data: &[i64], device: &Device| -> Result<Tensor> {
            cast_dtype_owned(
                Tensor::from_slice(data, (1, data.len()), device)?,
                DType::I64,
                "failed to cast token ids to i64",
            )
        };
        let to_tensor_u8 = |data: &[u8], device: &Device| -> Result<Tensor> {
            cast_dtype_owned(
                Tensor::from_slice(data, (1, data.len()), device)?,
                DType::U8,
                "failed to cast image mask to u8",
            )
        };

        let mut attention_tensor = match &attention_vec {
            Some(vec) => Some(to_tensor_i64(vec, self.device())?),
            None => None,
        };
        let mut image_mask_tensor = match &image_mask_vec {
            Some(vec) => Some(to_tensor_u8(vec, self.device())?),
            None => None,
        };

        let input_tensor =
            to_tensor_i64(&tokens, self.device()).context("failed to build prefill tokens")?;
        let mut forward_calls = 0u64;
        let mut max_seq_len_seen = tokens.len() as u64;
        let prefill_timer = Timer::new("decode.prefill_no_cache");
        let prefill_stage_started = Instant::now();
        info!(
            elapsed_ms = prefill_stage_started.elapsed().as_millis(),
            prompt_tokens = seq_len as u64,
            has_image_embeddings = image_embeddings_slice.is_some(),
            has_image_mask = image_mask_tensor.is_some(),
            "decode no-cache stage started: prefill"
        );
        emit_stage_trace(
            "decode.no_cache.prefill.started",
            &[
                (
                    "elapsed_ms",
                    prefill_stage_started.elapsed().as_millis().to_string(),
                ),
                ("prompt_tokens", (seq_len as u64).to_string()),
                (
                    "has_image_embeddings",
                    image_embeddings_slice.is_some().to_string(),
                ),
                ("has_image_mask", image_mask_tensor.is_some().to_string()),
            ],
        );
        let prefill = self.forward(
            Some(&input_tensor),
            None,
            attention_tensor.as_ref(),
            None,
            image_mask_tensor.as_ref(),
            forward_image_inputs,
            image_embeddings_slice,
            None,
            false,
        )?;
        info!(
            elapsed_ms = prefill_stage_started.elapsed().as_millis(),
            prompt_tokens = seq_len as u64,
            "decode no-cache stage completed: prefill"
        );
        emit_stage_trace(
            "decode.no_cache.prefill.completed",
            &[
                (
                    "elapsed_ms",
                    prefill_stage_started.elapsed().as_millis().to_string(),
                ),
                ("prompt_tokens", (seq_len as u64).to_string()),
            ],
        );
        prefill_timer.finish(|event| {
            event.add_field("prompt_tokens", seq_len as u64);
            event.add_field("final_seq", tokens.len() as u64);
            event.add_field("use_cache", false);
        });
        forward_calls += 1;
        let logits = prefill
            .logits
            .get(0)
            .context("prefill logits missing batch dimension")?
            .get(tokens.len() - 1)
            .context("prefill logits missing final timestep")?;
        let mut current = select_first_visible_token_id(
            tokenizer,
            &logits,
            &options,
            &tokens,
            options.eos_token_id,
            &mut rng,
        )?;
        if let (Some(cfg), Some(path)) = (
            debug_logits_config_from_env(),
            debug_logits_json_path_from_env(),
        ) {
            if cfg.step == 0 {
                let info = logits_top2_at_step(0, &logits)?;
                write_debug_logits_json(&path, &info, current)?;
            }
        }
        if let Some(eos) = options.eos_token_id
            && current == eos
        {
            total_timer.finish(|event| {
                event.add_field("prompt_tokens", seq_len as u64);
                event.add_field("generated_tokens", 0u64);
                event.add_field("max_new_tokens", options.max_new_tokens as u64);
                event.add_field("terminated_on_prefill", true);
                event.add_field("use_cache", false);
                event.add_field("forward_calls", forward_calls);
                event.add_field("max_seq_len_seen", max_seq_len_seen);
            });
            return self.empty_generation();
        }

        let progress_callback = options.progress_callback;
        let mut generated = Vec::with_capacity(options.max_new_tokens);
        for step in 0..options.max_new_tokens {
            generated.push(current);
            if step == 0 {
                info!(
                    generated_tokens = generated.len() as u64,
                    "decode no-cache stage completed: first_token_selection"
                );
                emit_stage_trace(
                    "decode.no_cache.first_token_selection.completed",
                    &[("generated_tokens", (generated.len() as u64).to_string())],
                );
            }
            if let Some(cb) = progress_callback {
                cb(generated.len(), &generated);
            }
            if step + 1 == options.max_new_tokens {
                break;
            }

            tokens.push(current);
            max_seq_len_seen = max_seq_len_seen.max(tokens.len() as u64);
            if let Some(mask) = image_mask_vec.as_mut() {
                mask.push(0);
            }
            if let Some(mask) = attention_vec.as_mut() {
                mask.push(1);
            }

            attention_tensor = match &attention_vec {
                Some(vec) => Some(to_tensor_i64(vec, self.device())?),
                None => None,
            };
            image_mask_tensor = match &image_mask_vec {
                Some(vec) => Some(to_tensor_u8(vec, self.device())?),
                None => None,
            };
            let input_tensor =
                to_tensor_i64(&tokens, self.device()).context("failed to build decode tokens")?;
            if step == 0 {
                info!(
                    prompt_tokens = seq_len as u64,
                    decode_tokens = tokens.len() as u64,
                    "decode no-cache stage started: first_decode_step"
                );
                emit_stage_trace(
                    "decode.no_cache.first_decode_step.started",
                    &[
                        ("prompt_tokens", (seq_len as u64).to_string()),
                        ("decode_tokens", (tokens.len() as u64).to_string()),
                    ],
                );
            }
            let forward = self.forward(
                Some(&input_tensor),
                None,
                attention_tensor.as_ref(),
                None,
                image_mask_tensor.as_ref(),
                forward_image_inputs,
                image_embeddings_slice,
                None,
                false,
            )?;
            if step == 0 {
                info!(
                    prompt_tokens = seq_len as u64,
                    decode_tokens = tokens.len() as u64,
                    "decode no-cache stage completed: first_decode_step"
                );
                emit_stage_trace(
                    "decode.no_cache.first_decode_step.completed",
                    &[
                        ("prompt_tokens", (seq_len as u64).to_string()),
                        ("decode_tokens", (tokens.len() as u64).to_string()),
                    ],
                );
            }
            let seq_pos = tokens.len() - 1;
            forward_calls += 1;
            let next_logits = forward
                .logits
                .get(0)
                .context("decode logits missing batch dimension")?
                .get(seq_pos)
                .context("decode logits missing timestep")?;
            current = select_token_id(&next_logits, &options, &tokens, &mut rng)?;
            if let (Some(cfg), Some(path)) = (
                debug_logits_config_from_env(),
                debug_logits_json_path_from_env(),
            ) {
                if cfg.step == step + 1 {
                    let info = logits_top2_at_step(cfg.step, &next_logits)?;
                    write_debug_logits_json(&path, &info, current)?;
                }
            }
            if let Some(eos) = options.eos_token_id
                && current == eos
            {
                break;
            }
        }

        let len = generated.len();
        total_timer.finish(|event| {
            event.add_field("prompt_tokens", seq_len as u64);
            event.add_field("generated_tokens", len as u64);
            event.add_field("max_new_tokens", options.max_new_tokens as u64);
            event.add_field("terminated_on_prefill", false);
            event.add_field("use_cache", false);
            event.add_field("forward_calls", forward_calls);
            event.add_field("max_seq_len_seen", max_seq_len_seen);
        });
        Ok(Tensor::from_vec(generated, (1, len), self.device())?.to_dtype(DType::I64)?)
    }

    fn empty_generation(&self) -> Result<Tensor> {
        Ok(Tensor::from_vec(Vec::<i64>::new(), (1, 0), self.device())?.to_dtype(DType::I64)?)
    }
}

fn round_ties_to_even(value: f64) -> f64 {
    let rounded = value.round();
    if (value - rounded).abs() != 0.5 {
        return rounded;
    }
    let truncated = value.trunc();
    if truncated as i64 % 2 == 0 {
        truncated
    } else {
        truncated + value.signum()
    }
}

pub fn build_global_view(image: &DynamicImage, base_size: u32) -> DynamicImage {
    let mean = (0.5 * 255.0) as u8;
    let mut canvas = RgbImage::from_pixel(base_size, base_size, Rgb([mean, mean, mean]));
    let (orig_w, orig_h) = image.dimensions();
    if orig_w == 0 || orig_h == 0 {
        return DynamicImage::ImageRgb8(canvas);
    }
    let scale = (base_size as f64 / orig_w as f64).min(base_size as f64 / orig_h as f64);
    let new_w = round_ties_to_even(orig_w as f64 * scale)
        .max(1.0)
        .min(base_size as f64) as u32;
    let new_h = round_ties_to_even(orig_h as f64 * scale)
        .max(1.0)
        .min(base_size as f64) as u32;

    let rgb_image = image.to_rgb8();
    let resized = resize_bicubic(&rgb_image, new_w, new_h);

    let x_off = round_ties_to_even((base_size as f64 - new_w as f64) * 0.5) as i64;
    let y_off = round_ties_to_even((base_size as f64 - new_h as f64) * 0.5) as i64;
    imageops::replace(&mut canvas, &resized, x_off, y_off);
    DynamicImage::ImageRgb8(canvas)
}

pub fn image_to_tensor(image: &DynamicImage, device: &Device, dtype: DType) -> Result<Tensor> {
    let rgb = image.to_rgb8();
    let (width, height) = rgb.dimensions();
    let mut data = Vec::with_capacity((width * height * 3) as usize);
    for c in 0..3 {
        for y in 0..height {
            for x in 0..width {
                let value = rgb.get_pixel(x, y)[c as usize] as f32 / 255.0;
                let normalized = (value - 0.5) / 0.5;
                data.push(normalized);
            }
        }
    }
    let tensor = Tensor::from_vec(data, (3, height as usize, width as usize), device)?;
    cast_dtype_owned(tensor, dtype, "image tensor dtype cast failed")
}

impl OcrEngine for DeepseekOcrModel {
    fn kind(&self) -> ModelKind {
        ModelKind::Deepseek
    }

    fn device(&self) -> &Device {
        self.device()
    }

    fn dtype(&self) -> DType {
        self.dtype()
    }

    fn weights_path(&self) -> Option<&Path> {
        Some(self.weights_path())
    }

    fn flash_attention_enabled(&self) -> bool {
        self.flash_attention_enabled()
    }

    fn decode(
        &self,
        tokenizer: &Tokenizer,
        prompt: &str,
        images: &[DynamicImage],
        vision: VisionSettings,
        params: &DecodeParameters,
        stream: Option<&dyn Fn(usize, &[i64])>,
    ) -> Result<DecodeOutcome> {
        let decode_stage_started = Instant::now();
        emit_stage_trace(
            "ocr_engine.decode.started",
            &[
                (
                    "elapsed_ms",
                    decode_stage_started.elapsed().as_millis().to_string(),
                ),
                ("image_count", images.len().to_string()),
                ("base_size", vision.base_size.to_string()),
                ("image_size", vision.image_size.to_string()),
                ("crop_mode", vision.crop_mode.to_string()),
                ("use_cache", params.use_cache.to_string()),
                ("max_new_tokens", params.max_new_tokens.to_string()),
            ],
        );
        emit_stage_trace(
            "ocr_engine.decode.prepare_vision_inputs.started",
            &[(
                "elapsed_ms",
                decode_stage_started.elapsed().as_millis().to_string(),
            )],
        );
        let owned_inputs = prepare_vision_inputs(
            self,
            images,
            vision.base_size,
            vision.image_size,
            vision.crop_mode,
        )
        .with_context(|| "vision input failed")?;
        emit_stage_trace(
            "ocr_engine.decode.prepare_vision_inputs.completed",
            &[
                (
                    "elapsed_ms",
                    decode_stage_started.elapsed().as_millis().to_string(),
                ),
                ("owned_inputs", owned_inputs.len().to_string()),
            ],
        );
        emit_stage_trace(
            "ocr_engine.decode.compute_image_embeddings.started",
            &[(
                "elapsed_ms",
                decode_stage_started.elapsed().as_millis().to_string(),
            )],
        );
        let embeddings = compute_image_embeddings(self, &owned_inputs)
            .with_context(|| "image embedding failed")?;
        emit_stage_trace(
            "ocr_engine.decode.compute_image_embeddings.completed",
            &[
                (
                    "elapsed_ms",
                    decode_stage_started.elapsed().as_millis().to_string(),
                ),
                ("embedding_count", embeddings.len().to_string()),
            ],
        );
        emit_stage_trace(
            "ocr_engine.decode.build_prompt_tokens.started",
            &[(
                "elapsed_ms",
                decode_stage_started.elapsed().as_millis().to_string(),
            )],
        );
        let (input_ids_vec, mask_vec) = build_prompt_tokens(
            tokenizer,
            prompt,
            &embeddings,
            &owned_inputs,
            PromptBuildOptions {
                vision,
                variant: self.variant,
            },
        )
        .with_context(|| "prompt formatting failed")?;
        let image_token_count = mask_vec.iter().filter(|&&flag| flag != 0).count();
        emit_stage_trace(
            "ocr_engine.decode.build_prompt_tokens.completed",
            &[
                (
                    "elapsed_ms",
                    decode_stage_started.elapsed().as_millis().to_string(),
                ),
                ("input_tokens", input_ids_vec.len().to_string()),
                ("image_tokens", image_token_count.to_string()),
            ],
        );

        let input_len = input_ids_vec.len();
        let device = self.device();

        emit_stage_trace(
            "ocr_engine.decode.materialize_tensors.started",
            &[(
                "elapsed_ms",
                decode_stage_started.elapsed().as_millis().to_string(),
            )],
        );
        let input_ids = cast_dtype_owned(
            Tensor::from_vec(input_ids_vec.clone(), (1, input_len), device)?,
            DType::I64,
            "failed to cast decode input_ids",
        )?;
        let mask_tensor = cast_dtype_owned(
            Tensor::from_vec(mask_vec.clone(), (1, mask_vec.len()), device)?,
            DType::U8,
            "failed to cast decode images_seq_mask",
        )?;
        emit_stage_trace(
            "ocr_engine.decode.materialize_tensors.completed",
            &[
                (
                    "elapsed_ms",
                    decode_stage_started.elapsed().as_millis().to_string(),
                ),
                ("input_tokens", input_len.to_string()),
                ("mask_tokens", mask_vec.len().to_string()),
            ],
        );

        let mut options = GenerateOptions::new(params.max_new_tokens);
        options.images_seq_mask = Some(&mask_tensor);
        if !embeddings.is_empty() {
            options.image_embeddings = Some(embeddings.as_slice());
        }
        options.eos_token_id = self.language_model().config().eos_token_id;
        options.use_cache = params.use_cache;
        options.do_sample = params.do_sample;
        options.temperature = params.temperature;
        options.top_p = params.top_p;
        options.top_k = params.top_k;
        options.repetition_penalty = params.repetition_penalty;
        options.no_repeat_ngram_size = params.no_repeat_ngram_size;
        options.seed = params.seed;
        options.progress_callback = stream;
        options.prefer_digit_first_token = prompt_requests_single_visible_digit(prompt);
        options.preferred_first_visible_text = prompt_requested_visible_first_token(prompt);

        emit_stage_trace(
            "ocr_engine.decode.generate.started",
            &[
                (
                    "elapsed_ms",
                    decode_stage_started.elapsed().as_millis().to_string(),
                ),
                ("use_cache", options.use_cache.to_string()),
                (
                    "has_image_embeddings",
                    options.image_embeddings.is_some().to_string(),
                ),
                (
                    "has_image_mask",
                    options.images_seq_mask.is_some().to_string(),
                ),
            ],
        );
        let generated = self.generate(tokenizer, &input_ids, options)?;
        emit_stage_trace(
            "ocr_engine.decode.generate.completed",
            &[
                (
                    "elapsed_ms",
                    decode_stage_started.elapsed().as_millis().to_string(),
                ),
                ("generated_rank", generated.rank().to_string()),
            ],
        );
        let generated_tokens = generated
            .to_vec2::<i64>()?
            .into_iter()
            .next()
            .unwrap_or_default();
        let decoded = tokenizer
            .decode(
                &generated_tokens
                    .iter()
                    .filter_map(|&id| u32::try_from(id).ok())
                    .collect::<Vec<_>>(),
                true,
            )
            .unwrap_or_default();
        let normalized = normalize_text(&decoded);
        trace_empty_output(&decoded, &normalized, &generated_tokens);
        emit_stage_trace(
            "ocr_engine.decode.completed",
            &[
                (
                    "elapsed_ms",
                    decode_stage_started.elapsed().as_millis().to_string(),
                ),
                ("prompt_tokens", input_len.to_string()),
                ("response_tokens", generated_tokens.len().to_string()),
            ],
        );

        Ok(DecodeOutcome {
            text: normalized,
            prompt_tokens: input_len,
            response_tokens: generated_tokens.len(),
            generated_tokens,
        })
    }
}

fn prepare_vision_inputs(
    model: &DeepseekOcrModel,
    images: &[DynamicImage],
    base_size: u32,
    image_size: u32,
    crop_mode: bool,
) -> Result<Vec<OwnedVisionInput>> {
    let timer = Timer::new("vision.prepare_inputs");
    if !images.is_empty() {
        trace!(
            "Preparing vision input (base_size={base_size}, image_size={image_size}, crop_mode={crop_mode})"
        );
    }
    let result = images
        .iter()
        .map(|image| {
            model
                .prepare_vision_input_from_image(image, base_size, image_size, crop_mode)
                .with_context(|| "failed to build vision input")
        })
        .collect::<Result<Vec<_>>>();
    match &result {
        Ok(inputs) => {
            timer.finish(|event| {
                event.add_field("images", inputs.len());
                event.add_field("base_size", base_size as u64);
                event.add_field("image_size", image_size as u64);
                event.add_field("crop_mode", crop_mode);
            });
        }
        Err(_) => {
            timer.finish(|_| {});
        }
    }
    result
}

fn compute_image_embeddings(
    model: &DeepseekOcrModel,
    owned_inputs: &[OwnedVisionInput],
) -> Result<Vec<Tensor>> {
    let timer = Timer::new("vision.compute_embeddings");
    if owned_inputs.is_empty() {
        timer.finish(|event| {
            event.add_field("images", 0u64);
        });
        return Ok(Vec::new());
    }
    let refs: Vec<Option<VisionInput<'_>>> = owned_inputs
        .iter()
        .map(|owned| Some(owned.as_ref()))
        .collect();
    trace!("Computing image embeddings for {} image(s)...", refs.len());
    let outputs = model.compute_image_embeddings(&refs);
    match &outputs {
        Ok(values) => {
            let tokens_total: u64 = values
                .iter()
                .map(|tensor| tensor.shape().dims().first().copied().unwrap_or(0) as u64)
                .sum();
            timer.finish(|event| {
                event.add_field("images", refs.len());
                event.add_field("device_is_cuda", model.device().is_cuda());
                event.add_field("device_is_metal", model.device().is_metal());
                event.add_field("token_rows_total", tokens_total);
            });
        }
        Err(_) => {
            timer.finish(|_| {});
        }
    }
    outputs
}

struct PromptBuildOptions {
    vision: VisionSettings,
    variant: OcrVariant,
}

fn build_prompt_tokens(
    tokenizer: &Tokenizer,
    prompt: &str,
    embeddings: &[Tensor],
    vision_inputs: &[OwnedVisionInput],
    options: PromptBuildOptions,
) -> Result<(Vec<i64>, Vec<u8>)> {
    let timer = Timer::new("prompt.build_tokens");
    let image_token_id = tokenizer
        .token_to_id("<image>")
        .ok_or_else(|| anyhow!("tokenizer missing <image> token"))? as i64;
    let bos_id = 0i64;

    let segments: Vec<&str> = prompt.split("<image>").collect();
    anyhow::ensure!(
        segments.len().saturating_sub(1) == embeddings.len(),
        "prompt/image embedding mismatch: {} slots vs {} embeddings",
        segments.len().saturating_sub(1),
        embeddings.len()
    );
    anyhow::ensure!(
        embeddings.len() == vision_inputs.len(),
        "vision input count {} does not match embeddings {}",
        vision_inputs.len(),
        embeddings.len()
    );

    let mut tokens = Vec::new();
    let mut mask = Vec::new();
    tokens.push(bos_id);
    mask.push(0);

    for (idx, segment) in segments.iter().enumerate() {
        let encoding = tokenizer
            .encode(*segment, false)
            .map_err(|err| anyhow!("tokenization failed: {err}"))?;
        tokens.extend(encoding.get_ids().iter().map(|&id| id as i64));
        mask.extend(std::iter::repeat_n(0u8, encoding.len()));
        if idx < embeddings.len() {
            let placeholders = build_image_placeholders(
                image_token_id,
                &vision_inputs[idx],
                embeddings[idx]
                    .shape()
                    .dims2()
                    .context("vision embedding must be 2D")?
                    .0,
                options.vision.base_size,
                options.vision.image_size,
                options.vision.crop_mode,
                options.variant,
            )?;
            tokens.extend(&placeholders);
            mask.extend(std::iter::repeat_n(1u8, placeholders.len()));
        }
    }

    let total_tokens = tokens.len();
    let image_tokens = mask.iter().filter(|&&flag| flag != 0).count();
    timer.finish(|event| {
        event.add_field("tokens", total_tokens);
        event.add_field("image_tokens", image_tokens);
        event.add_field("segments", segments.len());
        event.add_field("crop_mode", options.vision.crop_mode);
    });

    Ok((tokens, mask))
}

fn build_image_placeholders(
    image_token_id: i64,
    input: &OwnedVisionInput,
    expected_tokens: usize,
    base_size: u32,
    image_size: u32,
    crop_mode: bool,
    variant: OcrVariant,
) -> Result<Vec<i64>> {
    const PATCH_SIZE: u32 = 16;
    const DOWNSAMPLE_RATIO: u32 = 4;

    let mut placeholders = Vec::new();

    let push_grid_with_row_breaks =
        |placeholders: &mut Vec<i64>, rows: usize, cols: usize, add_terminal: bool| {
            for _ in 0..rows {
                placeholders.extend(std::iter::repeat_n(image_token_id, cols));
                placeholders.push(image_token_id);
            }
            if add_terminal {
                placeholders.push(image_token_id);
            }
        };

    let push_grid_flat = |placeholders: &mut Vec<i64>, rows: usize, cols: usize| {
        placeholders.extend(std::iter::repeat_n(image_token_id, rows * cols));
    };

    if crop_mode {
        let global_grid = (base_size / PATCH_SIZE) as usize;
        let num_queries_global = ((global_grid as f32) / (DOWNSAMPLE_RATIO as f32)).ceil() as usize;
        let local_grid = (image_size / PATCH_SIZE) as usize;
        let num_queries_local = ((local_grid as f32) / (DOWNSAMPLE_RATIO as f32)).ceil() as usize;
        let (width_crops, height_crops) = input.crop_shape.unwrap_or((1, 1));

        if width_crops > 1 || height_crops > 1 {
            let rows = num_queries_local * height_crops;
            let cols = num_queries_local * width_crops;
            match variant {
                OcrVariant::Ocr1 => {
                    push_grid_with_row_breaks(&mut placeholders, rows, cols, false);
                }
                OcrVariant::Ocr2 => {
                    push_grid_flat(&mut placeholders, rows, cols);
                }
            }
        }

        match variant {
            OcrVariant::Ocr1 => {
                push_grid_with_row_breaks(
                    &mut placeholders,
                    num_queries_global,
                    num_queries_global,
                    true,
                );
            }
            OcrVariant::Ocr2 => {
                push_grid_flat(&mut placeholders, num_queries_global, num_queries_global);
                placeholders.push(image_token_id);
            }
        }
    } else {
        let grid = (image_size / PATCH_SIZE) as usize;
        let num_queries = ((grid as f32) / (DOWNSAMPLE_RATIO as f32)).ceil() as usize;
        match variant {
            OcrVariant::Ocr1 => {
                push_grid_with_row_breaks(&mut placeholders, num_queries, num_queries, true);
            }
            OcrVariant::Ocr2 => {
                push_grid_flat(&mut placeholders, num_queries, num_queries);
                placeholders.push(image_token_id);
            }
        }
    }

    anyhow::ensure!(
        placeholders.len() == expected_tokens,
        "placeholder count {} does not match expected {}",
        placeholders.len(),
        expected_tokens
    );
    Ok(placeholders)
}

fn detect_ocr_variant(cfg: &DeepseekOcrConfig) -> OcrVariant {
    if cfg
        .vision_config
        .as_ref()
        .and_then(|v| v.model_name.as_deref())
        .map(|name| name.eq_ignore_ascii_case("deepencoderv2"))
        .unwrap_or(false)
    {
        return OcrVariant::Ocr2;
    }
    if cfg
        .vision_config
        .as_ref()
        .and_then(|v| v.width.get("qwen2-0-5b"))
        .is_some()
    {
        return OcrVariant::Ocr2;
    }
    OcrVariant::Ocr1
}

#[cfg(test)]
mod tests {
    use super::{
        FirstTokenCandidateClass, LowPrecisionLoadPolicy, LowPrecisionLoadPolicyGuard,
        current_low_precision_load_policy, decoded_first_token_is_single_digit,
        decoded_first_token_is_visible, empty_output_trace_enabled, language_input_compute_dtype,
        prompt_requested_visible_first_token, prompt_requests_single_visible_digit,
        select_first_visible_token_id_from_logits_with, stage_trace_enabled,
    };
    use candle_core::DType;
    use deepseek_ocr_core::sampling::{TokenSelectionParams, init_rng};
    use std::sync::{Mutex, OnceLock};

    #[derive(Clone, Copy)]
    struct TestSelectionParams;

    impl TokenSelectionParams for TestSelectionParams {
        fn do_sample(&self) -> bool {
            false
        }

        fn temperature(&self) -> f64 {
            0.0
        }

        fn top_p(&self) -> Option<f64> {
            None
        }

        fn top_k(&self) -> Option<usize> {
            None
        }

        fn repetition_penalty(&self) -> f32 {
            1.0
        }

        fn no_repeat_ngram_size(&self) -> Option<usize> {
            None
        }
    }

    fn env_lock() -> &'static Mutex<()> {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn low_precision_load_policy_defaults_to_upstream_parity() {
        assert_eq!(
            LowPrecisionLoadPolicy::default(),
            LowPrecisionLoadPolicy {
                preload_language_f32_aux: true,
                preload_vision_f32_aux: true,
                preload_linear_weight_f32: true,
                promote_language_input_f32: true,
                lazy_moe_experts: false,
                lazy_clip_transformer_layers: false,
            }
        );
    }

    #[test]
    fn low_precision_load_policy_guard_restores_previous_policy() {
        let original = current_low_precision_load_policy();
        {
            let _guard = LowPrecisionLoadPolicyGuard::enter(LowPrecisionLoadPolicy {
                preload_language_f32_aux: false,
                preload_vision_f32_aux: false,
                preload_linear_weight_f32: false,
                promote_language_input_f32: false,
                lazy_moe_experts: true,
                lazy_clip_transformer_layers: true,
            });
            assert_eq!(
                current_low_precision_load_policy(),
                LowPrecisionLoadPolicy {
                    preload_language_f32_aux: false,
                    preload_vision_f32_aux: false,
                    preload_linear_weight_f32: false,
                    promote_language_input_f32: false,
                    lazy_moe_experts: true,
                    lazy_clip_transformer_layers: true,
                }
            );
        }
        assert_eq!(current_low_precision_load_policy(), original);
    }

    #[test]
    fn language_input_dtype_follows_policy() {
        assert_eq!(language_input_compute_dtype(DType::F16), DType::F32);
        let _guard = LowPrecisionLoadPolicyGuard::enter(LowPrecisionLoadPolicy {
            preload_language_f32_aux: true,
            preload_vision_f32_aux: true,
            preload_linear_weight_f32: true,
            promote_language_input_f32: false,
            lazy_moe_experts: false,
            lazy_clip_transformer_layers: false,
        });
        assert_eq!(language_input_compute_dtype(DType::F16), DType::F16);
        assert_eq!(language_input_compute_dtype(DType::BF16), DType::BF16);
    }

    #[test]
    fn empty_output_trace_flag_parses_truthy_and_falsy_values() {
        let _guard = env_lock().lock().expect("env lock poisoned");
        // SAFETY: tests serialize access to this process-wide env var through env_lock().
        unsafe {
            std::env::remove_var("XIUXIAN_VISION_TRACE_EMPTY_OUTPUT");
        }
        assert!(!empty_output_trace_enabled());

        // SAFETY: tests serialize access to this process-wide env var through env_lock().
        unsafe {
            std::env::set_var("XIUXIAN_VISION_TRACE_EMPTY_OUTPUT", "yes");
        }
        assert!(empty_output_trace_enabled());

        // SAFETY: tests serialize access to this process-wide env var through env_lock().
        unsafe {
            std::env::set_var("XIUXIAN_VISION_TRACE_EMPTY_OUTPUT", "off");
        }
        assert!(!empty_output_trace_enabled());

        // SAFETY: tests serialize access to this process-wide env var through env_lock().
        unsafe {
            std::env::remove_var("XIUXIAN_VISION_TRACE_EMPTY_OUTPUT");
        }
    }

    #[test]
    fn stage_trace_flag_parses_truthy_and_falsy_values() {
        let _guard = env_lock().lock().expect("env lock poisoned");
        // SAFETY: tests serialize access to this process-wide env var through env_lock().
        unsafe {
            std::env::remove_var("XIUXIAN_VISION_STAGE_TRACE_STDERR");
        }
        assert!(!stage_trace_enabled());

        // SAFETY: tests serialize access to this process-wide env var through env_lock().
        unsafe {
            std::env::set_var("XIUXIAN_VISION_STAGE_TRACE_STDERR", "yes");
        }
        assert!(stage_trace_enabled());

        // SAFETY: tests serialize access to this process-wide env var through env_lock().
        unsafe {
            std::env::set_var("XIUXIAN_VISION_STAGE_TRACE_STDERR", "off");
        }
        assert!(!stage_trace_enabled());

        // SAFETY: tests serialize access to this process-wide env var through env_lock().
        unsafe {
            std::env::remove_var("XIUXIAN_VISION_STAGE_TRACE_STDERR");
        }
    }

    #[test]
    fn decoded_first_token_visibility_treats_whitespace_as_invisible() {
        assert!(!decoded_first_token_is_visible("  \n\n"));
        assert!(decoded_first_token_is_visible("A"));
    }

    #[test]
    fn decoded_first_token_single_digit_detection_matches_ascii_digits() {
        assert!(decoded_first_token_is_single_digit("2"));
        assert!(decoded_first_token_is_single_digit("  7  "));
        assert!(!decoded_first_token_is_single_digit("Hello"));
        assert!(!decoded_first_token_is_single_digit("12"));
        assert!(!decoded_first_token_is_single_digit("  \n\n"));
    }

    #[test]
    fn prompt_digit_detection_matches_canonical_smoke_prompt() {
        assert!(prompt_requests_single_visible_digit(
            "<image>\n<|grounding|>Return exactly one visible digit from the image. No markdown. No explanation."
        ));
        assert!(!prompt_requests_single_visible_digit(
            "<image>\nSummarize the receipt."
        ));
    }

    #[test]
    fn prompt_visible_token_detection_extracts_word_or_phrase_prefix() {
        assert_eq!(
            prompt_requested_visible_first_token(
                "<image>\n<|grounding|>Return only the visible word Telegram from the image. No label. No markdown. No explanation."
            )
            .as_deref(),
            Some("telegram")
        );
        assert_eq!(
            prompt_requested_visible_first_token(
                "<image>\n<|grounding|>Return only the visible phrase Telegram OCR from the image. No label. No markdown. No explanation."
            )
            .as_deref(),
            Some("telegram")
        );
        assert_eq!(
            prompt_requested_visible_first_token(
                "<image>\n<|grounding|>Return only the visible phrase 2026-03-09-001 from the image. No label. No markdown. No explanation."
            )
            .as_deref(),
            Some("2026")
        );
        assert_eq!(
            prompt_requested_visible_first_token(
                "<image>\n<|grounding|>Return only the visible phrase $128.50 from the image. No label. No markdown. No explanation."
            )
            .as_deref(),
            Some("128")
        );
        assert_eq!(
            prompt_requested_visible_first_token(
                "<image>\n<|grounding|>Return exactly one visible digit from the image. No markdown. No explanation."
            ),
            None
        );
        assert_eq!(
            prompt_requested_visible_first_token(
                "<image>\n<|grounding|>Return only the visible word managed sidecar from the image."
            ),
            None
        );
    }

    #[test]
    fn first_visible_token_selection_skips_invisible_argmax_candidate() {
        let params = TestSelectionParams;
        let mut rng = init_rng(Some(7));
        let token = select_first_visible_token_id_from_logits_with(
            &[9.0, 8.5, 1.0],
            &params,
            &[],
            None,
            &mut rng,
            |token_id| {
                if token_id == 0 {
                    FirstTokenCandidateClass::Invisible
                } else {
                    FirstTokenCandidateClass::Preferred
                }
            },
        )
        .expect("selection should succeed");
        assert_eq!(token, 1);
    }

    #[test]
    fn first_visible_token_selection_falls_back_when_all_candidates_are_invisible() {
        let params = TestSelectionParams;
        let mut rng = init_rng(Some(7));
        let token = select_first_visible_token_id_from_logits_with(
            &[9.0, 8.5, 1.0],
            &params,
            &[],
            None,
            &mut rng,
            |_| FirstTokenCandidateClass::Invisible,
        )
        .expect("selection should succeed");
        assert_eq!(token, 0);
    }

    #[test]
    fn first_visible_token_selection_defers_eos_until_visible_candidate_is_exhausted() {
        let params = TestSelectionParams;
        let mut rng = init_rng(Some(7));
        let token = select_first_visible_token_id_from_logits_with(
            &[9.0, 8.5, 8.0],
            &params,
            &[],
            Some(0),
            &mut rng,
            |token_id| {
                if token_id == 2 {
                    FirstTokenCandidateClass::Preferred
                } else {
                    FirstTokenCandidateClass::Invisible
                }
            },
        )
        .expect("selection should succeed");
        assert_eq!(token, 2);
    }

    #[test]
    fn first_visible_token_selection_returns_eos_when_only_eos_remains() {
        let params = TestSelectionParams;
        let mut rng = init_rng(Some(7));
        let token = select_first_visible_token_id_from_logits_with(
            &[9.0, 8.5, 8.0],
            &params,
            &[],
            Some(0),
            &mut rng,
            |_| FirstTokenCandidateClass::Invisible,
        )
        .expect("selection should succeed");
        assert_eq!(token, 0);
    }

    #[test]
    fn first_visible_token_selection_prefers_digit_candidate_over_higher_visible_word() {
        let params = TestSelectionParams;
        let mut rng = init_rng(Some(7));
        let token = select_first_visible_token_id_from_logits_with(
            &[20.0, 19.5, 19.0],
            &params,
            &[],
            Some(1),
            &mut rng,
            |token_id| match token_id {
                0 => FirstTokenCandidateClass::Visible,
                1 => FirstTokenCandidateClass::Invisible,
                2 => FirstTokenCandidateClass::Preferred,
                _ => FirstTokenCandidateClass::Invisible,
            },
        )
        .expect("selection should succeed");
        assert_eq!(token, 2);
    }

    #[test]
    fn first_visible_token_selection_prefers_requested_word_candidate_over_other_visible_text() {
        let params = TestSelectionParams;
        let mut rng = init_rng(Some(7));
        let token = select_first_visible_token_id_from_logits_with(
            &[20.0, 19.5, 19.0],
            &params,
            &[],
            Some(1),
            &mut rng,
            |token_id| match token_id {
                0 => FirstTokenCandidateClass::Visible,
                1 => FirstTokenCandidateClass::Invisible,
                2 => FirstTokenCandidateClass::Preferred,
                _ => FirstTokenCandidateClass::Invisible,
            },
        )
        .expect("selection should succeed");
        assert_eq!(token, 2);
    }

    #[test]
    fn first_visible_token_selection_falls_back_to_best_visible_when_no_digit_exists() {
        let params = TestSelectionParams;
        let mut rng = init_rng(Some(7));
        let token = select_first_visible_token_id_from_logits_with(
            &[20.0, 19.5, 19.0],
            &params,
            &[],
            Some(1),
            &mut rng,
            |token_id| match token_id {
                0 => FirstTokenCandidateClass::Visible,
                1 => FirstTokenCandidateClass::Invisible,
                2 => FirstTokenCandidateClass::Visible,
                _ => FirstTokenCandidateClass::Invisible,
            },
        )
        .expect("selection should succeed");
        assert_eq!(token, 0);
    }
}
