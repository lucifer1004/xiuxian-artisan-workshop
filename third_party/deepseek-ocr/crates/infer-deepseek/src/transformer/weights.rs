use std::{
    env,
    fmt::{self, Write as _},
    io::Write as _,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Instant,
};

use crate::{
    config::DeepseekV2Config,
    model::{
        LowPrecisionLoadPolicy, current_low_precision_load_policy, with_low_precision_load_policy,
    },
    quant_snapshot::{
        LinearSpec, QuantizedSnapshot, SnapshotLinear, SnapshotLinearMap, SnapshotLoadPlan,
    },
    quantization::{
        LinearLayerGroup, QuantModule, QuantizationOutcome, QuantizationState, backend_label,
    },
};
use anyhow::{Context, Result, ensure};
use candle_core::{DType, Device, Tensor, quantized::QMatMul};
use candle_nn::VarBuilder;
use tracing::{info, trace};

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
    let _ = std::io::stderr().flush();
}

fn skip_shared_experts_enabled() -> bool {
    env::var("XIUXIAN_VISION_SKIP_SHARED_EXPERTS")
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinearWeightMaterialization {
    DefaultContiguous,
    KeepViewForMetalFastPack,
}

/// Fully connected layer weights captured directly from safetensors via [`VarBuilder`].
#[derive(Clone)]
pub struct LinearWeights {
    pub weight: Option<Tensor>,
    pub weight_f32: Option<Tensor>,
    pub bias: Option<Tensor>,
    pub qmatmul: Option<Arc<QMatMul>>,
    pub out_dim: usize,
    pub in_dim: usize,
    pub label: String,
}

impl fmt::Debug for LinearWeights {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LinearWeights")
            .field("has_weight", &self.weight.is_some())
            .field("bias", &self.bias)
            .field("qmatmul", &self.qmatmul.is_some())
            .field("out_dim", &self.out_dim)
            .field("in_dim", &self.in_dim)
            .field("label", &self.label)
            .finish()
    }
}

impl LinearWeights {
    fn snapshot_spec(vb: &VarBuilder, out_dim: usize, in_dim: usize) -> LinearSpec {
        LinearSpec::new(qualified_name(vb, "weight"), out_dim, in_dim)
    }

    #[allow(clippy::too_many_arguments)]
    fn load(
        vb: &VarBuilder,
        out_dim: usize,
        in_dim: usize,
        bias: bool,
        materialization: LinearWeightMaterialization,
        group: LinearLayerGroup,
        module: QuantModule,
        snapshot_hits: Option<&mut SnapshotLinearMap>,
        snapshot_label: Option<&'static str>,
    ) -> Result<Self> {
        let load_started_at = Instant::now();
        let label = qualified_name(vb, "weight");
        let has_bias = bias && vb.contains_tensor("bias");
        emit_stage_trace(
            "deepseek.language.linear.weight.get.start",
            &[
                (
                    "elapsed_ms",
                    load_started_at.elapsed().as_millis().to_string(),
                ),
                ("label", label.clone()),
                ("out_dim", out_dim.to_string()),
                ("in_dim", in_dim.to_string()),
            ],
        );
        let weight_init = vb
            .get((out_dim, in_dim), "weight")
            .with_context(|| format!("missing linear weight `{label}`"))?;
        emit_stage_trace(
            "deepseek.language.linear.weight.get.completed",
            &[
                (
                    "elapsed_ms",
                    load_started_at.elapsed().as_millis().to_string(),
                ),
                ("label", label.clone()),
            ],
        );
        let weight_init = if matches!(
            materialization,
            LinearWeightMaterialization::DefaultContiguous
        ) || has_bias
        {
            let weight_init = weight_init.contiguous()?;
            emit_stage_trace(
                "deepseek.language.linear.weight.contiguous.completed",
                &[
                    (
                        "elapsed_ms",
                        load_started_at.elapsed().as_millis().to_string(),
                    ),
                    ("label", label.clone()),
                ],
            );
            weight_init
        } else {
            weight_init
        };
        let mut weight = Some(weight_init.clone());
        let load_policy = current_low_precision_load_policy();
        let weight_f32 = if load_policy.preload_linear_weight_f32
            && matches!(weight_init.dtype(), DType::F16 | DType::BF16)
        {
            let weight_f32 = weight_init.to_dtype(DType::F32)?.contiguous()?;
            emit_stage_trace(
                "deepseek.language.linear.weight_f32.completed",
                &[
                    (
                        "elapsed_ms",
                        load_started_at.elapsed().as_millis().to_string(),
                    ),
                    ("label", label.clone()),
                ],
            );
            Some(weight_f32)
        } else {
            None
        };
        let mut bias_tensor: Option<Tensor> = None;
        let device = vb.device();
        let quant = QuantizationState::global();
        let mut qmatmul = None;
        // If snapshot hits were preloaded, prefer them regardless of env quant targets/kind.
        if let Some(hits) = snapshot_hits
            && let Some(hit) = hits.remove(&label)
        {
            let container = snapshot_label.unwrap_or("snapshot");
            match hit {
                SnapshotLinear::Quantized { qmatmul: qm, bias } => {
                    let path = if device.is_cuda() || device.is_metal() {
                        "kernel_upcast"
                    } else {
                        "kernel"
                    };
                    trace!(
                        tensor = label,
                        ?group,
                        in_dim,
                        out_dim = out_dim,
                        backend = backend_label(device),
                        path,
                        container,
                        source = "snapshot",
                        action = "quantized",
                        "quant-linear"
                    );
                    quant.record_attempt(module, QuantizationOutcome::Quantized);
                    bias_tensor = bias;
                    qmatmul = Some(qm);
                    weight = None;
                }
                SnapshotLinear::Float {
                    weight: snapshot_weight,
                    bias,
                } => {
                    trace!(
                        tensor = label,
                        ?group,
                        in_dim,
                        out_dim = out_dim,
                        backend = backend_label(device),
                        path = "snapshot-float",
                        container,
                        source = "snapshot",
                        action = "float",
                        "quant-linear"
                    );
                    quant.record_attempt(module, QuantizationOutcome::Fallback);
                    bias_tensor = bias;
                    weight = Some(snapshot_weight);
                }
            }
        }
        // No runtime quantization fallback: use snapshot when available, otherwise float weights.
        if bias && bias_tensor.is_none() && vb.contains_tensor("bias") {
            bias_tensor = Some(
                vb.get(out_dim, "bias")
                    .with_context(|| {
                        format!("missing linear bias `{}`", qualified_name(vb, "bias"))
                    })?
                    .contiguous()?,
            );
            emit_stage_trace(
                "deepseek.language.linear.bias.completed",
                &[
                    (
                        "elapsed_ms",
                        load_started_at.elapsed().as_millis().to_string(),
                    ),
                    ("label", qualified_name(vb, "bias")),
                ],
            );
        }
        Ok(Self {
            weight,
            weight_f32,
            bias: bias_tensor,
            qmatmul,
            out_dim,
            in_dim,
            label,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RmsNormWeights {
    pub weight: Tensor,
}

impl RmsNormWeights {
    fn load(vb: &VarBuilder, hidden_size: usize) -> Result<Self> {
        let weight = vb.get(hidden_size, "weight").with_context(|| {
            format!("missing rmsnorm weight `{}`", qualified_name(vb, "weight"))
        })?;
        Ok(Self { weight })
    }
}

#[derive(Debug, Clone)]
pub struct AttentionWeights {
    pub q_proj: LinearWeights,
    pub k_proj: LinearWeights,
    pub v_proj: LinearWeights,
    pub o_proj: LinearWeights,
}

impl AttentionWeights {
    fn load(
        cfg: &DeepseekV2Config,
        layer_idx: usize,
        vb: &VarBuilder,
        snapshot: Option<&QuantizedSnapshot>,
    ) -> Result<Self> {
        let load_started_at = Instant::now();
        let hidden_size = cfg.hidden_size;
        let num_heads = cfg.num_attention_heads;
        ensure!(
            hidden_size.is_multiple_of(num_heads),
            "hidden_size {hidden_size} not divisible by num_attention_heads {num_heads}"
        );
        let head_dim = hidden_size / num_heads;
        let num_kv_heads = cfg.num_key_value_heads.unwrap_or(num_heads);
        let kv_head_dim = head_dim;
        let v_head_dim = non_zero_or(cfg.v_head_dim, head_dim);
        let attn_vb = vb.pp("self_attn");
        let q_vb = attn_vb.pp("q_proj");
        let k_vb = attn_vb.pp("k_proj");
        let v_vb = attn_vb.pp("v_proj");
        let o_vb = attn_vb.pp("o_proj");
        let mut plan = SnapshotLoadPlan::default();
        plan.push(LinearWeights::snapshot_spec(
            &q_vb,
            num_heads * head_dim,
            hidden_size,
        ));
        plan.push(LinearWeights::snapshot_spec(
            &k_vb,
            num_kv_heads * kv_head_dim,
            hidden_size,
        ));
        plan.push(LinearWeights::snapshot_spec(
            &v_vb,
            num_kv_heads * v_head_dim,
            hidden_size,
        ));
        plan.push(LinearWeights::snapshot_spec(
            &o_vb,
            hidden_size,
            num_heads * v_head_dim,
        ));
        let mut snapshot_hits = plan.execute(snapshot, vb.device(), None)?;
        let snapshot_label = snapshot.map(|s| s.container_label());

        let q_proj = LinearWeights::load(
            &q_vb,
            num_heads * head_dim,
            hidden_size,
            true,
            LinearWeightMaterialization::DefaultContiguous,
            LinearLayerGroup::Text,
            QuantModule::TextLinear,
            snapshot_hits.as_mut(),
            snapshot_label,
        )?;
        emit_stage_trace(
            "deepseek.language.transformer_layer.attention.q_proj.completed",
            &[
                (
                    "elapsed_ms",
                    load_started_at.elapsed().as_millis().to_string(),
                ),
                ("layer_idx", layer_idx.to_string()),
            ],
        );
        let k_proj = LinearWeights::load(
            &k_vb,
            num_kv_heads * kv_head_dim,
            hidden_size,
            true,
            LinearWeightMaterialization::DefaultContiguous,
            LinearLayerGroup::Text,
            QuantModule::TextLinear,
            snapshot_hits.as_mut(),
            snapshot_label,
        )?;
        emit_stage_trace(
            "deepseek.language.transformer_layer.attention.k_proj.completed",
            &[
                (
                    "elapsed_ms",
                    load_started_at.elapsed().as_millis().to_string(),
                ),
                ("layer_idx", layer_idx.to_string()),
            ],
        );
        let v_proj = LinearWeights::load(
            &v_vb,
            num_kv_heads * v_head_dim,
            hidden_size,
            true,
            LinearWeightMaterialization::DefaultContiguous,
            LinearLayerGroup::Text,
            QuantModule::TextLinear,
            snapshot_hits.as_mut(),
            snapshot_label,
        )?;
        emit_stage_trace(
            "deepseek.language.transformer_layer.attention.v_proj.completed",
            &[
                (
                    "elapsed_ms",
                    load_started_at.elapsed().as_millis().to_string(),
                ),
                ("layer_idx", layer_idx.to_string()),
            ],
        );
        let o_proj = LinearWeights::load(
            &o_vb,
            hidden_size,
            num_heads * v_head_dim,
            true,
            LinearWeightMaterialization::DefaultContiguous,
            LinearLayerGroup::Text,
            QuantModule::TextLinear,
            snapshot_hits.as_mut(),
            snapshot_label,
        )?;
        emit_stage_trace(
            "deepseek.language.transformer_layer.attention.o_proj.completed",
            &[
                (
                    "elapsed_ms",
                    load_started_at.elapsed().as_millis().to_string(),
                ),
                ("layer_idx", layer_idx.to_string()),
            ],
        );
        Ok(Self {
            q_proj,
            k_proj,
            v_proj,
            o_proj,
        })
    }
}

#[derive(Debug, Clone)]
pub struct DenseMlpWeights {
    pub gate_proj: LinearWeights,
    pub up_proj: LinearWeights,
    pub down_proj: LinearWeights,
}

impl DenseMlpWeights {
    fn load(
        vb: &VarBuilder,
        hidden_size: usize,
        intermediate_size: usize,
        snapshot: Option<&QuantizedSnapshot>,
        materialization: LinearWeightMaterialization,
    ) -> Result<Self> {
        let gate_vb = vb.pp("gate_proj");
        let up_vb = vb.pp("up_proj");
        let down_vb = vb.pp("down_proj");
        let mut plan = SnapshotLoadPlan::default();
        plan.push(LinearWeights::snapshot_spec(
            &gate_vb,
            intermediate_size,
            hidden_size,
        ));
        plan.push(LinearWeights::snapshot_spec(
            &up_vb,
            intermediate_size,
            hidden_size,
        ));
        plan.push(LinearWeights::snapshot_spec(
            &down_vb,
            hidden_size,
            intermediate_size,
        ));
        let mut snapshot_hits = plan.execute(snapshot, vb.device(), None)?;
        let snapshot_label = snapshot.map(|s| s.container_label());

        let gate_proj = LinearWeights::load(
            &gate_vb,
            intermediate_size,
            hidden_size,
            true,
            materialization,
            LinearLayerGroup::Text,
            QuantModule::TextLinear,
            snapshot_hits.as_mut(),
            snapshot_label,
        )?;
        let up_proj = LinearWeights::load(
            &up_vb,
            intermediate_size,
            hidden_size,
            true,
            materialization,
            LinearLayerGroup::Text,
            QuantModule::TextLinear,
            snapshot_hits.as_mut(),
            snapshot_label,
        )?;
        let down_proj = LinearWeights::load(
            &down_vb,
            hidden_size,
            intermediate_size,
            true,
            materialization,
            LinearLayerGroup::Text,
            QuantModule::TextLinear,
            snapshot_hits.as_mut(),
            snapshot_label,
        )?;
        Ok(Self {
            gate_proj,
            up_proj,
            down_proj,
        })
    }
}

fn load_shared_experts(
    layer_idx: usize,
    cfg: &DeepseekV2Config,
    vb: &VarBuilder,
    hidden_size: usize,
    moe_intermediate_size: usize,
    snapshot: Option<&QuantizedSnapshot>,
    load_started_at: &Instant,
) -> Result<Option<DenseMlpWeights>> {
    let Some(count) = cfg.n_shared_experts.filter(|c| *c > 0) else {
        return Ok(None);
    };
    if skip_shared_experts_enabled() {
        emit_stage_trace(
            "deepseek.language.transformer_layer.mlp.moe.shared_experts.skipped",
            &[
                (
                    "elapsed_ms",
                    load_started_at.elapsed().as_millis().to_string(),
                ),
                ("layer_idx", layer_idx.to_string()),
                ("shared_expert_count", count.to_string()),
                ("reason", "env_override".to_string()),
            ],
        );
        info!(
            elapsed_ms = load_started_at.elapsed().as_millis(),
            layer_idx,
            shared_expert_count = count,
            "deepseek language load stage completed: moe_shared_experts_skipped"
        );
        return Ok(None);
    }

    let vb = vb.pp("shared_experts");
    let intermediate = moe_intermediate_size * count;
    emit_stage_trace(
        "deepseek.language.transformer_layer.mlp.moe.shared_experts.start",
        &[
            (
                "elapsed_ms",
                load_started_at.elapsed().as_millis().to_string(),
            ),
            ("layer_idx", layer_idx.to_string()),
            ("shared_expert_count", count.to_string()),
        ],
    );
    info!(
        elapsed_ms = load_started_at.elapsed().as_millis(),
        layer_idx,
        shared_expert_count = count,
        "deepseek language load stage completed: moe_shared_experts_start"
    );
    Ok(Some(
        DenseMlpWeights::load(
            &vb,
            hidden_size,
            intermediate,
            snapshot,
            LinearWeightMaterialization::DefaultContiguous,
        )
        .with_context(|| format!("failed to load shared_experts for layer {layer_idx}"))?,
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoeExecutionBackend {
    Slow,
    MetalFast,
}

impl MoeExecutionBackend {
    pub fn label(self) -> &'static str {
        match self {
            Self::Slow => "slow",
            Self::MetalFast => "metal_fast",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "slow" => Some(Self::Slow),
            "metal_fast" | "metal-fast" => Some(Self::MetalFast),
            _ => None,
        }
    }

    fn from_env() -> Option<Self> {
        env::var("XIUXIAN_VISION_MOE_BACKEND")
            .ok()
            .as_deref()
            .and_then(Self::parse)
    }

    pub fn select(_device: &Device) -> Self {
        Self::from_env().unwrap_or(Self::Slow)
    }
}

#[derive(Debug, Clone)]
pub struct DeferredMoeLoadSource {
    inner: Arc<DeferredMoeLoadSourceInner>,
}

#[derive(Debug)]
struct DeferredMoeLoadSourceInner {
    weights_paths: Vec<PathBuf>,
    device: Device,
    dtype: DType,
    load_policy: LowPrecisionLoadPolicy,
}

impl DeferredMoeLoadSource {
    pub fn new(
        weights_paths: Vec<PathBuf>,
        dtype: DType,
        device: &Device,
        load_policy: LowPrecisionLoadPolicy,
    ) -> Self {
        Self {
            inner: Arc::new(DeferredMoeLoadSourceInner {
                weights_paths,
                device: device.clone(),
                dtype,
                load_policy,
            }),
        }
    }

    fn build_var_builder(&self) -> Result<VarBuilder<'static>> {
        unsafe {
            VarBuilder::from_mmaped_safetensors(
                self.inner.weights_paths.as_slice(),
                self.inner.dtype,
                &self.inner.device,
            )
        }
        .context("failed to rebuild var builder for deferred MoE expert")
    }

    #[cfg(test)]
    fn shares_state_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

#[derive(Clone)]
pub struct DeferredDenseMlpWeights {
    source: DeferredMoeLoadSource,
    prefix: String,
    hidden_size: usize,
    intermediate_size: usize,
    layer_idx: usize,
    expert_idx: usize,
    materialization: LinearWeightMaterialization,
    cache_policy: DeferredExpertCachePolicy,
    cache: Arc<Mutex<Option<DenseMlpWeights>>>,
}

impl fmt::Debug for DeferredDenseMlpWeights {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let loaded = self
            .cache
            .lock()
            .map(|slot| slot.is_some())
            .unwrap_or(false);
        f.debug_struct("DeferredDenseMlpWeights")
            .field("hidden_size", &self.hidden_size)
            .field("intermediate_size", &self.intermediate_size)
            .field("layer_idx", &self.layer_idx)
            .field("expert_idx", &self.expert_idx)
            .field("materialization", &self.materialization)
            .field("cache_policy", &self.cache_policy)
            .field("loaded", &loaded)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeferredExpertCachePolicy {
    Retain,
    Ephemeral,
}

impl DeferredExpertCachePolicy {
    fn label(self) -> &'static str {
        match self {
            Self::Retain => "retain",
            Self::Ephemeral => "ephemeral",
        }
    }
}

impl DeferredDenseMlpWeights {
    fn new_expert(
        source: DeferredMoeLoadSource,
        prefix: String,
        hidden_size: usize,
        intermediate_size: usize,
        layer_idx: usize,
        expert_idx: usize,
        materialization: LinearWeightMaterialization,
        cache_policy: DeferredExpertCachePolicy,
    ) -> Self {
        Self {
            source,
            prefix,
            hidden_size,
            intermediate_size,
            layer_idx,
            expert_idx,
            materialization,
            cache_policy,
            cache: Arc::new(Mutex::new(None)),
        }
    }

    fn materialize(&self) -> Result<DenseMlpWeights> {
        let mut slot = self
            .cache
            .lock()
            .expect("deferred moe expert cache mutex poisoned");
        if matches!(self.cache_policy, DeferredExpertCachePolicy::Retain) {
            if let Some(weights) = slot.as_ref() {
                return Ok(weights.clone());
            }
        }
        let weights = with_low_precision_load_policy(self.source.inner.load_policy, || {
            let vb = self.source.build_var_builder()?;
            let expert_vb = vb.pp(self.prefix.clone());
            DenseMlpWeights::load(
                &expert_vb,
                self.hidden_size,
                self.intermediate_size,
                None,
                self.materialization,
            )
            .with_context(|| {
                format!(
                    "failed to lazily load MoE expert {} for layer {}",
                    self.expert_idx, self.layer_idx
                )
            })
        })?;
        info!(
            layer_idx = self.layer_idx,
            expert_idx = self.expert_idx,
            cache_policy = self.cache_policy.label(),
            "deepseek language load stage completed: moe_expert_materialized"
        );
        if matches!(self.cache_policy, DeferredExpertCachePolicy::Retain) {
            *slot = Some(weights.clone());
        }
        Ok(weights)
    }

    #[cfg(test)]
    fn cache_policy(&self) -> DeferredExpertCachePolicy {
        self.cache_policy
    }

    #[cfg(test)]
    fn materialization(&self) -> LinearWeightMaterialization {
        self.materialization
    }

    #[cfg(test)]
    fn is_cached(&self) -> bool {
        self.cache
            .lock()
            .map(|slot| slot.is_some())
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone)]
pub enum MoeExpertWeights {
    Eager(DenseMlpWeights),
    Deferred(DeferredDenseMlpWeights),
}

impl MoeExpertWeights {
    pub fn resolve(&self) -> Result<DenseMlpWeights> {
        match self {
            MoeExpertWeights::Eager(weights) => Ok(weights.clone()),
            MoeExpertWeights::Deferred(deferred) => deferred.materialize(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MoeSlowWeights {
    pub experts: Vec<MoeExpertWeights>,
    pub shared_experts: Option<DenseMlpWeights>,
}

#[derive(Debug, Clone)]
pub struct MoeMetalFastExpertPack {
    pub gate_proj: Tensor,
    pub up_proj: Tensor,
    pub down_proj: Tensor,
    pub expert_count: usize,
    pub hidden_size: usize,
    pub intermediate_size: usize,
}

impl MoeMetalFastExpertPack {
    fn from_slow(weights: &MoeSlowWeights, layer_idx: usize) -> Result<Option<Self>> {
        let pack_started_at = Instant::now();
        let mut eager_experts = Vec::with_capacity(weights.experts.len());
        let mut hidden_size = None;
        let mut intermediate_size = None;

        for expert in &weights.experts {
            let expert = match expert {
                MoeExpertWeights::Eager(expert) => expert,
                MoeExpertWeights::Deferred(_) => return Ok(None),
            };
            if float_packable_weight(&expert.gate_proj).is_none() {
                return Ok(None);
            }
            if float_packable_weight(&expert.up_proj).is_none() {
                return Ok(None);
            }
            if float_packable_weight(&expert.down_proj).is_none() {
                return Ok(None);
            }

            hidden_size.get_or_insert(expert.gate_proj.in_dim);
            intermediate_size.get_or_insert(expert.gate_proj.out_dim);
            eager_experts.push(expert);
        }

        if eager_experts.is_empty() {
            return Ok(None);
        }

        emit_stage_trace(
            "deepseek.language.transformer_layer.mlp.moe.pack.start",
            &[
                (
                    "elapsed_ms",
                    pack_started_at.elapsed().as_millis().to_string(),
                ),
                ("layer_idx", layer_idx.to_string()),
                ("expert_count", eager_experts.len().to_string()),
            ],
        );
        let gate_proj = Tensor::stack(
            &eager_experts
                .iter()
                .map(|expert| {
                    float_packable_weight(&expert.gate_proj)
                        .expect("validated packable gate_proj")
                        .clone()
                })
                .collect::<Vec<_>>(),
            0,
        )?
        .contiguous()?;
        emit_stage_trace(
            "deepseek.language.transformer_layer.mlp.moe.pack.gate.completed",
            &[
                (
                    "elapsed_ms",
                    pack_started_at.elapsed().as_millis().to_string(),
                ),
                ("layer_idx", layer_idx.to_string()),
                ("expert_count", eager_experts.len().to_string()),
            ],
        );
        let up_proj = Tensor::stack(
            &eager_experts
                .iter()
                .map(|expert| {
                    float_packable_weight(&expert.up_proj)
                        .expect("validated packable up_proj")
                        .clone()
                })
                .collect::<Vec<_>>(),
            0,
        )?
        .contiguous()?;
        emit_stage_trace(
            "deepseek.language.transformer_layer.mlp.moe.pack.up.completed",
            &[
                (
                    "elapsed_ms",
                    pack_started_at.elapsed().as_millis().to_string(),
                ),
                ("layer_idx", layer_idx.to_string()),
                ("expert_count", eager_experts.len().to_string()),
            ],
        );
        let down_proj = Tensor::stack(
            &eager_experts
                .iter()
                .map(|expert| {
                    float_packable_weight(&expert.down_proj)
                        .expect("validated packable down_proj")
                        .clone()
                })
                .collect::<Vec<_>>(),
            0,
        )?
        .contiguous()?;
        emit_stage_trace(
            "deepseek.language.transformer_layer.mlp.moe.pack.down.completed",
            &[
                (
                    "elapsed_ms",
                    pack_started_at.elapsed().as_millis().to_string(),
                ),
                ("layer_idx", layer_idx.to_string()),
                ("expert_count", eager_experts.len().to_string()),
            ],
        );

        let pack = Self {
            gate_proj,
            up_proj,
            down_proj,
            expert_count: eager_experts.len(),
            hidden_size: hidden_size.expect("non-empty expert pack should set hidden_size"),
            intermediate_size: intermediate_size
                .expect("non-empty expert pack should set intermediate_size"),
        };
        emit_stage_trace(
            "deepseek.language.transformer_layer.mlp.moe.pack.completed",
            &[
                (
                    "elapsed_ms",
                    pack_started_at.elapsed().as_millis().to_string(),
                ),
                ("layer_idx", layer_idx.to_string()),
                ("expert_count", pack.expert_count.to_string()),
            ],
        );
        Ok(Some(pack))
    }
}

fn float_packable_weight(weights: &LinearWeights) -> Option<&Tensor> {
    if weights.qmatmul.is_some() || weights.bias.is_some() {
        return None;
    }
    weights.weight.as_ref()
}

fn routed_expert_materialization(
    backend: MoeExecutionBackend,
    snapshot_present: bool,
) -> LinearWeightMaterialization {
    if matches!(backend, MoeExecutionBackend::MetalFast) && !snapshot_present {
        LinearWeightMaterialization::KeepViewForMetalFastPack
    } else {
        LinearWeightMaterialization::DefaultContiguous
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RoutedExpertLoadStrategy {
    Eager,
    Deferred,
}

impl RoutedExpertLoadStrategy {
    fn label(self) -> &'static str {
        match self {
            Self::Eager => "eager",
            Self::Deferred => "deferred",
        }
    }
}

fn routed_expert_load_strategy(
    load_policy: LowPrecisionLoadPolicy,
    backend: MoeExecutionBackend,
    snapshot_present: bool,
    has_deferred_source: bool,
) -> RoutedExpertLoadStrategy {
    if snapshot_present || !has_deferred_source {
        return RoutedExpertLoadStrategy::Eager;
    }
    if load_policy.lazy_moe_experts || matches!(backend, MoeExecutionBackend::MetalFast) {
        RoutedExpertLoadStrategy::Deferred
    } else {
        RoutedExpertLoadStrategy::Eager
    }
}

fn routed_expert_cache_policy(
    backend: MoeExecutionBackend,
    load_strategy: RoutedExpertLoadStrategy,
) -> DeferredExpertCachePolicy {
    if matches!(backend, MoeExecutionBackend::MetalFast)
        && matches!(load_strategy, RoutedExpertLoadStrategy::Deferred)
    {
        DeferredExpertCachePolicy::Ephemeral
    } else {
        DeferredExpertCachePolicy::Retain
    }
}

#[derive(Debug, Clone)]
pub struct MoeMetalFastWeights {
    pub packed_experts: Option<MoeMetalFastExpertPack>,
    pub fallback_experts: Option<Vec<MoeExpertWeights>>,
    pub shared_experts: Option<DenseMlpWeights>,
    pub expert_count: usize,
}

impl MoeMetalFastWeights {
    pub fn fallback_layout(&self) -> Option<MoeSlowWeights> {
        self.fallback_experts
            .as_ref()
            .map(|experts| MoeSlowWeights {
                experts: experts.clone(),
                shared_experts: self.shared_experts.clone(),
            })
    }
}

#[derive(Debug, Clone)]
pub enum MoeBackendWeights {
    Slow(MoeSlowWeights),
    MetalFast(MoeMetalFastWeights),
}

impl MoeBackendWeights {
    fn new(
        layer_idx: usize,
        backend: MoeExecutionBackend,
        experts: Vec<MoeExpertWeights>,
        shared_experts: Option<DenseMlpWeights>,
    ) -> Result<Self> {
        match backend {
            MoeExecutionBackend::Slow => Ok(Self::Slow(MoeSlowWeights {
                experts,
                shared_experts,
            })),
            MoeExecutionBackend::MetalFast => {
                let expert_count = experts.len();
                let slow = MoeSlowWeights {
                    experts,
                    shared_experts: shared_experts.clone(),
                };
                let packed_experts = MoeMetalFastExpertPack::from_slow(&slow, layer_idx)?;
                let fallback_experts = if packed_experts.is_some() {
                    None
                } else {
                    Some(slow.experts)
                };
                Ok(Self::MetalFast(MoeMetalFastWeights {
                    packed_experts,
                    fallback_experts,
                    shared_experts,
                    expert_count,
                }))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct MoeWeights {
    pub gate_weight: Tensor,
    pub backend: MoeBackendWeights,
    pub aux_bias: Option<Tensor>,
}

impl MoeWeights {
    pub fn backend_kind(&self) -> MoeExecutionBackend {
        match &self.backend {
            MoeBackendWeights::Slow(_) => MoeExecutionBackend::Slow,
            MoeBackendWeights::MetalFast(_) => MoeExecutionBackend::MetalFast,
        }
    }

    pub fn expert_count(&self) -> usize {
        match &self.backend {
            MoeBackendWeights::Slow(weights) => weights.experts.len(),
            MoeBackendWeights::MetalFast(weights) => weights.expert_count,
        }
    }

    fn load(
        cfg: &DeepseekV2Config,
        layer_idx: usize,
        vb: &VarBuilder,
        snapshot: Option<&QuantizedSnapshot>,
        deferred_source: Option<&DeferredMoeLoadSource>,
    ) -> Result<Self> {
        let load_started_at = Instant::now();
        let load_policy = current_low_precision_load_policy();
        let backend = MoeExecutionBackend::select(vb.device());
        let hidden_size = cfg.hidden_size;
        let moe_intermediate_size = cfg
            .moe_intermediate_size
            .with_context(|| "config.moe_intermediate_size missing for MoE layer")?;
        let num_routed = cfg
            .n_routed_experts
            .with_context(|| "config.n_routed_experts missing for MoE layer")?;
        ensure!(num_routed > 0, "n_routed_experts must be > 0 for MoE");

        emit_stage_trace(
            "deepseek.language.transformer_layer.mlp.moe.gate.start",
            &[
                (
                    "elapsed_ms",
                    load_started_at.elapsed().as_millis().to_string(),
                ),
                ("layer_idx", layer_idx.to_string()),
                ("expert_count", num_routed.to_string()),
            ],
        );
        let gate_weight = vb
            .pp("gate")
            .get((num_routed, hidden_size), "weight")
            .with_context(|| format!("missing MoE gate weight for layer {layer_idx}"))?;
        emit_stage_trace(
            "deepseek.language.transformer_layer.mlp.moe.gate.completed",
            &[
                (
                    "elapsed_ms",
                    load_started_at.elapsed().as_millis().to_string(),
                ),
                ("layer_idx", layer_idx.to_string()),
                ("expert_count", num_routed.to_string()),
            ],
        );
        info!(
            elapsed_ms = load_started_at.elapsed().as_millis(),
            layer_idx,
            expert_count = num_routed,
            "deepseek language load stage completed: moe_gate_ready"
        );
        let routed_load_strategy = routed_expert_load_strategy(
            load_policy,
            backend,
            snapshot.is_some(),
            deferred_source.is_some(),
        );
        let routed_cache_policy = routed_expert_cache_policy(backend, routed_load_strategy);
        let lazy_experts = matches!(routed_load_strategy, RoutedExpertLoadStrategy::Deferred);
        if load_policy.lazy_moe_experts && snapshot.is_some() {
            info!(
                elapsed_ms = load_started_at.elapsed().as_millis(),
                layer_idx,
                "deepseek language load stage skipped: moe_lazy_experts_snapshot_present"
            );
        } else if load_policy.lazy_moe_experts && deferred_source.is_none() {
            info!(
                elapsed_ms = load_started_at.elapsed().as_millis(),
                layer_idx, "deepseek language load stage skipped: moe_lazy_experts_missing_source"
            );
        } else if lazy_experts {
            info!(
                elapsed_ms = load_started_at.elapsed().as_millis(),
                layer_idx,
                moe_backend = backend.label(),
                routed_load_strategy = routed_load_strategy.label(),
                routed_cache_policy = routed_cache_policy.label(),
                "deepseek language load stage enabled: moe_lazy_experts"
            );
        }
        let routed_materialization = routed_expert_materialization(backend, snapshot.is_some());
        let aux_bias = if vb.pp("gate").contains_tensor("e_score_correction_bias") {
            Some(
                vb.pp("gate")
                    .get(num_routed, "e_score_correction_bias")
                    .with_context(|| {
                        format!("missing MoE gate e_score_correction_bias for layer {layer_idx}")
                    })?,
            )
        } else {
            None
        };
        if aux_bias.is_some() {
            emit_stage_trace(
                "deepseek.language.transformer_layer.mlp.moe.aux_bias.completed",
                &[
                    (
                        "elapsed_ms",
                        load_started_at.elapsed().as_millis().to_string(),
                    ),
                    ("layer_idx", layer_idx.to_string()),
                ],
            );
        }

        let mut experts = Vec::with_capacity(num_routed);
        for expert_idx in 0..num_routed {
            emit_stage_trace(
                "deepseek.language.transformer_layer.mlp.moe.expert.start",
                &[
                    (
                        "elapsed_ms",
                        load_started_at.elapsed().as_millis().to_string(),
                    ),
                    ("layer_idx", layer_idx.to_string()),
                    ("expert_idx", expert_idx.to_string()),
                    ("lazy", lazy_experts.to_string()),
                    ("load_strategy", routed_load_strategy.label().to_string()),
                    ("cache_policy", routed_cache_policy.label().to_string()),
                ],
            );
            info!(
                elapsed_ms = load_started_at.elapsed().as_millis(),
                layer_idx,
                expert_idx,
                routed_load_strategy = routed_load_strategy.label(),
                routed_cache_policy = routed_cache_policy.label(),
                "deepseek language load stage completed: moe_expert_start"
            );
            if lazy_experts {
                let source = deferred_source
                    .cloned()
                    .expect("deferred source should exist when lazy experts are enabled");
                experts.push(MoeExpertWeights::Deferred(
                    DeferredDenseMlpWeights::new_expert(
                        source,
                        format!("model.layers.{layer_idx}.mlp.experts.{expert_idx}"),
                        hidden_size,
                        moe_intermediate_size,
                        layer_idx,
                        expert_idx,
                        routed_materialization,
                        routed_cache_policy,
                    ),
                ));
                emit_stage_trace(
                    "deepseek.language.transformer_layer.mlp.moe.expert.completed",
                    &[
                        (
                            "elapsed_ms",
                            load_started_at.elapsed().as_millis().to_string(),
                        ),
                        ("layer_idx", layer_idx.to_string()),
                        ("expert_idx", expert_idx.to_string()),
                        ("lazy", true.to_string()),
                        ("load_strategy", routed_load_strategy.label().to_string()),
                        ("cache_policy", routed_cache_policy.label().to_string()),
                        ("loaded_experts", experts.len().to_string()),
                    ],
                );
                info!(
                    elapsed_ms = load_started_at.elapsed().as_millis(),
                    layer_idx,
                    expert_idx,
                    staged_experts = experts.len(),
                    routed_cache_policy = routed_cache_policy.label(),
                    "deepseek language load stage completed: moe_expert_deferred"
                );
                continue;
            }

            let expert_vb = vb.pp(format!("experts.{expert_idx}"));
            let expert = DenseMlpWeights::load(
                &expert_vb,
                hidden_size,
                moe_intermediate_size,
                snapshot,
                routed_materialization,
            )
            .with_context(|| {
                format!("failed to load MoE expert {expert_idx} (layer {layer_idx})")
            })?;
            experts.push(MoeExpertWeights::Eager(expert));
            emit_stage_trace(
                "deepseek.language.transformer_layer.mlp.moe.expert.completed",
                &[
                    (
                        "elapsed_ms",
                        load_started_at.elapsed().as_millis().to_string(),
                    ),
                    ("layer_idx", layer_idx.to_string()),
                    ("expert_idx", expert_idx.to_string()),
                    ("lazy", false.to_string()),
                    ("load_strategy", routed_load_strategy.label().to_string()),
                    ("loaded_experts", experts.len().to_string()),
                ],
            );
            info!(
                elapsed_ms = load_started_at.elapsed().as_millis(),
                layer_idx,
                expert_idx,
                loaded_experts = experts.len(),
                "deepseek language load stage completed: moe_expert_ready"
            );
        }

        let shared_experts = load_shared_experts(
            layer_idx,
            cfg,
            vb,
            hidden_size,
            moe_intermediate_size,
            snapshot,
            &load_started_at,
        )?;
        if shared_experts.is_some() {
            emit_stage_trace(
                "deepseek.language.transformer_layer.mlp.moe.shared_experts.completed",
                &[
                    (
                        "elapsed_ms",
                        load_started_at.elapsed().as_millis().to_string(),
                    ),
                    ("layer_idx", layer_idx.to_string()),
                ],
            );
            info!(
                elapsed_ms = load_started_at.elapsed().as_millis(),
                layer_idx, "deepseek language load stage completed: moe_shared_experts_ready"
            );
        }
        let backend = MoeBackendWeights::new(layer_idx, backend, experts, shared_experts)?;
        emit_stage_trace(
            "deepseek.language.transformer_layer.mlp.moe.backend.completed",
            &[
                (
                    "elapsed_ms",
                    load_started_at.elapsed().as_millis().to_string(),
                ),
                ("layer_idx", layer_idx.to_string()),
                (
                    "moe_backend",
                    match &backend {
                        MoeBackendWeights::Slow(_) => MoeExecutionBackend::Slow.label(),
                        MoeBackendWeights::MetalFast(_) => MoeExecutionBackend::MetalFast.label(),
                    }
                    .to_string(),
                ),
                (
                    "packed_experts",
                    match &backend {
                        MoeBackendWeights::Slow(_) => false,
                        MoeBackendWeights::MetalFast(weights) => weights.packed_experts.is_some(),
                    }
                    .to_string(),
                ),
            ],
        );
        info!(
            elapsed_ms = load_started_at.elapsed().as_millis(),
            layer_idx,
            moe_backend = match &backend {
                MoeBackendWeights::Slow(_) => MoeExecutionBackend::Slow.label(),
                MoeBackendWeights::MetalFast(_) => MoeExecutionBackend::MetalFast.label(),
            },
            packed_experts = match &backend {
                MoeBackendWeights::Slow(_) => false,
                MoeBackendWeights::MetalFast(weights) => weights.packed_experts.is_some(),
            },
            fallback_experts_kept = match &backend {
                MoeBackendWeights::Slow(_) => true,
                MoeBackendWeights::MetalFast(weights) => weights.fallback_experts.is_some(),
            },
            "deepseek language load stage completed: moe_backend_ready"
        );

        Ok(Self {
            gate_weight,
            backend,
            aux_bias,
        })
    }
}

#[derive(Debug, Clone)]
pub enum MlpWeights {
    Dense(DenseMlpWeights),
    Moe(MoeWeights),
}

impl MlpWeights {
    fn load(
        cfg: &DeepseekV2Config,
        layer_idx: usize,
        vb: &VarBuilder,
        snapshot: Option<&QuantizedSnapshot>,
        deferred_source: Option<&DeferredMoeLoadSource>,
    ) -> Result<Self> {
        let hidden_size = cfg.hidden_size;
        let intermediate_size = cfg.intermediate_size;
        if should_use_moe(cfg, layer_idx) {
            MoeWeights::load(cfg, layer_idx, vb, snapshot, deferred_source).map(MlpWeights::Moe)
        } else {
            DenseMlpWeights::load(
                vb,
                hidden_size,
                intermediate_size,
                snapshot,
                LinearWeightMaterialization::DefaultContiguous,
            )
            .map(MlpWeights::Dense)
        }
    }
}

#[derive(Debug, Clone)]
pub struct TransformerBlockWeights {
    pub attention: AttentionWeights,
    pub mlp: MlpWeights,
    pub input_layernorm: RmsNormWeights,
    pub post_attention_layernorm: RmsNormWeights,
}

impl TransformerBlockWeights {
    pub fn load(
        cfg: &DeepseekV2Config,
        layer_idx: usize,
        vb: &VarBuilder,
        snapshot: Option<&QuantizedSnapshot>,
        deferred_source: Option<&DeferredMoeLoadSource>,
    ) -> Result<Self> {
        let load_started_at = Instant::now();
        let attention = AttentionWeights::load(cfg, layer_idx, vb, snapshot)?;
        emit_stage_trace(
            "deepseek.language.transformer_layer.attention.completed",
            &[
                (
                    "elapsed_ms",
                    load_started_at.elapsed().as_millis().to_string(),
                ),
                ("layer_idx", layer_idx.to_string()),
            ],
        );
        info!(
            elapsed_ms = load_started_at.elapsed().as_millis(),
            layer_idx, "deepseek language load stage completed: transformer_layer_attention_ready"
        );
        let mlp = MlpWeights::load(cfg, layer_idx, &vb.pp("mlp"), snapshot, deferred_source)?;
        emit_stage_trace(
            "deepseek.language.transformer_layer.mlp.completed",
            &[
                (
                    "elapsed_ms",
                    load_started_at.elapsed().as_millis().to_string(),
                ),
                ("layer_idx", layer_idx.to_string()),
                ("moe", matches!(mlp, MlpWeights::Moe(_)).to_string()),
            ],
        );
        info!(
            elapsed_ms = load_started_at.elapsed().as_millis(),
            layer_idx,
            moe = matches!(mlp, MlpWeights::Moe(_)),
            "deepseek language load stage completed: transformer_layer_mlp_ready"
        );
        let input_layernorm = RmsNormWeights::load(&vb.pp("input_layernorm"), cfg.hidden_size)?;
        let post_attention_layernorm =
            RmsNormWeights::load(&vb.pp("post_attention_layernorm"), cfg.hidden_size)?;
        emit_stage_trace(
            "deepseek.language.transformer_layer.norms.completed",
            &[
                (
                    "elapsed_ms",
                    load_started_at.elapsed().as_millis().to_string(),
                ),
                ("layer_idx", layer_idx.to_string()),
            ],
        );
        info!(
            elapsed_ms = load_started_at.elapsed().as_millis(),
            layer_idx, "deepseek language load stage completed: transformer_layer_norms_ready"
        );
        emit_stage_trace(
            "deepseek.language.transformer_layer.completed",
            &[
                (
                    "elapsed_ms",
                    load_started_at.elapsed().as_millis().to_string(),
                ),
                ("layer_idx", layer_idx.to_string()),
            ],
        );
        Ok(Self {
            attention,
            mlp,
            input_layernorm,
            post_attention_layernorm,
        })
    }
}

#[derive(Debug, Clone)]
pub struct TransformerWeights {
    pub layers: Vec<TransformerBlockWeights>,
}

impl TransformerWeights {
    pub fn load(
        cfg: &DeepseekV2Config,
        vb: &VarBuilder,
        snapshot: Option<&QuantizedSnapshot>,
    ) -> Result<Self> {
        Self::load_with_deferred_source(cfg, vb, snapshot, None)
    }

    pub fn load_with_deferred_source(
        cfg: &DeepseekV2Config,
        vb: &VarBuilder,
        snapshot: Option<&QuantizedSnapshot>,
        deferred_source: Option<&DeferredMoeLoadSource>,
    ) -> Result<Self> {
        let load_started_at = Instant::now();
        emit_stage_trace(
            "deepseek.language.transformer_layers.start",
            &[
                (
                    "elapsed_ms",
                    load_started_at.elapsed().as_millis().to_string(),
                ),
                ("layer_count", cfg.num_hidden_layers.to_string()),
            ],
        );
        info!(
            elapsed_ms = load_started_at.elapsed().as_millis(),
            layer_count = cfg.num_hidden_layers,
            "deepseek language load stage completed: transformer_layers_start"
        );
        let mut layers = Vec::with_capacity(cfg.num_hidden_layers);
        for layer_idx in 0..cfg.num_hidden_layers {
            emit_stage_trace(
                "deepseek.language.transformer_layer.start",
                &[
                    (
                        "elapsed_ms",
                        load_started_at.elapsed().as_millis().to_string(),
                    ),
                    ("layer_idx", layer_idx.to_string()),
                    ("moe", should_use_moe(cfg, layer_idx).to_string()),
                ],
            );
            info!(
                elapsed_ms = load_started_at.elapsed().as_millis(),
                layer_idx,
                moe = should_use_moe(cfg, layer_idx),
                "deepseek language load stage completed: transformer_layer_start"
            );
            let layer_vb = vb.pp(format!("layers.{layer_idx}"));
            let layer =
                TransformerBlockWeights::load(cfg, layer_idx, &layer_vb, snapshot, deferred_source)
                    .with_context(|| format!("failed to load transformer layer `{layer_idx}`"))?;
            layers.push(layer);
            info!(
                elapsed_ms = load_started_at.elapsed().as_millis(),
                layer_idx,
                loaded_layers = layers.len(),
                moe = should_use_moe(cfg, layer_idx),
                "deepseek language load stage completed: transformer_layer_ready"
            );
        }
        info!(
            elapsed_ms = load_started_at.elapsed().as_millis(),
            loaded_layers = layers.len(),
            "deepseek language load stage completed: transformer_layers_ready"
        );
        Ok(Self { layers })
    }
}

#[derive(Debug, Clone)]
pub struct DeepseekLanguageModelWeights {
    pub token_embedding: Tensor,
    pub transformer: TransformerWeights,
    pub final_layernorm: RmsNormWeights,
    pub lm_head_weight: Option<Tensor>,
    pub lm_head_q: Option<Arc<QMatMul>>,
    pub lm_out_dim: usize,
    pub lm_in_dim: usize,
    pub lm_head_label: String,
}

impl DeepseekLanguageModelWeights {
    pub fn load(
        cfg: &DeepseekV2Config,
        vb: &VarBuilder,
        snapshot: Option<&QuantizedSnapshot>,
    ) -> Result<Self> {
        Self::load_with_deferred_source(cfg, vb, snapshot, None)
    }

    pub fn load_with_deferred_source(
        cfg: &DeepseekV2Config,
        vb: &VarBuilder,
        snapshot: Option<&QuantizedSnapshot>,
        deferred_source: Option<&DeferredMoeLoadSource>,
    ) -> Result<Self> {
        let load_started_at = Instant::now();
        let model_vb = vb.pp("model");
        let token_embedding = model_vb
            .pp("embed_tokens")
            .get((cfg.vocab_size, cfg.hidden_size), "weight")
            .with_context(|| {
                format!(
                    "missing token embedding `{}`",
                    qualified_name(&model_vb.pp("embed_tokens"), "weight")
                )
            })?;
        let token_embedding = token_embedding.contiguous()?;
        emit_stage_trace(
            "deepseek.language.token_embedding.completed",
            &[(
                "elapsed_ms",
                load_started_at.elapsed().as_millis().to_string(),
            )],
        );
        info!(
            elapsed_ms = load_started_at.elapsed().as_millis(),
            "deepseek language load stage completed: token_embedding"
        );
        let transformer = TransformerWeights::load_with_deferred_source(
            cfg,
            &model_vb,
            snapshot,
            deferred_source,
        )?;
        info!(
            elapsed_ms = load_started_at.elapsed().as_millis(),
            "deepseek language load stage completed: transformer"
        );
        let final_layernorm = RmsNormWeights::load(&model_vb.pp("norm"), cfg.hidden_size)
            .with_context(|| {
                format!(
                    "missing final layernorm `{}`",
                    qualified_name(&model_vb.pp("norm"), "weight")
                )
            })?;
        emit_stage_trace(
            "deepseek.language.final_layernorm.completed",
            &[(
                "elapsed_ms",
                load_started_at.elapsed().as_millis().to_string(),
            )],
        );
        info!(
            elapsed_ms = load_started_at.elapsed().as_millis(),
            "deepseek language load stage completed: final_layernorm"
        );
        let lm_head_vb = vb.pp("lm_head");
        let lm_head_label = qualified_name(&lm_head_vb, "weight");
        let mut lm_head_weight = Some(
            lm_head_vb
                .get((cfg.vocab_size, cfg.hidden_size), "weight")
                .with_context(|| format!("missing lm_head weight `{}`", lm_head_label))?
                .contiguous()?,
        );
        emit_stage_trace(
            "deepseek.language.lm_head_weight.completed",
            &[(
                "elapsed_ms",
                load_started_at.elapsed().as_millis().to_string(),
            )],
        );
        info!(
            elapsed_ms = load_started_at.elapsed().as_millis(),
            "deepseek language load stage completed: lm_head_weight"
        );

        if cfg.tie_word_embeddings {
            ensure!(
                token_embedding.shape().dims() == [cfg.vocab_size, cfg.hidden_size],
                "tie_word_embeddings enabled but embedding/logit weights differ: {:?} vs {:?}",
                token_embedding.shape().dims(),
                [cfg.vocab_size, cfg.hidden_size]
            );
        }

        // Try offline snapshot first, falling back to runtime quantization.
        let quant = QuantizationState::global();
        let mut lm_q = None;
        if let Some(snapshot) = snapshot {
            let mut plan = SnapshotLoadPlan::default();
            plan.push(LinearSpec::new(
                lm_head_label.clone(),
                cfg.vocab_size,
                cfg.hidden_size,
            ));
            let mut hits = plan.execute(Some(snapshot), vb.device(), None)?;
            if let Some(hit) = hits.as_mut().and_then(|map| map.remove(&lm_head_label)) {
                match hit {
                    SnapshotLinear::Quantized { qmatmul, bias: _ } => {
                        let path = if vb.device().is_cuda() || vb.device().is_metal() {
                            "kernel_upcast"
                        } else {
                            "kernel"
                        };
                        trace!(
                            tensor = lm_head_label,
                            module = "lm_head",
                            in_dim = cfg.hidden_size,
                            out_dim = cfg.vocab_size,
                            backend = backend_label(vb.device()),
                            path,
                            container = snapshot.container_label(),
                            source = "snapshot",
                            action = "quantized",
                            "quant-linear"
                        );
                        quant.record_attempt(QuantModule::LmHead, QuantizationOutcome::Quantized);
                        lm_q = Some(qmatmul);
                        lm_head_weight = None;
                    }
                    SnapshotLinear::Float { weight, bias: _ } => {
                        trace!(
                            tensor = lm_head_label,
                            module = "lm_head",
                            in_dim = cfg.hidden_size,
                            out_dim = cfg.vocab_size,
                            backend = backend_label(vb.device()),
                            path = "snapshot-float",
                            container = snapshot.container_label(),
                            source = "snapshot",
                            action = "float",
                            "quant-linear"
                        );
                        quant.record_attempt(QuantModule::LmHead, QuantizationOutcome::Fallback);
                        lm_head_weight = Some(weight);
                    }
                }
            }
        }
        emit_stage_trace(
            "deepseek.language.weights_ready",
            &[
                (
                    "elapsed_ms",
                    load_started_at.elapsed().as_millis().to_string(),
                ),
                ("quantized_lm_head", lm_q.is_some().to_string()),
            ],
        );
        info!(
            elapsed_ms = load_started_at.elapsed().as_millis(),
            quantized_lm_head = lm_q.is_some(),
            "deepseek language load stage completed: language_weights_ready"
        );

        Ok(Self {
            token_embedding,
            transformer,
            final_layernorm,
            lm_head_weight,
            lm_head_q: lm_q,
            lm_out_dim: cfg.vocab_size,
            lm_in_dim: cfg.hidden_size,
            lm_head_label,
        })
    }
}

fn should_use_moe(cfg: &DeepseekV2Config, layer_idx: usize) -> bool {
    let num_routed = cfg.n_routed_experts.unwrap_or(0);
    if num_routed == 0 {
        return false;
    }
    let first_dense = cfg.first_k_dense_replace.unwrap_or(0);
    if layer_idx < first_dense {
        return false;
    }
    layer_idx.is_multiple_of(cfg.moe_layer_freq)
}

fn non_zero_or(value: Option<usize>, fallback: usize) -> usize {
    match value {
        Some(v) if v > 0 => v,
        _ => fallback,
    }
}

pub(crate) fn qualified_name(vb: &VarBuilder, tensor: &str) -> String {
    let prefix = vb.prefix();
    if prefix.is_empty() {
        tensor.to_string()
    } else {
        let mut composed = String::with_capacity(prefix.len() + tensor.len() + 1);
        let _ = write!(composed, "{prefix}.{tensor}");
        composed
    }
}

// Runtime quantization path removed: no `maybe_quantize_linear` fallback.

#[cfg(test)]
mod tests {
    use super::{
        DeferredDenseMlpWeights, DeferredExpertCachePolicy, DeferredMoeLoadSource, DenseMlpWeights,
        LinearWeightMaterialization, LinearWeights, MoeBackendWeights, MoeExecutionBackend,
        MoeExpertWeights, MoeMetalFastExpertPack, MoeSlowWeights, RoutedExpertLoadStrategy,
        load_shared_experts, routed_expert_cache_policy, routed_expert_load_strategy,
        routed_expert_materialization, skip_shared_experts_enabled, stage_trace_enabled,
    };
    use crate::config::DeepseekV2Config;
    use crate::model::LowPrecisionLoadPolicy;
    use crate::quantization::{LinearLayerGroup, QuantModule};
    use candle_core::{
        DType, Device, Tensor,
        quantized::{GgmlDType, QMatMul, QTensor},
    };
    use candle_nn::VarBuilder;
    use std::time::Instant;
    use std::{
        collections::HashMap,
        path::PathBuf,
        sync::{Arc, Mutex, OnceLock},
    };

    fn env_lock() -> &'static Mutex<()> {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_LOCK.get_or_init(|| Mutex::new(()))
    }

    fn test_linear(label: &str, out_dim: usize, in_dim: usize, fill: f32) -> LinearWeights {
        let data = vec![fill; out_dim * in_dim];
        LinearWeights {
            weight: Some(Tensor::from_vec(data, (out_dim, in_dim), &Device::Cpu).unwrap()),
            weight_f32: None,
            bias: None,
            qmatmul: None,
            out_dim,
            in_dim,
            label: label.to_string(),
        }
    }

    fn test_linear_with_weight(label: &str, weight: Tensor) -> LinearWeights {
        let dims = weight.shape().dims();
        let [out_dim, in_dim]: [usize; 2] = dims.try_into().expect("expected rank-2 weight");
        LinearWeights {
            weight: Some(weight),
            weight_f32: None,
            bias: None,
            qmatmul: None,
            out_dim,
            in_dim,
            label: label.to_string(),
        }
    }

    fn test_linear_with_bias(label: &str, out_dim: usize, in_dim: usize) -> LinearWeights {
        let mut linear = test_linear(label, out_dim, in_dim, 1.0);
        linear.bias = Some(Tensor::zeros(out_dim, DType::F32, &Device::Cpu).unwrap());
        linear
    }

    fn test_linear_quantized(
        label: &str,
        out_dim: usize,
        in_dim: usize,
        fill: f32,
    ) -> LinearWeights {
        let weight = Tensor::from_vec(
            vec![fill; out_dim * in_dim],
            (out_dim, in_dim),
            &Device::Cpu,
        )
        .unwrap();
        let qtensor = QTensor::quantize(&weight, GgmlDType::Q8_0).unwrap();
        LinearWeights {
            weight: None,
            weight_f32: None,
            bias: None,
            qmatmul: Some(Arc::new(QMatMul::from_qtensor(qtensor).unwrap())),
            out_dim,
            in_dim,
            label: label.to_string(),
        }
    }

    fn test_non_contiguous_weight(out_dim: usize, in_dim: usize) -> Tensor {
        let base = Tensor::from_vec(
            (0..(out_dim * in_dim))
                .map(|v| v as f32)
                .collect::<Vec<_>>(),
            (in_dim, out_dim),
            &Device::Cpu,
        )
        .unwrap();
        let view = base.transpose(0, 1).unwrap();
        assert_eq!(view.shape().dims(), &[out_dim, in_dim]);
        assert!(!view.is_contiguous());
        view
    }

    fn test_var_builder(weight: Tensor, bias: Option<Tensor>) -> VarBuilder<'static> {
        let mut tensors = HashMap::from([("weight".to_string(), weight)]);
        if let Some(bias) = bias {
            tensors.insert("bias".to_string(), bias);
        }
        VarBuilder::from_tensors(tensors, DType::F32, &Device::Cpu)
    }

    fn test_dense_var_builder(gate: Tensor, up: Tensor, down: Tensor) -> VarBuilder<'static> {
        let tensors = HashMap::from([
            ("gate_proj.weight".to_string(), gate),
            ("up_proj.weight".to_string(), up),
            ("down_proj.weight".to_string(), down),
        ]);
        VarBuilder::from_tensors(tensors, DType::F32, &Device::Cpu)
    }

    fn test_dense(fill: f32) -> DenseMlpWeights {
        DenseMlpWeights {
            gate_proj: test_linear("gate", 3, 2, fill),
            up_proj: test_linear("up", 3, 2, fill + 1.0),
            down_proj: test_linear("down", 2, 3, fill + 2.0),
        }
    }

    #[test]
    fn deferred_moe_load_source_clone_shares_state() {
        let source = DeferredMoeLoadSource::new(
            vec![PathBuf::from("model.safetensors")],
            DType::F16,
            &Device::Cpu,
            LowPrecisionLoadPolicy {
                preload_language_f32_aux: false,
                preload_vision_f32_aux: false,
                preload_linear_weight_f32: false,
                promote_language_input_f32: false,
                lazy_moe_experts: true,
                lazy_clip_transformer_layers: false,
            },
        );
        let cloned = source.clone();
        assert!(source.shares_state_with(&cloned));
    }

    #[test]
    fn moe_execution_backend_parses_known_values() {
        assert_eq!(
            MoeExecutionBackend::parse("metal_fast"),
            Some(MoeExecutionBackend::MetalFast)
        );
        assert_eq!(
            MoeExecutionBackend::parse("metal-fast"),
            Some(MoeExecutionBackend::MetalFast)
        );
        assert_eq!(
            MoeExecutionBackend::parse("slow"),
            Some(MoeExecutionBackend::Slow)
        );
        assert_eq!(MoeExecutionBackend::parse("unknown"), None);
    }

    #[test]
    fn moe_execution_backend_defaults_to_slow() {
        assert_eq!(
            MoeExecutionBackend::select(&Device::Cpu),
            MoeExecutionBackend::Slow
        );
    }

    #[test]
    fn routed_expert_materialization_only_keeps_views_for_metal_fast_without_snapshot() {
        assert_eq!(
            routed_expert_materialization(MoeExecutionBackend::Slow, false),
            LinearWeightMaterialization::DefaultContiguous
        );
        assert_eq!(
            routed_expert_materialization(MoeExecutionBackend::MetalFast, true),
            LinearWeightMaterialization::DefaultContiguous
        );
        assert_eq!(
            routed_expert_materialization(MoeExecutionBackend::MetalFast, false),
            LinearWeightMaterialization::KeepViewForMetalFastPack
        );
    }

    #[test]
    fn routed_expert_load_strategy_keeps_eager_for_slow_backend_by_default() {
        assert_eq!(
            routed_expert_load_strategy(
                LowPrecisionLoadPolicy::default(),
                MoeExecutionBackend::Slow,
                false,
                true,
            ),
            RoutedExpertLoadStrategy::Eager
        );
    }

    #[test]
    fn routed_expert_load_strategy_defers_for_metal_fast_without_snapshot() {
        assert_eq!(
            routed_expert_load_strategy(
                LowPrecisionLoadPolicy::default(),
                MoeExecutionBackend::MetalFast,
                false,
                true,
            ),
            RoutedExpertLoadStrategy::Deferred
        );
    }

    #[test]
    fn routed_expert_load_strategy_respects_snapshot_and_missing_source() {
        assert_eq!(
            routed_expert_load_strategy(
                LowPrecisionLoadPolicy {
                    lazy_moe_experts: true,
                    ..LowPrecisionLoadPolicy::default()
                },
                MoeExecutionBackend::MetalFast,
                true,
                true,
            ),
            RoutedExpertLoadStrategy::Eager
        );
        assert_eq!(
            routed_expert_load_strategy(
                LowPrecisionLoadPolicy {
                    lazy_moe_experts: true,
                    ..LowPrecisionLoadPolicy::default()
                },
                MoeExecutionBackend::MetalFast,
                false,
                false,
            ),
            RoutedExpertLoadStrategy::Eager
        );
    }

    #[test]
    fn routed_expert_cache_policy_is_ephemeral_for_metal_fast_deferred() {
        assert_eq!(
            routed_expert_cache_policy(
                MoeExecutionBackend::MetalFast,
                RoutedExpertLoadStrategy::Deferred,
            ),
            DeferredExpertCachePolicy::Ephemeral
        );
    }

    #[test]
    fn routed_expert_cache_policy_retains_for_non_metal_or_eager_paths() {
        assert_eq!(
            routed_expert_cache_policy(
                MoeExecutionBackend::Slow,
                RoutedExpertLoadStrategy::Deferred,
            ),
            DeferredExpertCachePolicy::Retain
        );
        assert_eq!(
            routed_expert_cache_policy(
                MoeExecutionBackend::MetalFast,
                RoutedExpertLoadStrategy::Eager,
            ),
            DeferredExpertCachePolicy::Retain
        );
    }

    #[test]
    fn linear_weights_default_mode_makes_weight_contiguous() {
        let vb = test_var_builder(test_non_contiguous_weight(3, 2), None);
        let weights = LinearWeights::load(
            &vb,
            3,
            2,
            false,
            LinearWeightMaterialization::DefaultContiguous,
            LinearLayerGroup::Text,
            QuantModule::TextLinear,
            None,
            None,
        )
        .expect("linear load should succeed");
        assert!(
            weights
                .weight
                .as_ref()
                .expect("dense path should keep weight")
                .is_contiguous()
        );
    }

    #[test]
    fn linear_weights_keep_view_mode_preserves_non_contiguous_weight() {
        let vb = test_var_builder(test_non_contiguous_weight(3, 2), None);
        let weights = LinearWeights::load(
            &vb,
            3,
            2,
            false,
            LinearWeightMaterialization::KeepViewForMetalFastPack,
            LinearLayerGroup::Text,
            QuantModule::TextLinear,
            None,
            None,
        )
        .expect("linear load should succeed");
        assert!(
            !weights
                .weight
                .as_ref()
                .expect("dense path should keep weight")
                .is_contiguous()
        );
    }

    #[test]
    fn linear_weights_keep_view_mode_still_forces_contiguous_when_bias_exists() {
        let vb = test_var_builder(
            test_non_contiguous_weight(3, 2),
            Some(Tensor::zeros(3, DType::F32, &Device::Cpu).unwrap()),
        );
        let weights = LinearWeights::load(
            &vb,
            3,
            2,
            true,
            LinearWeightMaterialization::KeepViewForMetalFastPack,
            LinearLayerGroup::Text,
            QuantModule::TextLinear,
            None,
            None,
        )
        .expect("linear load should succeed");
        assert!(
            weights
                .weight
                .as_ref()
                .expect("bias path should keep weight")
                .is_contiguous()
        );
        assert!(weights.bias.is_some());
    }

    #[test]
    fn dense_mlp_keep_view_mode_preserves_non_contiguous_projection_weights() {
        let vb = test_dense_var_builder(
            test_non_contiguous_weight(3, 2),
            test_non_contiguous_weight(3, 2),
            test_non_contiguous_weight(2, 3),
        );
        let weights = DenseMlpWeights::load(
            &vb,
            2,
            3,
            None,
            LinearWeightMaterialization::KeepViewForMetalFastPack,
        )
        .expect("dense mlp load should succeed");
        assert!(
            !weights
                .gate_proj
                .weight
                .as_ref()
                .expect("gate weight should exist")
                .is_contiguous()
        );
        assert!(
            !weights
                .up_proj
                .weight
                .as_ref()
                .expect("up weight should exist")
                .is_contiguous()
        );
        assert!(
            !weights
                .down_proj
                .weight
                .as_ref()
                .expect("down weight should exist")
                .is_contiguous()
        );
    }

    #[test]
    fn metal_fast_pack_stacks_eager_float_experts() {
        let slow = MoeSlowWeights {
            experts: vec![
                MoeExpertWeights::Eager(test_dense(1.0)),
                MoeExpertWeights::Eager(test_dense(2.0)),
            ],
            shared_experts: None,
        };
        let pack = MoeMetalFastExpertPack::from_slow(&slow, 0)
            .expect("pack construction should succeed")
            .expect("eager float experts should pack");
        assert_eq!(pack.gate_proj.shape().dims(), &[2, 3, 2]);
        assert_eq!(pack.up_proj.shape().dims(), &[2, 3, 2]);
        assert_eq!(pack.down_proj.shape().dims(), &[2, 2, 3]);
        assert_eq!(pack.expert_count, 2);
        assert_eq!(pack.hidden_size, 2);
        assert_eq!(pack.intermediate_size, 3);
    }

    #[test]
    fn metal_fast_pack_skips_deferred_experts() {
        let source = DeferredMoeLoadSource::new(
            vec![PathBuf::from("model.safetensors")],
            DType::F16,
            &Device::Cpu,
            LowPrecisionLoadPolicy::default(),
        );
        let slow = MoeSlowWeights {
            experts: vec![MoeExpertWeights::Deferred(
                DeferredDenseMlpWeights::new_expert(
                    source,
                    "model.layers.0.mlp.experts.0".to_string(),
                    2,
                    3,
                    0,
                    0,
                    LinearWeightMaterialization::DefaultContiguous,
                    DeferredExpertCachePolicy::Retain,
                ),
            )],
            shared_experts: None,
        };
        assert!(
            MoeMetalFastExpertPack::from_slow(&slow, 0)
                .expect("deferred experts should skip packing")
                .is_none()
        );
    }

    #[test]
    fn metal_fast_pack_accepts_view_backed_experts_and_outputs_contiguous_pack() {
        let slow = MoeSlowWeights {
            experts: vec![
                MoeExpertWeights::Eager(DenseMlpWeights {
                    gate_proj: test_linear_with_weight("gate0", test_non_contiguous_weight(3, 2)),
                    up_proj: test_linear_with_weight("up0", test_non_contiguous_weight(3, 2)),
                    down_proj: test_linear_with_weight("down0", test_non_contiguous_weight(2, 3)),
                }),
                MoeExpertWeights::Eager(DenseMlpWeights {
                    gate_proj: test_linear_with_weight("gate1", test_non_contiguous_weight(3, 2)),
                    up_proj: test_linear_with_weight("up1", test_non_contiguous_weight(3, 2)),
                    down_proj: test_linear_with_weight("down1", test_non_contiguous_weight(2, 3)),
                }),
            ],
            shared_experts: None,
        };
        let pack = MoeMetalFastExpertPack::from_slow(&slow, 3)
            .expect("pack construction should succeed")
            .expect("view-backed eager experts should pack");
        assert!(pack.gate_proj.is_contiguous());
        assert!(pack.up_proj.is_contiguous());
        assert!(pack.down_proj.is_contiguous());
        assert_eq!(pack.gate_proj.shape().dims(), &[2, 3, 2]);
    }

    #[test]
    fn metal_fast_backend_prepares_pack_when_eager_float_experts_exist() {
        let backend = MoeBackendWeights::new(
            0,
            MoeExecutionBackend::MetalFast,
            vec![
                MoeExpertWeights::Eager(test_dense(1.0)),
                MoeExpertWeights::Eager(test_dense(2.0)),
            ],
            None,
        )
        .expect("backend build should succeed");
        match backend {
            MoeBackendWeights::MetalFast(weights) => {
                assert!(weights.packed_experts.is_some());
                assert!(weights.fallback_experts.is_none());
                assert_eq!(weights.expert_count, 2);
            }
            MoeBackendWeights::Slow(_) => panic!("expected metal fast backend"),
        }
    }

    #[test]
    fn metal_fast_backend_falls_back_for_bias_or_quantized_experts() {
        let bias_backend = MoeBackendWeights::new(
            0,
            MoeExecutionBackend::MetalFast,
            vec![MoeExpertWeights::Eager(DenseMlpWeights {
                gate_proj: test_linear_with_bias("gate", 3, 2),
                up_proj: test_linear("up", 3, 2, 2.0),
                down_proj: test_linear("down", 2, 3, 3.0),
            })],
            None,
        )
        .expect("backend build should succeed");
        match bias_backend {
            MoeBackendWeights::MetalFast(weights) => {
                assert!(weights.packed_experts.is_none());
                assert!(weights.fallback_experts.is_some());
            }
            MoeBackendWeights::Slow(_) => panic!("expected metal fast backend"),
        }

        let quantized_backend = MoeBackendWeights::new(
            0,
            MoeExecutionBackend::MetalFast,
            vec![MoeExpertWeights::Eager(DenseMlpWeights {
                gate_proj: test_linear_quantized("gate", 32, 32, 1.0),
                up_proj: test_linear("up", 32, 32, 2.0),
                down_proj: test_linear("down", 32, 32, 3.0),
            })],
            None,
        )
        .expect("backend build should succeed");
        match quantized_backend {
            MoeBackendWeights::MetalFast(weights) => {
                assert!(weights.packed_experts.is_none());
                assert!(weights.fallback_experts.is_some());
            }
            MoeBackendWeights::Slow(_) => panic!("expected metal fast backend"),
        }
    }

    #[test]
    fn metal_fast_backend_keeps_deferred_experts_as_fallback_layout() {
        let source = DeferredMoeLoadSource::new(
            vec![PathBuf::from("model.safetensors")],
            DType::F16,
            &Device::Cpu,
            LowPrecisionLoadPolicy::default(),
        );
        let backend = MoeBackendWeights::new(
            0,
            MoeExecutionBackend::MetalFast,
            vec![MoeExpertWeights::Deferred(
                DeferredDenseMlpWeights::new_expert(
                    source,
                    "model.layers.0.mlp.experts.0".to_string(),
                    2,
                    3,
                    0,
                    0,
                    LinearWeightMaterialization::DefaultContiguous,
                    DeferredExpertCachePolicy::Retain,
                ),
            )],
            None,
        )
        .expect("backend build should succeed");
        match backend {
            MoeBackendWeights::MetalFast(weights) => {
                assert!(weights.packed_experts.is_none());
                assert!(weights.fallback_experts.is_some());
                assert_eq!(weights.expert_count, 1);
            }
            MoeBackendWeights::Slow(_) => panic!("expected metal fast backend"),
        }
    }

    #[test]
    fn skip_shared_experts_flag_parses_truthy_and_falsy_values() {
        let _guard = env_lock().lock().expect("env lock poisoned");
        // SAFETY: tests serialize access to this process-wide env var through env_lock().
        unsafe {
            std::env::remove_var("XIUXIAN_VISION_SKIP_SHARED_EXPERTS");
        }
        assert!(!skip_shared_experts_enabled());

        // SAFETY: tests serialize access to this process-wide env var through env_lock().
        unsafe {
            std::env::set_var("XIUXIAN_VISION_SKIP_SHARED_EXPERTS", "yes");
        }
        assert!(skip_shared_experts_enabled());

        // SAFETY: tests serialize access to this process-wide env var through env_lock().
        unsafe {
            std::env::set_var("XIUXIAN_VISION_SKIP_SHARED_EXPERTS", "off");
        }
        assert!(!skip_shared_experts_enabled());

        // SAFETY: tests serialize access to this process-wide env var through env_lock().
        unsafe {
            std::env::remove_var("XIUXIAN_VISION_SKIP_SHARED_EXPERTS");
        }
    }

    #[test]
    fn load_shared_experts_short_circuits_when_skip_flag_is_enabled() {
        let _guard = env_lock().lock().expect("env lock poisoned");
        // SAFETY: tests serialize access to this process-wide env var through env_lock().
        unsafe {
            std::env::set_var("XIUXIAN_VISION_SKIP_SHARED_EXPERTS", "on");
        }
        let vb = VarBuilder::from_tensors(HashMap::new(), DType::F32, &Device::Cpu);
        let cfg: DeepseekV2Config = serde_json::from_value(serde_json::json!({
            "vocab_size": 32,
            "hidden_size": 2,
            "intermediate_size": 3,
            "num_hidden_layers": 1,
            "num_attention_heads": 1,
            "max_position_embeddings": 32,
            "hidden_act": "relu",
            "n_shared_experts": 2
        }))
        .expect("minimal shared-expert config should parse");
        let shared = load_shared_experts(0, &cfg, &vb, 2, 3, None, &Instant::now())
            .expect("skip flag should bypass shared-expert tensor lookup");
        assert!(shared.is_none());

        // SAFETY: tests serialize access to this process-wide env var through env_lock().
        unsafe {
            std::env::remove_var("XIUXIAN_VISION_SKIP_SHARED_EXPERTS");
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
}

#[test]
fn deferred_dense_mlp_weights_store_cache_policy() {
    let source = DeferredMoeLoadSource::new(
        vec![PathBuf::from("model.safetensors")],
        DType::F16,
        &Device::Cpu,
        LowPrecisionLoadPolicy::default(),
    );
    let deferred = DeferredDenseMlpWeights::new_expert(
        source,
        "model.layers.0.mlp.experts.0".to_string(),
        2,
        3,
        0,
        0,
        LinearWeightMaterialization::KeepViewForMetalFastPack,
        DeferredExpertCachePolicy::Ephemeral,
    );
    assert_eq!(
        deferred.cache_policy(),
        DeferredExpertCachePolicy::Ephemeral
    );
    assert_eq!(
        deferred.materialization(),
        LinearWeightMaterialization::KeepViewForMetalFastPack
    );
    assert!(!deferred.is_cached());
}
