use std::{
    env,
    io::{self, Write as _},
    time::Instant,
};

use crate::{
    config::DeepseekV2Config,
    quantization::run_quantized_matmul,
    transformer::{
        cache::{KvCacheChunk, KvCacheEntry},
        weights::{
            AttentionWeights, DenseMlpWeights, LinearWeights, MlpWeights, MoeBackendWeights,
            MoeExecutionBackend, MoeMetalFastExpertPack, MoeMetalFastWeights, MoeSlowWeights,
            MoeWeights, TransformerBlockWeights,
        },
    },
};
use anyhow::{Context, Result, bail, ensure};
use candle_core::{DType, Device, Tensor, shape::D};
#[cfg(feature = "flash-attn")]
use candle_flash_attn::flash_attn;
use candle_nn::ops::{rms_norm_slow, sigmoid, softmax};

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

fn is_expert_linear_label(label: &str) -> bool {
    label.contains(".mlp.experts.") || label.contains(".shared_experts.")
}

fn should_trace_expert_linear(label: &str) -> bool {
    stage_trace_enabled() && is_expert_linear_label(label)
}

fn is_low_precision(t: &Tensor) -> bool {
    matches!(t.dtype(), DType::F16 | DType::BF16)
}

fn moe_expert_f32_compute_enabled() -> bool {
    env::var("XIUXIAN_VISION_MOE_EXPERT_F32_COMPUTE")
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .and_then(|value| match value.as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        })
        .unwrap_or(true)
}

fn shared_expert_f32_compute_enabled(default: bool) -> bool {
    env::var("XIUXIAN_VISION_SHARED_EXPERT_F32_COMPUTE")
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .and_then(|value| match value.as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        })
        .unwrap_or(default)
}

fn prefill_attention_f32_enabled() -> bool {
    env::var("XIUXIAN_VISION_PREFILL_ATTENTION_F32")
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .and_then(|value| match value.as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        })
        .unwrap_or(true)
}

fn moe_combine_f32_enabled() -> bool {
    env::var("XIUXIAN_VISION_MOE_COMBINE_F32")
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .and_then(|value| match value.as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        })
        .unwrap_or(true)
}

fn moe_gate_input_f32_enabled() -> bool {
    env::var("XIUXIAN_VISION_MOE_GATE_INPUT_F32")
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .and_then(|value| match value.as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        })
        .unwrap_or(true)
}

fn skip_shared_experts_enabled() -> bool {
    env::var("XIUXIAN_VISION_SKIP_SHARED_EXPERTS")
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .and_then(|value| match value.as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        })
        .unwrap_or(false)
}

fn preferred_dense_compute_dtype(weights: &DenseMlpWeights) -> Option<DType> {
    weights
        .gate_proj
        .weight
        .as_ref()
        .map(Tensor::dtype)
        .filter(|dtype| matches!(dtype, DType::F16 | DType::BF16))
}

fn prepare_dense_mlp_input(
    hidden_states: &Tensor,
    weights: &DenseMlpWeights,
    use_f32_compute: bool,
) -> Result<Tensor> {
    if use_f32_compute {
        Ok(hidden_states.to_dtype(DType::F32)?)
    } else if let Some(dtype) = preferred_dense_compute_dtype(weights) {
        Ok(hidden_states.to_dtype(dtype)?)
    } else {
        Ok(hidden_states.clone())
    }
}
/// Low-precision (f16/bf16) can accumulate enough numeric error in sensitive
/// reductions to flip greedy argmax in near-tie steps. Keep these ops in f32,
/// then cast back to the working dtype.
fn rms_norm_stable(x: &Tensor, weight: &Tensor, eps: f32) -> Result<Tensor> {
    // Use f32 accumulation for RMSNorm to reduce drift in low precision.
    let x_f32 = x.to_dtype(DType::F32)?;
    let w_f32 = weight.to_dtype(DType::F32)?.contiguous()?;
    rms_norm_slow(&x_f32, &w_f32, eps).context("rms_norm f32 failed")
}

fn add_stable(a: &Tensor, b: &Tensor) -> Result<Tensor> {
    if matches!(a.dtype(), DType::F16 | DType::BF16) {
        let a_f32 = a.to_dtype(DType::F32)?;
        let b_f32 = b.to_dtype(DType::F32)?;
        Ok(a_f32
            .add(&b_f32)?
            .to_dtype(a.dtype())?
            .to_device(a.device())?)
    } else {
        Ok(a.add(b)?)
    }
}

fn preferred_packed_compute_dtype(weights: &MoeMetalFastExpertPack) -> Option<DType> {
    let dtype = weights.gate_proj.dtype();
    matches!(dtype, DType::F16 | DType::BF16).then_some(dtype)
}

fn prepare_packed_mlp_input(
    hidden_states: &Tensor,
    weights: &MoeMetalFastExpertPack,
    use_f32_compute: bool,
) -> Result<Tensor> {
    if use_f32_compute {
        return Ok(if hidden_states.dtype() == DType::F32 {
            hidden_states.clone()
        } else {
            hidden_states.to_dtype(DType::F32)?
        });
    }

    let Some(preferred_dtype) = preferred_packed_compute_dtype(weights) else {
        return Ok(hidden_states.clone());
    };
    if hidden_states.dtype() == preferred_dtype {
        Ok(hidden_states.clone())
    } else {
        Ok(hidden_states.to_dtype(preferred_dtype)?)
    }
}

fn select_packed_expert_weight(
    packed_weights: &Tensor,
    expert_idx: usize,
    out_dim: usize,
    in_dim: usize,
) -> Result<Tensor> {
    packed_weights
        .narrow(0, expert_idx, 1)?
        .reshape((out_dim, in_dim))?
        .contiguous()
        .context("failed to materialize packed expert weight")
}

/// Candle implementation of a single DeepSeek transformer decoder block (non-flash path).
///
/// This version supports dense MLP layers. Routed MoE layers return a `bail!` placeholder for now.
pub struct TransformerBlock<'a> {
    pub cfg: &'a DeepseekV2Config,
    pub weights: &'a TransformerBlockWeights,
    use_flash_attention: bool,
}

pub struct BlockOutput {
    pub hidden_states: Tensor,
    pub present_key_value: Option<KvCacheChunk>,
    pub aux_loss: Option<Tensor>,
}

struct MlpForwardOutput {
    hidden_states: Tensor,
    aux_loss: Option<Tensor>,
}

impl<'a> TransformerBlock<'a> {
    pub fn new(
        cfg: &'a DeepseekV2Config,
        weights: &'a TransformerBlockWeights,
        use_flash_attention: bool,
    ) -> Self {
        Self {
            cfg,
            weights,
            use_flash_attention,
        }
    }

    pub fn forward(
        &self,
        layer_idx: usize,
        hidden_states: &Tensor,
        additive_attn_bias: Option<&Tensor>,
        rope: Option<(&Tensor, &Tensor)>,
        past_key_value: Option<&KvCacheEntry>,
        use_cache: bool,
    ) -> Result<BlockOutput> {
        if is_low_precision(hidden_states) {
            let (_, seq_len, _) = hidden_states.shape().dims3()?;
            if seq_len == 1 {
                // Decode steps are the most sensitive to low-precision drift.
                // Keep the block in f32 and carry f32 residuals through decode.
                return self
                    .forward_internal_f32(
                        layer_idx,
                        hidden_states,
                        additive_attn_bias,
                        rope,
                        past_key_value,
                        use_cache,
                    )
                    .context("block forward (low precision decode f32) failed");
            }
            return self
                .forward_internal(
                    layer_idx,
                    hidden_states,
                    additive_attn_bias,
                    rope,
                    past_key_value,
                    use_cache,
                )
                .context("block forward (low precision) failed");
        }

        self.forward_internal(
            layer_idx,
            hidden_states,
            additive_attn_bias,
            rope,
            past_key_value,
            use_cache,
        )
    }

    fn forward_internal(
        &self,
        layer_idx: usize,
        hidden_states: &Tensor,
        additive_attn_bias: Option<&Tensor>,
        rope: Option<(&Tensor, &Tensor)>,
        past_key_value: Option<&KvCacheEntry>,
        use_cache: bool,
    ) -> Result<BlockOutput> {
        let block_started = Instant::now();
        let residual = hidden_states;
        let normed = rms_norm_stable(
            residual,
            &self.weights.input_layernorm.weight,
            self.cfg.rms_norm_eps,
        )?;
        let use_prefill_attention_f32 =
            !is_low_precision(residual) || prefill_attention_f32_enabled();
        let attention_input = if use_prefill_attention_f32 {
            normed.clone()
        } else {
            normed.to_dtype(residual.dtype())?
        };

        emit_stage_trace(
            "block.forward.attention.started",
            &[
                (
                    "elapsed_ms",
                    block_started.elapsed().as_millis().to_string(),
                ),
                ("layer_idx", layer_idx.to_string()),
                ("seq_len", hidden_states.dim(D::Minus2)?.to_string()),
                (
                    "prefill_attention_f32",
                    use_prefill_attention_f32.to_string(),
                ),
            ],
        );
        let (attn_out, present_cache) = attention_forward(
            &attention_input,
            &self.weights.attention,
            self.cfg,
            AttentionForwardOptions {
                additive_attn_bias,
                rope,
                past_key_value,
                use_cache,
                use_flash_attention: self.use_flash_attention,
                use_low_precision_f32: use_prefill_attention_f32,
            },
        )
        .context("attention forward failed")?;
        emit_stage_trace(
            "block.forward.attention.completed",
            &[
                (
                    "elapsed_ms",
                    block_started.elapsed().as_millis().to_string(),
                ),
                ("layer_idx", layer_idx.to_string()),
            ],
        );
        let hidden_states = add_stable(residual, &attn_out).context("residual add (attention)")?;

        let residual = &hidden_states;
        let mlp_kind = match &self.weights.mlp {
            MlpWeights::Dense(_) => "dense",
            MlpWeights::Moe(_) => "moe",
        };
        emit_stage_trace(
            "block.forward.mlp.started",
            &[
                (
                    "elapsed_ms",
                    block_started.elapsed().as_millis().to_string(),
                ),
                ("layer_idx", layer_idx.to_string()),
                ("mlp_kind", mlp_kind.to_string()),
            ],
        );
        let normed = rms_norm_stable(
            residual,
            &self.weights.post_attention_layernorm.weight,
            self.cfg.rms_norm_eps,
        )?;
        let (mlp_hidden, aux_loss) = if is_low_precision(residual) {
            match &self.weights.mlp {
                MlpWeights::Dense(dense) => (
                    run_dense_mlp_f32_keep(&normed.to_dtype(DType::F32)?, dense, self.cfg)?
                        .to_dtype(residual.dtype())?,
                    None,
                ),
                MlpWeights::Moe(moe) => (
                    run_moe(layer_idx, &normed.to_dtype(DType::F32)?, moe, self.cfg)?
                        .hidden_states
                        .to_dtype(DType::F32)?,
                    None,
                ),
            }
        } else {
            let MlpForwardOutput {
                hidden_states,
                aux_loss,
            } = mlp_forward(layer_idx, &normed, &self.weights.mlp, self.cfg)
                .context("mlp forward failed")?;
            (hidden_states, aux_loss)
        };
        emit_stage_trace(
            "block.forward.mlp.completed",
            &[
                (
                    "elapsed_ms",
                    block_started.elapsed().as_millis().to_string(),
                ),
                ("layer_idx", layer_idx.to_string()),
                ("mlp_kind", mlp_kind.to_string()),
            ],
        );

        let output = add_stable(residual, &mlp_hidden).context("residual add (mlp)")?;
        let present = if use_cache { present_cache } else { None };
        Ok(BlockOutput {
            hidden_states: output,
            present_key_value: present,
            aux_loss,
        })
    }

    fn forward_internal_f32(
        &self,
        layer_idx: usize,
        hidden_states: &Tensor,
        additive_attn_bias: Option<&Tensor>,
        rope: Option<(&Tensor, &Tensor)>,
        past_key_value: Option<&KvCacheEntry>,
        use_cache: bool,
    ) -> Result<BlockOutput> {
        let residual_f32 = hidden_states.to_dtype(DType::F32)?;
        let w_in = self
            .weights
            .input_layernorm
            .weight
            .to_dtype(DType::F32)?
            .contiguous()?;
        let normed_f32 = rms_norm_slow(&residual_f32, &w_in, self.cfg.rms_norm_eps)
            .context("input rms norm failed")?;

        let (attn_out, mut present_cache) = attention_forward_f32_keep(
            &normed_f32,
            &self.weights.attention,
            self.cfg,
            additive_attn_bias,
            rope,
            past_key_value,
            use_cache,
            hidden_states.dtype(),
        )
        .context("attention forward f32 keep failed")?;
        if let Some(chunk) = present_cache.as_mut() {
            chunk.key_t = chunk.key_t.contiguous()?;
            chunk.value = chunk.value.contiguous()?;
        }

        let post_attn = residual_f32
            .add(&attn_out)
            .context("residual add (attention)")?;

        let w_post = self
            .weights
            .post_attention_layernorm
            .weight
            .to_dtype(DType::F32)?
            .contiguous()?;
        let normed2 = rms_norm_slow(&post_attn, &w_post, self.cfg.rms_norm_eps)
            .context("post-attention rms norm failed")?;

        let (mlp_out_f32, aux_loss) = match &self.weights.mlp {
            MlpWeights::Dense(dense) => (run_dense_mlp_f32_keep(&normed2, dense, self.cfg)?, None),
            MlpWeights::Moe(moe) => {
                let out = run_moe(layer_idx, &normed2, moe, self.cfg)?
                    .hidden_states
                    .to_dtype(DType::F32)?;
                (out, None)
            }
        };

        let out_f32 = post_attn.add(&mlp_out_f32).context("residual add (mlp)")?;

        let (_, seq_len, _) = hidden_states.shape().dims3()?;
        let keep_f32_out = seq_len == 1;
        let out = if keep_f32_out {
            out_f32
        } else {
            out_f32.to_dtype(hidden_states.dtype())?
        };
        Ok(BlockOutput {
            hidden_states: out,
            present_key_value: if use_cache { present_cache } else { None },
            aux_loss,
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn attention_forward_f32_keep(
    hidden_states: &Tensor,
    weights: &AttentionWeights,
    cfg: &DeepseekV2Config,
    additive_attn_bias: Option<&Tensor>,
    rope: Option<(&Tensor, &Tensor)>,
    past_key_value: Option<&KvCacheEntry>,
    use_cache: bool,
    _cache_dtype: DType,
) -> Result<(Tensor, Option<KvCacheChunk>)> {
    let device = hidden_states.device();
    let (batch, seq_len, hidden_size) = hidden_states.shape().dims3()?;
    let head_dim = hidden_size / cfg.num_attention_heads;
    let v_head_dim_cfg = cfg.v_head_dim.unwrap_or(head_dim);
    let v_head_dim = if v_head_dim_cfg == 0 {
        head_dim
    } else {
        v_head_dim_cfg
    };
    let num_kv_heads = cfg.num_key_value_heads.unwrap_or(cfg.num_attention_heads);
    let kv_head_dim_cfg = cfg.qk_nope_head_dim.unwrap_or(head_dim);
    let kv_head_dim = if kv_head_dim_cfg == 0 {
        head_dim
    } else {
        kv_head_dim_cfg
    };

    let q = apply_linear_f32_keep(hidden_states, &weights.q_proj)?
        .reshape((batch, seq_len, cfg.num_attention_heads, head_dim))?
        .transpose(1, 2)?
        .contiguous()?;
    let k = apply_linear_f32_keep(hidden_states, &weights.k_proj)?
        .reshape((batch, seq_len, num_kv_heads, kv_head_dim))?
        .transpose(1, 2)?
        .contiguous()?;
    let v = apply_linear_f32_keep(hidden_states, &weights.v_proj)?
        .reshape((batch, seq_len, num_kv_heads, v_head_dim))?
        .transpose(1, 2)?
        .contiguous()?;

    let mut q = q;
    let mut k = k;
    if let Some((cos, sin)) = rope {
        let rope_dim_cfg = cfg.qk_rope_head_dim.unwrap_or(head_dim);
        let rope_dim = if rope_dim_cfg == 0 {
            head_dim
        } else {
            rope_dim_cfg
        };
        if rope_dim > 0 {
            let q_rot = q.narrow(D::Minus1, 0, rope_dim)?;
            let q_tail = if rope_dim < head_dim {
                Some(q.narrow(D::Minus1, rope_dim, head_dim - rope_dim)?)
            } else {
                None
            };
            let k_rot = k.narrow(D::Minus1, 0, rope_dim)?;
            let k_tail = if rope_dim < kv_head_dim {
                Some(k.narrow(D::Minus1, rope_dim, kv_head_dim - rope_dim)?)
            } else {
                None
            };
            let q_rot = apply_rope(
                &q_rot,
                &cos.to_dtype(DType::F32)?,
                &sin.to_dtype(DType::F32)?,
                cfg.use_mla,
            )?;
            let k_rot = apply_rope(
                &k_rot,
                &cos.to_dtype(DType::F32)?,
                &sin.to_dtype(DType::F32)?,
                cfg.use_mla,
            )?;
            q = if let Some(tail) = q_tail {
                Tensor::cat(&[q_rot, tail], D::Minus1)?
            } else {
                q_rot
            };
            k = if let Some(tail) = k_tail {
                Tensor::cat(&[k_rot, tail], D::Minus1)?
            } else {
                k_rot
            };
            q = q.contiguous()?;
            k = k.contiguous()?;
        }
    }

    ensure!(cfg.num_attention_heads.is_multiple_of(num_kv_heads));
    let repeats = cfg.num_attention_heads / num_kv_heads;
    let k_new = repeat_kv(&k, repeats)?.contiguous()?;
    let v_new = repeat_kv(&v, repeats)?.contiguous()?;

    let mut cache_key_t_view: Option<Tensor> = None;
    let mut cache_value_view: Option<Tensor> = None;
    let past_len = if let Some(cache) = past_key_value {
        let past_len = cache.seq_len();
        if past_len > 0 {
            let key_view = cache.key_view()?.contiguous()?;
            let value_view = cache.value_view()?.contiguous()?;
            cache_key_t_view = Some(key_view.to_dtype(DType::F32)?);
            cache_value_view = Some(value_view.to_dtype(DType::F32)?);
        }
        past_len
    } else {
        0
    };

    let k_new_t = transpose(&k_new, 2, 3)?;

    let scores_new = q.matmul(&k_new_t)?;
    let attn_scores_mat = if let Some(cache_key_t) = cache_key_t_view.as_ref() {
        if past_len > 0 {
            let scores_past = q.matmul(cache_key_t)?;
            Tensor::cat(&[scores_past, scores_new], D::Minus1)?
        } else {
            scores_new
        }
    } else {
        scores_new
    };

    let scale = (head_dim as f64).sqrt();
    let mut attn_scores = (attn_scores_mat / scale)?;
    if let Some(bias) = additive_attn_bias {
        let bias = bias.to_dtype(DType::F32)?;
        let bias = bias.broadcast_as(attn_scores.shape().dims())?;
        attn_scores = attn_scores.broadcast_add(&bias)?;
    }
    let attn_weights = softmax(&attn_scores, D::Minus1).context("attention softmax f32 failed")?;

    let out_f32 = if let Some(cache_value) = cache_value_view.as_ref() {
        if past_len > 0 {
            let w_past = attn_weights.narrow(D::Minus1, 0, past_len)?;
            let w_new = attn_weights.narrow(D::Minus1, past_len, seq_len)?;
            let past_out = w_past.matmul(cache_value)?;
            let new_out = w_new.matmul(&v_new)?;
            past_out.add(&new_out)?
        } else {
            attn_weights.matmul(&v_new)?
        }
    } else {
        attn_weights.matmul(&v_new)?
    };

    let present = if use_cache {
        let store_dtype = DType::F32;
        let (k_store, v_store) = if store_dtype == DType::F32 {
            (k_new_t.to_dtype(DType::F32)?, v_new.to_dtype(DType::F32)?)
        } else {
            (k_new_t.to_dtype(store_dtype)?, v_new.to_dtype(store_dtype)?)
        };
        Some(KvCacheChunk::new(k_store, v_store)?)
    } else {
        None
    };

    let attn_output = out_f32.permute((0, 2, 1, 3))?.reshape((
        batch,
        seq_len,
        cfg.num_attention_heads * v_head_dim,
    ))?;

    let out = apply_linear_f32_keep(&attn_output, &weights.o_proj)?;
    let out = out.to_device(device)?;
    Ok((out, present))
}

struct AttentionForwardOptions<'a> {
    additive_attn_bias: Option<&'a Tensor>,
    rope: Option<(&'a Tensor, &'a Tensor)>,
    past_key_value: Option<&'a KvCacheEntry>,
    use_cache: bool,
    use_flash_attention: bool,
    use_low_precision_f32: bool,
}

fn attention_forward(
    hidden_states: &Tensor,
    weights: &AttentionWeights,
    cfg: &DeepseekV2Config,
    options: AttentionForwardOptions<'_>,
) -> Result<(Tensor, Option<KvCacheChunk>)> {
    if cfg.q_lora_rank.is_some() || cfg.kv_lora_rank.is_some() {
        bail!("LoRA attention path not yet implemented");
    }

    if options.use_flash_attention
        && let Some(result) = flash_attention_forward(
            hidden_states,
            weights,
            cfg,
            options.rope,
            options.additive_attn_bias,
            options.past_key_value,
            options.use_cache,
        )?
    {
        return Ok(result);
    }

    let (batch, seq_len, hidden_size) = hidden_states
        .shape()
        .dims3()
        .context("attention expects hidden_states with shape [batch, seq, hidden]")?;
    if hidden_size != cfg.hidden_size {
        bail!(
            "config hidden_size {} does not match tensor hidden dim {}",
            cfg.hidden_size,
            hidden_size
        );
    }

    let head_dim = hidden_size / cfg.num_attention_heads;
    let num_kv_heads = cfg.num_key_value_heads.unwrap_or(cfg.num_attention_heads);
    let kv_head_dim = head_dim;
    let v_head_dim = if cfg.v_head_dim.unwrap_or(0) == 0 {
        head_dim
    } else {
        cfg.v_head_dim.unwrap()
    };

    let use_attn_f32 = is_low_precision(hidden_states);
    let use_attn_f32 = use_attn_f32 && options.use_low_precision_f32;
    let use_f32_projection = use_attn_f32 || hidden_states.dtype() == DType::F32;
    let use_cache_f32 = matches!(hidden_states.dtype(), DType::F16 | DType::BF16) && use_attn_f32;

    // Query / key / value projections.
    let mut q = if use_f32_projection {
        apply_linear_f32_keep(hidden_states, &weights.q_proj)?
    } else {
        apply_linear(hidden_states, &weights.q_proj)?
    }
    .reshape((batch, seq_len, cfg.num_attention_heads, head_dim))?;
    let mut k = if use_f32_projection {
        apply_linear_f32_keep(hidden_states, &weights.k_proj)?
    } else {
        apply_linear(hidden_states, &weights.k_proj)?
    }
    .reshape((batch, seq_len, num_kv_heads, kv_head_dim))?;
    let v = if use_f32_projection {
        apply_linear_f32_keep(hidden_states, &weights.v_proj)?
    } else {
        apply_linear(hidden_states, &weights.v_proj)?
    }
    .reshape((batch, seq_len, num_kv_heads, v_head_dim))?;

    q = q.permute((0, 2, 1, 3))?;
    k = k.permute((0, 2, 1, 3))?;
    let v = v.permute((0, 2, 1, 3))?;

    let rope_dim = cfg.qk_rope_head_dim.unwrap_or(head_dim);
    let rope_dim = if rope_dim == 0 { head_dim } else { rope_dim };
    ensure!(
        rope_dim <= head_dim,
        "rope dimension {} exceeds q head dimension {}",
        rope_dim,
        head_dim
    );
    ensure!(
        rope_dim <= kv_head_dim,
        "rope dimension {} exceeds k head dimension {}",
        rope_dim,
        kv_head_dim
    );
    if rope_dim > 0 {
        let (cos, sin) = options.rope.context("missing rope tensors for attention")?;
        ensure!(
            cos.shape().dims() == [batch, 1, seq_len, rope_dim],
            "cos shape {:?} incompatible with (batch={}, seq={}, rope_dim={})",
            cos.shape().dims(),
            batch,
            seq_len,
            rope_dim
        );
        ensure!(
            sin.shape().dims() == [batch, 1, seq_len, rope_dim],
            "sin shape {:?} incompatible with (batch={}, seq={}, rope_dim={})",
            sin.shape().dims(),
            batch,
            seq_len,
            rope_dim
        );
        let cos = if use_f32_projection {
            cos.to_dtype(DType::F32)?
        } else {
            cos.clone()
        };
        let sin = if use_f32_projection {
            sin.to_dtype(DType::F32)?
        } else {
            sin.clone()
        };
        if use_f32_projection {
            q = q.to_dtype(DType::F32)?;
            k = k.to_dtype(DType::F32)?;
        }
        let q_rot = q.narrow(D::Minus1, 0, rope_dim)?;
        let k_rot = k.narrow(D::Minus1, 0, rope_dim)?;
        let q_tail = if rope_dim < head_dim {
            Some(q.narrow(D::Minus1, rope_dim, head_dim - rope_dim)?)
        } else {
            None
        };
        let k_tail = if rope_dim < kv_head_dim {
            Some(k.narrow(D::Minus1, rope_dim, kv_head_dim - rope_dim)?)
        } else {
            None
        };
        let q_rot = apply_rope(&q_rot, &cos, &sin, cfg.use_mla)?;
        let k_rot = apply_rope(&k_rot, &cos, &sin, cfg.use_mla)?;
        q = if let Some(tail) = q_tail {
            Tensor::cat(&[q_rot, tail], D::Minus1)?
        } else {
            q_rot
        };
        k = if let Some(tail) = k_tail {
            Tensor::cat(&[k_rot, tail], D::Minus1)?
        } else {
            k_rot
        };
    }

    ensure!(
        cfg.num_attention_heads.is_multiple_of(num_kv_heads),
        "num_attention_heads {} must be divisible by num_key_value_heads {}",
        cfg.num_attention_heads,
        num_kv_heads
    );
    let repeats = cfg.num_attention_heads / num_kv_heads;
    let mut k_new = repeat_kv(&k, repeats)?;
    let mut v_new = repeat_kv(&v, repeats)?;
    if use_attn_f32 {
        v_new = v_new.to_dtype(DType::F32)?;
    }

    q = q.contiguous()?;
    k_new = k_new.contiguous()?;
    v_new = v_new.contiguous()?;

    let mut cache_key_t_view: Option<Tensor> = None;
    let mut cache_value_view: Option<Tensor> = None;
    let past_len = if let Some(cache) = options.past_key_value {
        let key_view = cache.key_view()?;
        let value_view = cache.value_view()?;
        let (cache_batch, cache_heads, cache_dim, _) = key_view
            .shape()
            .dims4()
            .context("cache key tensor must be 4D")?;
        ensure!(
            cache_batch == batch,
            "cache batch {} does not match current batch {}",
            cache_batch,
            batch
        );
        ensure!(
            cache_heads == cfg.num_attention_heads,
            "cache heads {} does not match attention heads {}",
            cache_heads,
            cfg.num_attention_heads
        );
        ensure!(
            cache_dim == kv_head_dim,
            "cache key head dim {} does not match kv_head_dim {}",
            cache_dim,
            kv_head_dim
        );
        let value_dims = value_view.shape().dims();
        ensure!(
            value_dims[0] == batch,
            "cache value batch {} does not match current batch {}",
            value_dims[0],
            batch
        );
        ensure!(
            value_dims[1] == cfg.num_attention_heads,
            "cache value heads {} does not match attention heads {}",
            value_dims[1],
            cfg.num_attention_heads
        );
        ensure!(
            value_dims[3] == v_head_dim,
            "cache value head dim {} does not match v_head_dim {}",
            value_dims[3],
            v_head_dim
        );
        cache_key_t_view = Some(key_view);
        cache_value_view = Some(value_view);
        cache.seq_len()
    } else {
        0
    };

    let k_new_t = transpose(&k_new, 2, 3)?.contiguous()?;
    let attn_scores_mat = if use_attn_f32 {
        // f16->f32: attention score matmul can be extremely sensitive.
        let q_f32 = q.to_dtype(DType::F32)?;
        let k_new_t_f32 = k_new_t.to_dtype(DType::F32)?;
        if let Some(cache_key_t) = cache_key_t_view.as_ref() {
            let scores_new = q_f32.matmul(&k_new_t_f32)?;
            if past_len > 0 {
                let cache_key_t_f32 = cache_key_t.contiguous()?.to_dtype(DType::F32)?;
                let scores_past = q_f32.matmul(&cache_key_t_f32)?;
                Tensor::cat(&[scores_past, scores_new], D::Minus1)?
            } else {
                scores_new
            }
        } else {
            q_f32.matmul(&k_new_t_f32)?
        }
    } else if let Some(cache_key_t) = cache_key_t_view.as_ref() {
        let k_new_t_f16 = k_new_t.to_dtype(q.dtype())?;
        let scores_new = q.matmul(&k_new_t_f16)?;
        if past_len > 0 {
            let cache_key_t = if q.dtype() == DType::F32 {
                cache_key_t.contiguous()?.to_dtype(DType::F32)?
            } else {
                cache_key_t.contiguous()?.to_dtype(q.dtype())?
            };
            let scores_past = q.matmul(&cache_key_t)?;
            Tensor::cat(&[scores_past, scores_new], D::Minus1)?
        } else {
            scores_new
        }
    } else {
        let k_new_t_f16 = k_new_t.to_dtype(q.dtype())?;
        q.matmul(&k_new_t_f16)?
    };

    let scale = (head_dim as f64).sqrt();
    let mut attn_scores = (attn_scores_mat / scale)?;
    if let Some(bias) = options.additive_attn_bias {
        let bias = if bias.dtype() != attn_scores.dtype() {
            bias.to_dtype(attn_scores.dtype())?
        } else {
            bias.clone()
        };
        let bias = bias.broadcast_as(attn_scores.shape().dims())?;
        attn_scores = attn_scores.broadcast_add(&bias)?;
    }
    if use_attn_f32 {
        // Keep attention scores in f32 before softmax to reduce near-tie drift.
        attn_scores = attn_scores.to_dtype(DType::F32)?;
    }
    let attn_weights = if use_attn_f32 {
        // Keep softmax in f32 for stability; downstream matmul decides whether to cast.
        softmax(&attn_scores.to_dtype(DType::F32)?, D::Minus1)
            .context("attention softmax failed")?
    } else {
        softmax(&attn_scores, D::Minus1).context("attention softmax failed")?
    };

    // Keep (attn_weights @ values) matmul in f32 when we upcasted softmax.
    let attn_output = if attn_weights.dtype() == DType::F32 && use_attn_f32 {
        let v_new_f32 = v_new.to_dtype(DType::F32)?;
        if let Some(cache_value_view) = cache_value_view.as_ref() {
            let accum = if past_len > 0 {
                let cache_value_f32 = cache_value_view.contiguous()?.to_dtype(DType::F32)?;
                Some(
                    attn_weights
                        .narrow(D::Minus1, 0, past_len)?
                        .matmul(&cache_value_f32)?,
                )
            } else {
                None
            };
            let contrib_new = attn_weights
                .narrow(D::Minus1, past_len, seq_len)?
                .matmul(&v_new_f32)?;
            if let Some(existing) = accum {
                existing.add(&contrib_new)?
            } else {
                contrib_new
            }
        } else {
            attn_weights.matmul(&v_new_f32)?
        }
    } else {
        let v_new = if v_new.dtype() == attn_weights.dtype() {
            v_new.clone()
        } else {
            v_new.to_dtype(attn_weights.dtype())?
        };
        if let Some(cache_value_view) = cache_value_view.as_ref() {
            let accum = if past_len > 0 {
                let cache_value = cache_value_view
                    .contiguous()?
                    .to_dtype(attn_weights.dtype())?;
                Some(
                    attn_weights
                        .narrow(D::Minus1, 0, past_len)?
                        .matmul(&cache_value)?,
                )
            } else {
                None
            };
            let contrib_new = attn_weights
                .narrow(D::Minus1, past_len, seq_len)?
                .matmul(&v_new)?;
            if let Some(existing) = accum {
                existing.add(&contrib_new)?
            } else {
                contrib_new
            }
        } else {
            attn_weights.matmul(&v_new)?
        }
    };
    let present = if options.use_cache {
        let store_dtype = if use_cache_f32 || use_attn_f32 {
            DType::F32
        } else {
            k_new_t.dtype()
        };
        let (k_store, v_store) = if store_dtype == k_new_t.dtype() {
            (k_new_t.clone(), v_new.clone())
        } else {
            (k_new_t.to_dtype(store_dtype)?, v_new.to_dtype(store_dtype)?)
        };
        Some(KvCacheChunk::new(k_store, v_store)?)
    } else {
        None
    };
    let attn_output = attn_output.permute((0, 2, 1, 3))?.reshape((
        batch,
        seq_len,
        cfg.num_attention_heads * v_head_dim,
    ))?;

    let out = if use_attn_f32 {
        // Output projection is another sensitive reduction path; keep it in f32.
        apply_linear_f32_then_cast(&attn_output, &weights.o_proj, hidden_states.dtype())?
    } else {
        apply_linear(&attn_output, &weights.o_proj)?
    };
    Ok((out, present))
}

fn flash_attention_forward(
    hidden_states: &Tensor,
    weights: &AttentionWeights,
    cfg: &DeepseekV2Config,
    rope: Option<(&Tensor, &Tensor)>,
    additive_attn_bias: Option<&Tensor>,
    past_key_value: Option<&KvCacheEntry>,
    use_cache: bool,
) -> Result<Option<(Tensor, Option<KvCacheChunk>)>> {
    #[cfg(not(feature = "flash-attn"))]
    {
        let _ = (
            hidden_states,
            weights,
            cfg,
            rope,
            additive_attn_bias,
            past_key_value,
            use_cache,
        );
        Ok(None)
    }
    #[cfg(feature = "flash-attn")]
    {
        if additive_attn_bias.is_some() || past_key_value.is_some() || use_cache {
            return Ok(None);
        }
        let device = hidden_states.device();
        if !device.is_cuda() {
            return Ok(None);
        }
        let (batch, seq_len, hidden_size) = hidden_states.shape().dims3()?;
        let dtype = hidden_states.dtype();
        match dtype {
            DType::F16 | DType::BF16 => {}
            _ => return Ok(None),
        }
        let head_dim = hidden_size / cfg.num_attention_heads;
        if !head_dim.is_multiple_of(8) || head_dim > 256 {
            return Ok(None);
        }
        let num_kv_heads = cfg.num_key_value_heads.unwrap_or(cfg.num_attention_heads);
        if !cfg.num_attention_heads.is_multiple_of(num_kv_heads) {
            return Ok(None);
        }

        let mut q = apply_linear(hidden_states, &weights.q_proj)?
            .reshape((batch, seq_len, cfg.num_attention_heads, head_dim))?
            .to_dtype(dtype)?
            .to_device(device)?;
        let kv_head_dim = head_dim;
        let mut k = apply_linear(hidden_states, &weights.k_proj)?
            .reshape((batch, seq_len, num_kv_heads, kv_head_dim))?
            .to_dtype(dtype)?
            .to_device(device)?;
        let v_head_dim = if cfg.v_head_dim.unwrap_or(0) == 0 {
            head_dim
        } else {
            cfg.v_head_dim.unwrap()
        };
        let v = apply_linear(hidden_states, &weights.v_proj)?
            .reshape((batch, seq_len, num_kv_heads, v_head_dim))?
            .to_dtype(dtype)?
            .to_device(device)?;

        if let Some((cos, sin)) = rope {
            let (cos, sin) = (cos.to_device(device)?, sin.to_device(device)?);
            let rope_dim_cfg = cfg.qk_rope_head_dim.unwrap_or(head_dim);
            let rope_dim = if rope_dim_cfg == 0 {
                head_dim
            } else {
                rope_dim_cfg
            };
            ensure!(
                rope_dim <= head_dim,
                "rope dimension {} exceeds q head dimension {}",
                rope_dim,
                head_dim
            );
            ensure!(
                rope_dim <= kv_head_dim,
                "rope dimension {} exceeds k head dimension {}",
                rope_dim,
                kv_head_dim
            );
            ensure!(
                cos.shape().dims() == [batch, 1, seq_len, rope_dim],
                "cos shape {:?} incompatible with (batch={}, seq={}, rope_dim={})",
                cos.shape().dims(),
                batch,
                seq_len,
                rope_dim
            );
            ensure!(
                sin.shape().dims() == [batch, 1, seq_len, rope_dim],
                "sin shape {:?} incompatible with (batch={}, seq={}, rope_dim={})",
                sin.shape().dims(),
                batch,
                seq_len,
                rope_dim
            );
            let q_rot = q.narrow(D::Minus1, 0, rope_dim)?;
            let q_tail = if rope_dim < head_dim {
                Some(q.narrow(D::Minus1, rope_dim, head_dim - rope_dim)?)
            } else {
                None
            };
            let k_rot = k.narrow(D::Minus1, 0, rope_dim)?;
            let k_tail = if rope_dim < kv_head_dim {
                Some(k.narrow(D::Minus1, rope_dim, kv_head_dim - rope_dim)?)
            } else {
                None
            };
            let q_rot = apply_rope(&q_rot, &cos, &sin, cfg.use_mla)?;
            let k_rot = apply_rope(&k_rot, &cos, &sin, cfg.use_mla)?;
            q = if let Some(tail) = q_tail {
                Tensor::cat(&[q_rot, tail], D::Minus1)?
            } else {
                q_rot
            }
            .contiguous()?;
            k = if let Some(tail) = k_tail {
                Tensor::cat(&[k_rot, tail], D::Minus1)?
            } else {
                k_rot
            }
            .contiguous()?;
        }

        q = q.contiguous()?;
        k = k.contiguous()?;
        let v = v.contiguous()?;
        let causal = true;
        let scale = 1.0 / (head_dim as f32).sqrt();
        let q = q.transpose(1, 2)?;
        let k = k.transpose(1, 2)?;
        let v_t = v.transpose(1, 2)?;
        let attn = flash_attn(&q, &k, &v_t, scale, causal)?;
        let attn = attn.transpose(1, 2)?.reshape((
            batch,
            seq_len,
            cfg.num_attention_heads * v_head_dim,
        ))?;
        let out = apply_linear(&attn, &weights.o_proj)?;
        Ok(Some((out, None)))
    }
}

fn mlp_forward(
    layer_idx: usize,
    hidden_states: &Tensor,
    weights: &MlpWeights,
    cfg: &DeepseekV2Config,
) -> Result<MlpForwardOutput> {
    match weights {
        MlpWeights::Dense(dense) => run_dense_mlp(layer_idx, hidden_states, dense, cfg),
        MlpWeights::Moe(moe) => run_moe(layer_idx, hidden_states, moe, cfg),
    }
}

fn apply_linear(input: &Tensor, weights: &LinearWeights) -> Result<Tensor> {
    let linear_started = Instant::now();
    let dims = input.shape().dims();
    if dims.len() < 2 {
        bail!("linear expects rank >= 2, received {:?}", dims);
    }
    let last_dim = *dims.last().expect("at least one dim");
    let (out_dim, in_dim) = (weights.out_dim, weights.in_dim);
    if in_dim != last_dim {
        bail!(
            "linear weight expects input dim {}, got {}",
            in_dim,
            last_dim
        );
    }

    let leading = dims[..dims.len() - 1].iter().product::<usize>();
    let input2d = input.reshape((leading, in_dim))?.contiguous()?;
    if should_trace_expert_linear(&weights.label) {
        emit_stage_trace(
            "block.forward.moe.linear.started",
            &[
                (
                    "elapsed_ms",
                    linear_started.elapsed().as_millis().to_string(),
                ),
                ("label", weights.label.clone()),
                ("rows", leading.to_string()),
                ("in_dim", in_dim.to_string()),
                ("out_dim", out_dim.to_string()),
                ("input_dtype", format!("{:?}", input2d.dtype())),
                (
                    "has_preloaded_weight_f32",
                    weights.weight_f32.is_some().to_string(),
                ),
            ],
        );
    }
    let proj = if let Some(qm) = &weights.qmatmul {
        run_quantized_matmul(&weights.label, qm, &input2d)?
    } else {
        let weight = weights
            .weight
            .as_ref()
            .context("float linear weight missing for non-quantized layer")?;
        let weight = if input2d.dtype() == DType::F32 {
            if let Some(weight_f32) = &weights.weight_f32 {
                weight_f32.clone()
            } else if weight.dtype() != DType::F32 {
                if should_trace_expert_linear(&weights.label) {
                    emit_stage_trace(
                        "block.forward.moe.linear.weight.materialize.started",
                        &[
                            (
                                "elapsed_ms",
                                linear_started.elapsed().as_millis().to_string(),
                            ),
                            ("label", weights.label.clone()),
                            ("weight_dtype", format!("{:?}", weight.dtype())),
                            ("target_dtype", "F32".to_string()),
                        ],
                    );
                }
                let materialized = weight.to_dtype(DType::F32)?;
                if should_trace_expert_linear(&weights.label) {
                    emit_stage_trace(
                        "block.forward.moe.linear.weight.materialize.completed",
                        &[
                            (
                                "elapsed_ms",
                                linear_started.elapsed().as_millis().to_string(),
                            ),
                            ("label", weights.label.clone()),
                        ],
                    );
                }
                materialized
            } else {
                weight.clone()
            }
        } else if weight.dtype() != input2d.dtype() {
            if should_trace_expert_linear(&weights.label) {
                emit_stage_trace(
                    "block.forward.moe.linear.weight.materialize.started",
                    &[
                        (
                            "elapsed_ms",
                            linear_started.elapsed().as_millis().to_string(),
                        ),
                        ("label", weights.label.clone()),
                        ("weight_dtype", format!("{:?}", weight.dtype())),
                        ("target_dtype", format!("{:?}", input2d.dtype())),
                    ],
                );
            }
            let materialized = weight.to_dtype(input2d.dtype())?;
            if should_trace_expert_linear(&weights.label) {
                emit_stage_trace(
                    "block.forward.moe.linear.weight.materialize.completed",
                    &[
                        (
                            "elapsed_ms",
                            linear_started.elapsed().as_millis().to_string(),
                        ),
                        ("label", weights.label.clone()),
                    ],
                );
            }
            materialized
        } else {
            weight.clone()
        };
        let weight = weight.contiguous()?;
        if should_trace_expert_linear(&weights.label) {
            emit_stage_trace(
                "block.forward.moe.linear.matmul.started",
                &[
                    (
                        "elapsed_ms",
                        linear_started.elapsed().as_millis().to_string(),
                    ),
                    ("label", weights.label.clone()),
                    ("weight_dtype", format!("{:?}", weight.dtype())),
                ],
            );
        }
        input2d.matmul(&transpose(&weight, 0, 1)?)?
    };
    if should_trace_expert_linear(&weights.label) {
        emit_stage_trace(
            "block.forward.moe.linear.matmul.completed",
            &[
                (
                    "elapsed_ms",
                    linear_started.elapsed().as_millis().to_string(),
                ),
                ("label", weights.label.clone()),
            ],
        );
    }
    let proj = if let Some(bias) = &weights.bias {
        let bias = if bias.dtype() != proj.dtype() {
            bias.to_dtype(proj.dtype())?
        } else {
            bias.clone()
        };
        proj.broadcast_add(&bias.reshape((1, out_dim))?)?
    } else {
        proj
    };
    proj.reshape(
        dims[..dims.len() - 1]
            .iter()
            .copied()
            .chain(std::iter::once(out_dim))
            .collect::<Vec<_>>(),
    )
    .context("failed to reshape linear output")
}

fn apply_linear_f32_then_cast(
    input: &Tensor,
    weights: &LinearWeights,
    out_dtype: DType,
) -> Result<Tensor> {
    let dims = input.shape().dims();
    if dims.len() < 2 {
        bail!("linear expects rank >= 2, received {:?}", dims);
    }
    let last_dim = *dims.last().expect("at least one dim");
    let (out_dim, in_dim) = (weights.out_dim, weights.in_dim);
    if in_dim != last_dim {
        bail!(
            "linear weight expects input dim {}, got {}",
            in_dim,
            last_dim
        );
    }

    // Quantized path: keep existing behaviour.
    if let Some(qm) = &weights.qmatmul {
        let leading = dims[..dims.len() - 1].iter().product::<usize>();
        let input2d = input.reshape((leading, in_dim))?.contiguous()?;
        let proj = run_quantized_matmul(&weights.label, qm, &input2d)?;
        let proj = if let Some(bias) = &weights.bias {
            proj.broadcast_add(&bias.reshape((1, out_dim))?)?
        } else {
            proj
        };
        return Ok(proj
            .reshape(
                dims[..dims.len() - 1]
                    .iter()
                    .copied()
                    .chain(std::iter::once(out_dim))
                    .collect::<Vec<_>>(),
            )?
            .to_dtype(out_dtype)?);
    }

    let leading = dims[..dims.len() - 1].iter().product::<usize>();
    let x2d = input
        .to_dtype(DType::F32)?
        .reshape((leading, in_dim))?
        .contiguous()?;
    let weight = weights
        .weight
        .as_ref()
        .context("float linear weight missing for non-quantized layer")?
        .to_dtype(DType::F32)?
        .contiguous()?;
    let mut proj = x2d.matmul(&transpose(&weight, 0, 1)?)?;
    if let Some(bias) = &weights.bias {
        let bias = bias.to_dtype(DType::F32)?;
        proj = proj.broadcast_add(&bias.reshape((1, out_dim))?)?;
    }
    Ok(proj
        .reshape(
            dims[..dims.len() - 1]
                .iter()
                .copied()
                .chain(std::iter::once(out_dim))
                .collect::<Vec<_>>(),
        )?
        .to_dtype(out_dtype)?)
}

fn apply_linear_f32_keep(input: &Tensor, weights: &LinearWeights) -> Result<Tensor> {
    let linear_started = Instant::now();
    let dims = input.shape().dims();
    if dims.len() < 2 {
        bail!("linear expects rank >= 2, received {:?}", dims);
    }
    let last_dim = *dims.last().expect("at least one dim");
    let (out_dim, in_dim) = (weights.out_dim, weights.in_dim);
    if in_dim != last_dim {
        bail!(
            "linear weight expects input dim {}, got {}",
            in_dim,
            last_dim
        );
    }
    let leading = dims[..dims.len() - 1].iter().product::<usize>();
    let input2d = input.reshape((leading, in_dim))?.contiguous()?;
    if should_trace_expert_linear(&weights.label) {
        emit_stage_trace(
            "block.forward.moe.linear.started",
            &[
                (
                    "elapsed_ms",
                    linear_started.elapsed().as_millis().to_string(),
                ),
                ("label", weights.label.clone()),
                ("rows", leading.to_string()),
                ("in_dim", in_dim.to_string()),
                ("out_dim", out_dim.to_string()),
                (
                    "has_preloaded_weight_f32",
                    weights.weight_f32.is_some().to_string(),
                ),
            ],
        );
    }

    let proj = if let Some(qm) = &weights.qmatmul {
        run_quantized_matmul(&weights.label, qm, &input2d)?.to_dtype(DType::F32)?
    } else {
        let weight = weights
            .weight
            .as_ref()
            .context("float linear weight missing for non-quantized layer")?;
        let x = input2d.to_dtype(DType::F32)?;
        let w = if let Some(w_f32) = &weights.weight_f32 {
            w_f32.clone()
        } else {
            if should_trace_expert_linear(&weights.label) {
                emit_stage_trace(
                    "block.forward.moe.linear.weight.materialize.started",
                    &[
                        (
                            "elapsed_ms",
                            linear_started.elapsed().as_millis().to_string(),
                        ),
                        ("label", weights.label.clone()),
                    ],
                );
            }
            let materialized = weight.to_dtype(DType::F32)?;
            if should_trace_expert_linear(&weights.label) {
                emit_stage_trace(
                    "block.forward.moe.linear.weight.materialize.completed",
                    &[
                        (
                            "elapsed_ms",
                            linear_started.elapsed().as_millis().to_string(),
                        ),
                        ("label", weights.label.clone()),
                    ],
                );
            }
            materialized
        };
        let w = w.contiguous()?;
        if should_trace_expert_linear(&weights.label) {
            emit_stage_trace(
                "block.forward.moe.linear.matmul.started",
                &[
                    (
                        "elapsed_ms",
                        linear_started.elapsed().as_millis().to_string(),
                    ),
                    ("label", weights.label.clone()),
                ],
            );
        }
        x.matmul(&transpose(&w, 0, 1)?)?
    };
    if should_trace_expert_linear(&weights.label) {
        emit_stage_trace(
            "block.forward.moe.linear.matmul.completed",
            &[
                (
                    "elapsed_ms",
                    linear_started.elapsed().as_millis().to_string(),
                ),
                ("label", weights.label.clone()),
            ],
        );
    }

    let proj = if let Some(bias) = &weights.bias {
        let bias = bias.to_dtype(DType::F32)?;
        proj.broadcast_add(&bias.reshape((1, out_dim))?)?
    } else {
        proj
    };

    proj.reshape(
        dims[..dims.len() - 1]
            .iter()
            .copied()
            .chain(std::iter::once(out_dim))
            .collect::<Vec<_>>(),
    )
    .context("failed to reshape linear output")
}

fn repeat_kv(t: &Tensor, repeats: usize) -> Result<Tensor> {
    if repeats == 0 {
        bail!("repeat_kv expects repeats >= 1");
    }
    if repeats == 1 {
        return Ok(t.clone());
    }
    let (batch, heads, seq_len, dim) = t
        .shape()
        .dims4()
        .context("expected [batch, heads, seq, dim] tensor")?;
    let expanded = t
        .unsqueeze(2)?
        .expand((batch, heads, repeats, seq_len, dim))?
        .reshape((batch, heads * repeats, seq_len, dim))?;
    Ok(expanded.contiguous()?)
}

fn apply_activation(input: &Tensor, name: &str) -> Result<Tensor> {
    let normalized = name.to_ascii_lowercase();
    match normalized.as_str() {
        "silu" | "swish" => Ok(input.silu()?),
        "relu" => Ok(input.relu()?),
        // Match PyTorch `nn.GELU()` default (approximate="none").
        "gelu" => Ok(input.gelu_erf()?),
        "gelu_erf" => Ok(input.gelu_erf()?),
        _ => bail!("activation `{name}` not implemented"),
    }
}

fn run_dense_mlp_f32_keep(
    hidden_states: &Tensor,
    weights: &DenseMlpWeights,
    cfg: &DeepseekV2Config,
) -> Result<Tensor> {
    let gate = apply_linear_f32_keep(hidden_states, &weights.gate_proj)?;
    let up = apply_linear_f32_keep(hidden_states, &weights.up_proj)?;
    let activated = apply_activation(&gate, &cfg.hidden_act)
        .with_context(|| format!("unsupported activation {}", cfg.hidden_act))?;
    let fused = activated.broadcast_mul(&up)?;
    apply_linear_f32_keep(&fused, &weights.down_proj)
}

fn run_dense_mlp_with_policy(
    hidden_states: &Tensor,
    weights: &DenseMlpWeights,
    cfg: &DeepseekV2Config,
    use_f32: bool,
) -> Result<MlpForwardOutput> {
    let (gate, up) = if use_f32 {
        (
            apply_linear_f32_keep(hidden_states, &weights.gate_proj)?,
            apply_linear_f32_keep(hidden_states, &weights.up_proj)?,
        )
    } else {
        (
            apply_linear(hidden_states, &weights.gate_proj)?,
            apply_linear(hidden_states, &weights.up_proj)?,
        )
    };

    let activated = apply_activation(&gate, &cfg.hidden_act)
        .with_context(|| format!("unsupported activation {}", cfg.hidden_act))?;
    let fused = activated.broadcast_mul(&up)?;

    let down = if use_f32 {
        let out_dtype = hidden_states.dtype();
        apply_linear_f32_then_cast(&fused, &weights.down_proj, out_dtype)?
    } else {
        apply_linear(&fused, &weights.down_proj)?
    };
    Ok(MlpForwardOutput {
        hidden_states: down,
        aux_loss: None,
    })
}

fn run_dense_mlp(
    _layer_idx: usize,
    hidden_states: &Tensor,
    weights: &DenseMlpWeights,
    cfg: &DeepseekV2Config,
) -> Result<MlpForwardOutput> {
    run_dense_mlp_with_policy(hidden_states, weights, cfg, is_low_precision(hidden_states))
}

fn apply_packed_linear(
    input: &Tensor,
    packed_weights: &Tensor,
    expert_idx: usize,
    out_dim: usize,
    in_dim: usize,
) -> Result<Tensor> {
    let dims = input.shape().dims();
    if dims.len() < 2 {
        bail!("linear expects rank >= 2, received {:?}", dims);
    }
    let last_dim = *dims.last().expect("at least one dim");
    if in_dim != last_dim {
        bail!(
            "packed linear weight expects input dim {}, got {}",
            in_dim,
            last_dim
        );
    }

    let leading = dims[..dims.len() - 1].iter().product::<usize>();
    let input2d = input.reshape((leading, in_dim))?.contiguous()?;
    let mut weight = select_packed_expert_weight(packed_weights, expert_idx, out_dim, in_dim)?;
    if weight.dtype() != input2d.dtype() {
        weight = weight.to_dtype(input2d.dtype())?;
    }
    let proj = input2d.matmul(&transpose(&weight, 0, 1)?)?;
    proj.reshape(
        dims[..dims.len() - 1]
            .iter()
            .copied()
            .chain(std::iter::once(out_dim))
            .collect::<Vec<_>>(),
    )
    .context("failed to reshape packed linear output")
}

fn apply_packed_linear_f32_then_cast(
    input: &Tensor,
    packed_weights: &Tensor,
    expert_idx: usize,
    out_dim: usize,
    in_dim: usize,
    out_dtype: DType,
) -> Result<Tensor> {
    let dims = input.shape().dims();
    if dims.len() < 2 {
        bail!("linear expects rank >= 2, received {:?}", dims);
    }
    let last_dim = *dims.last().expect("at least one dim");
    if in_dim != last_dim {
        bail!(
            "packed linear weight expects input dim {}, got {}",
            in_dim,
            last_dim
        );
    }

    let leading = dims[..dims.len() - 1].iter().product::<usize>();
    let x2d = input
        .to_dtype(DType::F32)?
        .reshape((leading, in_dim))?
        .contiguous()?;
    let weight = select_packed_expert_weight(packed_weights, expert_idx, out_dim, in_dim)?
        .to_dtype(DType::F32)?
        .contiguous()?;
    let proj = x2d.matmul(&transpose(&weight, 0, 1)?)?;
    Ok(proj
        .reshape(
            dims[..dims.len() - 1]
                .iter()
                .copied()
                .chain(std::iter::once(out_dim))
                .collect::<Vec<_>>(),
        )?
        .to_dtype(out_dtype)?)
}

fn apply_packed_linear_f32_keep(
    input: &Tensor,
    packed_weights: &Tensor,
    expert_idx: usize,
    out_dim: usize,
    in_dim: usize,
) -> Result<Tensor> {
    let dims = input.shape().dims();
    if dims.len() < 2 {
        bail!("linear expects rank >= 2, received {:?}", dims);
    }
    let last_dim = *dims.last().expect("at least one dim");
    if in_dim != last_dim {
        bail!(
            "packed linear weight expects input dim {}, got {}",
            in_dim,
            last_dim
        );
    }

    let leading = dims[..dims.len() - 1].iter().product::<usize>();
    let x2d = input
        .to_dtype(DType::F32)?
        .reshape((leading, in_dim))?
        .contiguous()?;
    let weight = select_packed_expert_weight(packed_weights, expert_idx, out_dim, in_dim)?
        .to_dtype(DType::F32)?
        .contiguous()?;
    let proj = x2d.matmul(&transpose(&weight, 0, 1)?)?;
    proj.reshape(
        dims[..dims.len() - 1]
            .iter()
            .copied()
            .chain(std::iter::once(out_dim))
            .collect::<Vec<_>>(),
    )
    .context("failed to reshape packed linear output")
}

fn run_packed_dense_mlp_with_policy(
    hidden_states: &Tensor,
    weights: &MoeMetalFastExpertPack,
    expert_idx: usize,
    cfg: &DeepseekV2Config,
    use_f32: bool,
) -> Result<MlpForwardOutput> {
    let (gate, up) = if use_f32 {
        (
            apply_packed_linear_f32_keep(
                hidden_states,
                &weights.gate_proj,
                expert_idx,
                weights.intermediate_size,
                weights.hidden_size,
            )?,
            apply_packed_linear_f32_keep(
                hidden_states,
                &weights.up_proj,
                expert_idx,
                weights.intermediate_size,
                weights.hidden_size,
            )?,
        )
    } else {
        (
            apply_packed_linear(
                hidden_states,
                &weights.gate_proj,
                expert_idx,
                weights.intermediate_size,
                weights.hidden_size,
            )?,
            apply_packed_linear(
                hidden_states,
                &weights.up_proj,
                expert_idx,
                weights.intermediate_size,
                weights.hidden_size,
            )?,
        )
    };

    let activated = apply_activation(&gate, &cfg.hidden_act)
        .with_context(|| format!("unsupported activation {}", cfg.hidden_act))?;
    let fused = activated.broadcast_mul(&up)?;

    let down = if use_f32 {
        apply_packed_linear_f32_then_cast(
            &fused,
            &weights.down_proj,
            expert_idx,
            weights.hidden_size,
            weights.intermediate_size,
            hidden_states.dtype(),
        )?
    } else {
        apply_packed_linear(
            &fused,
            &weights.down_proj,
            expert_idx,
            weights.hidden_size,
            weights.intermediate_size,
        )?
    };
    Ok(MlpForwardOutput {
        hidden_states: down,
        aux_loss: None,
    })
}

fn run_packed_mlp_by_ids(
    input: &Tensor,
    weights: &MoeMetalFastExpertPack,
    expert_ids: &Tensor,
    cfg: &DeepseekV2Config,
    use_f32: bool,
) -> Result<Tensor> {
    let (token_count, hidden_size) = input
        .shape()
        .dims2()
        .context("packed token-major mlp expects rank 2 input")?;
    ensure!(
        hidden_size == weights.hidden_size,
        "packed token-major mlp expects hidden size {}, got {}",
        weights.hidden_size,
        hidden_size
    );
    let expert_ids = expert_ids
        .to_dtype(DType::U32)?
        .contiguous()?
        .to_vec1::<u32>()?;
    ensure!(
        expert_ids.len() == token_count,
        "packed token-major mlp expert id count {} does not match token count {}",
        expert_ids.len(),
        token_count
    );

    let mut tokens_per_expert = vec![Vec::new(); weights.expert_count];
    for (token_idx, expert_id) in expert_ids.into_iter().enumerate() {
        let expert_idx = expert_id as usize;
        ensure!(
            expert_idx < weights.expert_count,
            "packed token-major mlp expert id {} out of range 0..{}",
            expert_idx,
            weights.expert_count
        );
        tokens_per_expert[expert_idx].push(token_idx as u32);
    }

    let output = Tensor::zeros(
        (token_count, weights.hidden_size),
        input.dtype(),
        input.device(),
    )?;
    for (expert_idx, token_positions) in tokens_per_expert.iter().enumerate() {
        if token_positions.is_empty() {
            continue;
        }
        let positions = Tensor::from_vec(
            token_positions.clone(),
            (token_positions.len(),),
            input.device(),
        )?
        .contiguous()?;
        let expert_input = input.index_select(&positions, 0)?.contiguous()?;
        let expert_output =
            run_packed_dense_mlp_with_policy(&expert_input, weights, expert_idx, cfg, use_f32)?
                .hidden_states;
        let idx_matrix = positions
            .reshape((token_positions.len(), 1))?
            .expand((token_positions.len(), weights.hidden_size))?
            .contiguous()?;
        output.scatter_set(&idx_matrix, &expert_output, 0)?;
    }
    Ok(output)
}

struct MoeRoutingWork {
    batch: usize,
    seq_len: usize,
    hidden: usize,
    token_count: usize,
    topk: usize,
    device: Device,
    hidden_dtype: DType,
    combine_work_dtype: DType,
    use_f32_expert_compute: bool,
    use_f32_shared_compute: bool,
    tokens: Tensor,
    topk_weights: Tensor,
    topk_indices: Tensor,
    slow_staging: Option<MoeSlowRoutingStaging>,
}

struct MoeSlowRoutingStaging {
    assignment_count: usize,
    idxs: Tensor,
    sorted_tokens: Tensor,
    tokens_per_expert: Vec<usize>,
}

fn build_moe_routing_work(
    layer_idx: usize,
    hidden_states: &Tensor,
    gate_weight: &Tensor,
    aux_bias: Option<&Tensor>,
    cfg: &DeepseekV2Config,
    backend: MoeExecutionBackend,
    build_slow_staging: bool,
    n_routed: usize,
    moe_started: &Instant,
) -> Result<MoeRoutingWork> {
    let num_experts_per_tok = cfg
        .num_experts_per_tok
        .with_context(|| "MoE config missing num_experts_per_tok")?;
    ensure!(
        num_experts_per_tok > 0 && num_experts_per_tok <= n_routed,
        "num_experts_per_tok ({num_experts_per_tok}) must be within 1..=n_routed_experts ({n_routed})"
    );
    let topk_method = cfg.topk_method.as_deref().unwrap_or("greedy");
    ensure!(
        topk_method == "greedy",
        "MoE topk_method `{topk_method}` not yet supported (greedy only)"
    );
    let scoring = cfg.scoring_func.as_deref().unwrap_or("softmax");
    ensure!(
        scoring == "softmax" || scoring == "sigmoid",
        "MoE scoring `{scoring}` not yet supported"
    );
    ensure!(
        cfg.ep_size <= 1,
        "MoE ep_size > 1 not supported in Candle port (got {})",
        cfg.ep_size
    );

    let (batch, seq_len, hidden) = hidden_states.shape().dims3()?;
    let token_count = batch * seq_len;
    let topk = num_experts_per_tok;
    let assignment_count = token_count * topk;
    let use_f32_expert_compute = moe_expert_f32_compute_enabled();
    let use_f32_shared_compute = shared_expert_f32_compute_enabled(use_f32_expert_compute);
    let combine_work_dtype = if moe_combine_f32_enabled() {
        DType::F32
    } else {
        hidden_states.dtype()
    };
    let gate_input_dtype = if moe_gate_input_f32_enabled() {
        DType::F32
    } else {
        hidden_states.dtype()
    };
    emit_stage_trace(
        "block.forward.moe.started",
        &[
            ("elapsed_ms", moe_started.elapsed().as_millis().to_string()),
            ("layer_idx", layer_idx.to_string()),
            ("token_count", token_count.to_string()),
            ("topk", topk.to_string()),
            ("n_routed", n_routed.to_string()),
            (
                "expert_compute",
                if use_f32_expert_compute {
                    "f32".to_string()
                } else {
                    "native".to_string()
                },
            ),
            (
                "shared_compute",
                if use_f32_shared_compute {
                    "f32".to_string()
                } else {
                    "native".to_string()
                },
            ),
            ("combine_dtype", format!("{combine_work_dtype:?}")),
            ("gate_input_dtype", format!("{gate_input_dtype:?}")),
            ("backend", backend.label().to_string()),
        ],
    );

    let device = hidden_states.device().clone();
    let tokens = hidden_states.reshape((token_count, hidden))?.contiguous()?;

    let gate_tokens = if tokens.dtype() == gate_input_dtype {
        tokens.clone()
    } else {
        tokens.to_dtype(gate_input_dtype)?
    };
    let gate_weight = if gate_weight.dtype() == gate_input_dtype {
        gate_weight.clone()
    } else {
        gate_weight.to_dtype(gate_input_dtype)?
    }
    .contiguous()?;
    let mut logits = gate_tokens.matmul(&transpose(&gate_weight, 0, 1)?)?;
    if logits.dtype() != DType::F32 {
        logits = logits.to_dtype(DType::F32)?;
    }
    if let Some(bias) = aux_bias {
        let bias = bias.to_dtype(DType::F32)?.reshape((1, n_routed))?;
        logits = logits.broadcast_add(&bias)?;
    }
    let scores = match scoring {
        "softmax" => softmax(&logits, D::Minus1)?,
        "sigmoid" => sigmoid(&logits)?,
        _ => unreachable!("validated scoring method earlier"),
    };
    let scores = scores.to_dtype(DType::F32)?;
    let scores = scores.contiguous()?;
    let (sorted_scores, sorted_indices) = scores.sort_last_dim(false)?;
    let sorted_scores = sorted_scores.contiguous()?;
    let sorted_indices = sorted_indices.contiguous()?;
    let mut topk_weights = sorted_scores.narrow(D::Minus1, 0, topk)?;
    let topk_indices = sorted_indices
        .narrow(D::Minus1, 0, topk)?
        .to_dtype(DType::I64)?
        .contiguous()?;

    if topk > 1 && cfg.norm_topk_prob {
        let denom = topk_weights.sum_keepdim(D::Minus1)?;
        let eps = Tensor::full(1e-20f32, denom.shape(), denom.device())?;
        topk_weights = topk_weights.broadcast_div(&denom.add(&eps)?)?;
    }
    if cfg.routed_scaling_factor != 1.0 {
        let scale = Tensor::full(
            cfg.routed_scaling_factor,
            topk_weights.shape(),
            topk_weights.device(),
        )?;
        topk_weights = topk_weights.mul(&scale)?;
    }
    let topk_weights = topk_weights.contiguous()?;
    let topk_indices = topk_indices.contiguous()?;
    let slow_staging = if build_slow_staging {
        let flat_topk_ids = topk_indices.reshape((assignment_count,))?.contiguous()?;
        let flat_ids = flat_topk_ids.to_vec1::<i64>()?;
        let mut idxs_vec: Vec<u32> = (0..assignment_count as u32).collect();
        idxs_vec.sort_by_key(|&pos| flat_ids[pos as usize]);

        let idxs =
            Tensor::from_vec(idxs_vec.clone(), (assignment_count,), &device)?.contiguous()?;
        let mut token_pos_vec: Vec<u32> = Vec::with_capacity(assignment_count);
        for &pos in &idxs_vec {
            token_pos_vec.push((pos as usize / topk) as u32);
        }
        let token_pos =
            Tensor::from_vec(token_pos_vec, (assignment_count,), &device)?.contiguous()?;
        let sorted_tokens = tokens.index_select(&token_pos, 0)?.contiguous()?;

        let mut tokens_per_expert = vec![0usize; n_routed];
        for &expert_id in &flat_ids {
            let expert_id = expert_id as usize;
            ensure!(
                expert_id < n_routed,
                "expert id {expert_id} out of range 0..{n_routed}"
            );
            tokens_per_expert[expert_id] += 1;
        }
        let nonzero_experts = tokens_per_expert
            .iter()
            .filter(|&&count| count != 0)
            .count();
        let max_tokens_per_expert = tokens_per_expert.iter().copied().max().unwrap_or(0);
        emit_stage_trace(
            "block.forward.moe.routing.completed",
            &[
                ("elapsed_ms", moe_started.elapsed().as_millis().to_string()),
                ("layer_idx", layer_idx.to_string()),
                ("assignment_count", assignment_count.to_string()),
                ("nonzero_experts", nonzero_experts.to_string()),
                ("max_tokens_per_expert", max_tokens_per_expert.to_string()),
                ("slow_staging", true.to_string()),
            ],
        );
        Some(MoeSlowRoutingStaging {
            assignment_count,
            idxs,
            sorted_tokens,
            tokens_per_expert,
        })
    } else {
        emit_stage_trace(
            "block.forward.moe.routing.completed",
            &[
                ("elapsed_ms", moe_started.elapsed().as_millis().to_string()),
                ("layer_idx", layer_idx.to_string()),
                ("assignment_count", assignment_count.to_string()),
                ("slow_staging", false.to_string()),
            ],
        );
        None
    };

    Ok(MoeRoutingWork {
        batch,
        seq_len,
        hidden,
        token_count,
        topk,
        device,
        hidden_dtype: hidden_states.dtype(),
        combine_work_dtype,
        use_f32_expert_compute,
        use_f32_shared_compute,
        tokens,
        topk_weights,
        topk_indices,
        slow_staging,
    })
}

fn collect_moe_routed_outputs<F>(
    layer_idx: usize,
    work: &MoeRoutingWork,
    moe_started: &Instant,
    linear_path_label: &str,
    mut expert_forward: F,
) -> Result<Vec<Tensor>>
where
    F: FnMut(usize, &Tensor, bool) -> Result<Tensor>,
{
    let slow = work
        .slow_staging
        .as_ref()
        .context("slow-path routing staging missing for routed expert collection")?;
    let mut outputs = Vec::new();
    let mut start_idx = 0usize;
    for (expert_idx, &num_tokens) in slow.tokens_per_expert.iter().enumerate() {
        let end_idx = start_idx + num_tokens;
        ensure!(
            end_idx <= slow.assignment_count,
            "moe routing slice overflow"
        );
        if num_tokens != 0 {
            emit_stage_trace(
                "block.forward.moe.expert.started",
                &[
                    ("elapsed_ms", moe_started.elapsed().as_millis().to_string()),
                    ("layer_idx", layer_idx.to_string()),
                    ("expert_idx", expert_idx.to_string()),
                    ("num_tokens", num_tokens.to_string()),
                    ("hidden_dtype", format!("{:?}", work.hidden_dtype)),
                    ("linear_path", linear_path_label.to_string()),
                ],
            );
            let tokens_for_expert = slow.sorted_tokens.narrow(0, start_idx, num_tokens)?;
            let expert_out =
                expert_forward(expert_idx, &tokens_for_expert, work.use_f32_expert_compute)?;
            outputs.push(expert_out.to_dtype(work.combine_work_dtype)?);
            emit_stage_trace(
                "block.forward.moe.expert.completed",
                &[
                    ("elapsed_ms", moe_started.elapsed().as_millis().to_string()),
                    ("layer_idx", layer_idx.to_string()),
                    ("expert_idx", expert_idx.to_string()),
                    ("num_tokens", num_tokens.to_string()),
                ],
            );
        }
        start_idx = end_idx;
    }
    ensure!(
        start_idx == slow.assignment_count,
        "moe routing consumed {start_idx} assignments but expected {}",
        slow.assignment_count
    );
    Ok(outputs)
}

fn combine_moe_routed_outputs(
    layer_idx: usize,
    work: &MoeRoutingWork,
    moe_started: &Instant,
    outputs: Vec<Tensor>,
) -> Result<Tensor> {
    let slow = work
        .slow_staging
        .as_ref()
        .context("slow-path routing staging missing for routed expert combine")?;
    let outs = if outputs.is_empty() {
        Tensor::zeros(
            (slow.assignment_count, work.hidden),
            work.combine_work_dtype,
            &work.device,
        )?
    } else {
        Tensor::cat(&outputs, 0)?
    }
    .contiguous()?;
    emit_stage_trace(
        "block.forward.moe.experts.completed",
        &[
            ("elapsed_ms", moe_started.elapsed().as_millis().to_string()),
            ("layer_idx", layer_idx.to_string()),
            ("output_rows", slow.assignment_count.to_string()),
        ],
    );

    let new_x = {
        let new_x = Tensor::zeros(
            (slow.assignment_count, work.hidden),
            work.combine_work_dtype,
            &work.device,
        )?;
        let idx_matrix = slow
            .idxs
            .reshape((slow.assignment_count, 1))?
            .expand((slow.assignment_count, work.hidden))?
            .contiguous()?;
        new_x.scatter_set(&idx_matrix, &outs, 0)?;
        new_x
    };
    emit_stage_trace(
        "block.forward.moe.scatter.completed",
        &[
            ("elapsed_ms", moe_started.elapsed().as_millis().to_string()),
            ("layer_idx", layer_idx.to_string()),
        ],
    );

    let combined = {
        let x = new_x.reshape((work.token_count, work.topk, work.hidden))?;
        let w = work.topk_weights.to_dtype(work.combine_work_dtype)?;
        x.broadcast_mul(&w.unsqueeze(D::Minus1)?)?
            .sum(1)?
            .to_dtype(work.hidden_dtype)?
            .reshape((work.batch, work.seq_len, work.hidden))?
    };
    emit_stage_trace(
        "block.forward.moe.combine.completed",
        &[
            ("elapsed_ms", moe_started.elapsed().as_millis().to_string()),
            ("layer_idx", layer_idx.to_string()),
        ],
    );
    Ok(combined)
}

fn maybe_add_shared_moe_output(
    layer_idx: usize,
    hidden_states: &Tensor,
    combined: Tensor,
    shared: Option<&DenseMlpWeights>,
    use_f32_shared_compute: bool,
    cfg: &DeepseekV2Config,
    moe_started: &Instant,
) -> Result<Tensor> {
    let Some(shared) = shared else {
        emit_stage_trace(
            "block.forward.moe.completed",
            &[
                ("elapsed_ms", moe_started.elapsed().as_millis().to_string()),
                ("layer_idx", layer_idx.to_string()),
            ],
        );
        return Ok(combined);
    };

    if skip_shared_experts_enabled() {
        emit_stage_trace(
            "block.forward.moe.shared.skipped",
            &[
                ("elapsed_ms", moe_started.elapsed().as_millis().to_string()),
                ("layer_idx", layer_idx.to_string()),
                ("reason", "env_override".to_string()),
            ],
        );
        emit_stage_trace(
            "block.forward.moe.completed",
            &[
                ("elapsed_ms", moe_started.elapsed().as_millis().to_string()),
                ("layer_idx", layer_idx.to_string()),
            ],
        );
        return Ok(combined);
    }

    emit_stage_trace(
        "block.forward.moe.shared.started",
        &[
            ("elapsed_ms", moe_started.elapsed().as_millis().to_string()),
            ("layer_idx", layer_idx.to_string()),
        ],
    );
    let shared_input = prepare_dense_mlp_input(hidden_states, &shared, use_f32_shared_compute)?;
    let shared_out = if use_f32_shared_compute {
        run_dense_mlp_f32_keep(&shared_input, &shared, cfg)?
    } else {
        run_dense_mlp_with_policy(&shared_input, &shared, cfg, false)?.hidden_states
    }
    .to_dtype(hidden_states.dtype())?;
    let shared_out = shared_out.to_device(hidden_states.device())?;
    let combined = add_stable(&combined, &shared_out)?;
    emit_stage_trace(
        "block.forward.moe.shared.completed",
        &[
            ("elapsed_ms", moe_started.elapsed().as_millis().to_string()),
            ("layer_idx", layer_idx.to_string()),
        ],
    );
    emit_stage_trace(
        "block.forward.moe.completed",
        &[
            ("elapsed_ms", moe_started.elapsed().as_millis().to_string()),
            ("layer_idx", layer_idx.to_string()),
        ],
    );
    Ok(combined)
}

fn run_moe(
    layer_idx: usize,
    hidden_states: &Tensor,
    weights: &MoeWeights,
    cfg: &DeepseekV2Config,
) -> Result<MlpForwardOutput> {
    let n_routed = cfg
        .n_routed_experts
        .with_context(|| "MoE config missing n_routed_experts")?;
    ensure!(n_routed > 0, "n_routed_experts must be > 0 for MoE");
    ensure!(
        weights.expert_count() == n_routed,
        "MoE expert count {} does not match config n_routed_experts {}",
        weights.expert_count(),
        n_routed
    );

    match &weights.backend {
        MoeBackendWeights::Slow(layout) => run_moe_slow_path(
            layer_idx,
            hidden_states,
            &weights.gate_weight,
            layout,
            weights.aux_bias.as_ref(),
            cfg,
            MoeExecutionBackend::Slow,
        ),
        MoeBackendWeights::MetalFast(layout) => run_moe_metal_fast_skeleton(
            layer_idx,
            hidden_states,
            &weights.gate_weight,
            layout,
            weights.aux_bias.as_ref(),
            cfg,
        ),
    }
}

fn run_moe_metal_fast_skeleton(
    layer_idx: usize,
    hidden_states: &Tensor,
    gate_weight: &Tensor,
    weights: &MoeMetalFastWeights,
    aux_bias: Option<&Tensor>,
    cfg: &DeepseekV2Config,
) -> Result<MlpForwardOutput> {
    let fallback = weights.fallback_layout();
    let packed_impl = if weights.packed_experts.is_some() {
        "packed_token_major_routed_experts"
    } else {
        "skeleton_fallback"
    };
    emit_stage_trace(
        "block.forward.moe.backend.selected",
        &[
            ("layer_idx", layer_idx.to_string()),
            (
                "backend",
                MoeExecutionBackend::MetalFast.label().to_string(),
            ),
            (
                "packed_experts",
                weights.packed_experts.is_some().to_string(),
            ),
            ("implementation", packed_impl.to_string()),
        ],
    );
    if let Some(packed) = weights.packed_experts.as_ref() {
        let moe_started = Instant::now();
        let work = build_moe_routing_work(
            layer_idx,
            hidden_states,
            gate_weight,
            aux_bias,
            cfg,
            MoeExecutionBackend::MetalFast,
            false,
            weights.expert_count,
            &moe_started,
        )?;
        let packed_input =
            prepare_packed_mlp_input(&work.tokens, packed, work.use_f32_expert_compute)?;
        let linear_path = if work.use_f32_expert_compute {
            "run_dense_mlp_packed_token_major_f32_keep"
        } else {
            "run_dense_mlp_packed_token_major_native"
        };
        let mut combined = Tensor::zeros(
            (work.token_count, work.hidden),
            work.combine_work_dtype,
            &work.device,
        )?;
        for slot_idx in 0..work.topk {
            emit_stage_trace(
                "block.forward.moe.fast.slot.started",
                &[
                    ("elapsed_ms", moe_started.elapsed().as_millis().to_string()),
                    ("layer_idx", layer_idx.to_string()),
                    ("slot_idx", slot_idx.to_string()),
                    ("linear_path", linear_path.to_string()),
                ],
            );
            let expert_ids = work
                .topk_indices
                .narrow(D::Minus1, slot_idx, 1)?
                .reshape((work.token_count,))?
                .to_dtype(DType::U32)?
                .contiguous()?;
            let slot_out = run_packed_mlp_by_ids(
                &packed_input,
                packed,
                &expert_ids,
                cfg,
                work.use_f32_expert_compute,
            )?
            .to_dtype(work.combine_work_dtype)?;
            let slot_weight = work
                .topk_weights
                .narrow(D::Minus1, slot_idx, 1)?
                .to_dtype(work.combine_work_dtype)?
                .reshape((work.token_count, 1))?;
            let weighted = slot_out.broadcast_mul(&slot_weight)?;
            combined = combined.add(&weighted)?;
            emit_stage_trace(
                "block.forward.moe.fast.slot.completed",
                &[
                    ("elapsed_ms", moe_started.elapsed().as_millis().to_string()),
                    ("layer_idx", layer_idx.to_string()),
                    ("slot_idx", slot_idx.to_string()),
                ],
            );
        }
        emit_stage_trace(
            "block.forward.moe.experts.completed",
            &[
                ("elapsed_ms", moe_started.elapsed().as_millis().to_string()),
                ("layer_idx", layer_idx.to_string()),
                ("output_rows", work.token_count.to_string()),
            ],
        );
        emit_stage_trace(
            "block.forward.moe.scatter.skipped",
            &[
                ("elapsed_ms", moe_started.elapsed().as_millis().to_string()),
                ("layer_idx", layer_idx.to_string()),
                ("reason", "token_major_packed_path".to_string()),
            ],
        );
        let combined = combined.to_dtype(work.hidden_dtype)?.reshape((
            work.batch,
            work.seq_len,
            work.hidden,
        ))?;
        emit_stage_trace(
            "block.forward.moe.combine.completed",
            &[
                ("elapsed_ms", moe_started.elapsed().as_millis().to_string()),
                ("layer_idx", layer_idx.to_string()),
            ],
        );
        let combined = maybe_add_shared_moe_output(
            layer_idx,
            hidden_states,
            combined,
            weights.shared_experts.as_ref(),
            work.use_f32_shared_compute,
            cfg,
            &moe_started,
        )?;
        return Ok(MlpForwardOutput {
            hidden_states: combined,
            aux_loss: None,
        });
    }
    let fallback = fallback.context(
        "metal_fast backend missing fallback experts when packed experts are unavailable",
    )?;
    run_moe_slow_path(
        layer_idx,
        hidden_states,
        gate_weight,
        &fallback,
        aux_bias,
        cfg,
        MoeExecutionBackend::MetalFast,
    )
}

fn run_moe_slow_path(
    layer_idx: usize,
    hidden_states: &Tensor,
    gate_weight: &Tensor,
    weights: &MoeSlowWeights,
    aux_bias: Option<&Tensor>,
    cfg: &DeepseekV2Config,
    backend: MoeExecutionBackend,
) -> Result<MlpForwardOutput> {
    let moe_started = Instant::now();
    let work = build_moe_routing_work(
        layer_idx,
        hidden_states,
        gate_weight,
        aux_bias,
        cfg,
        backend,
        true,
        weights.experts.len(),
        &moe_started,
    )?;
    let linear_path = if work.use_f32_expert_compute {
        "run_dense_mlp_f32_keep"
    } else {
        "run_dense_mlp_native"
    };
    let outputs = collect_moe_routed_outputs(
        layer_idx,
        &work,
        &moe_started,
        linear_path,
        |expert_idx, tokens_for_expert, use_f32_compute| {
            let expert_weights = weights.experts[expert_idx].resolve()?;
            let expert_input =
                prepare_dense_mlp_input(tokens_for_expert, &expert_weights, use_f32_compute)?;
            if use_f32_compute {
                run_dense_mlp_f32_keep(&expert_input, &expert_weights, cfg)
            } else {
                run_dense_mlp_with_policy(&expert_input, &expert_weights, cfg, false)
                    .map(|output| output.hidden_states)
            }
        },
    )?;
    let combined = combine_moe_routed_outputs(layer_idx, &work, &moe_started, outputs)?;
    let combined = maybe_add_shared_moe_output(
        layer_idx,
        hidden_states,
        combined,
        weights.shared_experts.as_ref(),
        work.use_f32_shared_compute,
        cfg,
        &moe_started,
    )?;

    Ok(MlpForwardOutput {
        hidden_states: combined,
        aux_loss: None,
    })
}

fn transpose(t: &Tensor, dim0: usize, dim1: usize) -> Result<Tensor> {
    let mut dims: Vec<usize> = (0..t.rank()).collect();
    dims.swap(dim0, dim1);
    Ok(t.permute(dims)?)
}

fn apply_rope(x: &Tensor, cos: &Tensor, sin: &Tensor, reorder: bool) -> Result<Tensor> {
    // DeepSeek MLA path uses an extra even/odd regrouping before rotate_half.
    let x = if reorder {
        let last = x.dim(D::Minus1)?;
        if last == 0 {
            x.clone()
        } else {
            ensure!(
                last % 2 == 0,
                "apply_rope expects an even rope dimension, got {last}"
            );
            let (b, h, s, d) = x
                .shape()
                .dims4()
                .context("apply_rope expects x with shape [batch, heads, seq, rope_dim]")?;
            ensure!(d == last, "internal rope dim mismatch (d={d}, last={last})");
            x.reshape((b, h, s, d / 2, 2))?
                .transpose(3, 4)?
                .contiguous()?
                .reshape((b, h, s, d))?
        }
    } else {
        x.clone()
    };

    let out_dtype = x.dtype();
    let low_precision = is_low_precision(&x);
    let (x, cos, sin) = if low_precision {
        (
            x.to_dtype(DType::F32)?,
            cos.to_dtype(DType::F32)?,
            sin.to_dtype(DType::F32)?,
        )
    } else {
        let cos = if cos.dtype() == out_dtype {
            cos.clone()
        } else {
            cos.to_dtype(out_dtype)?
        };
        let sin = if sin.dtype() == out_dtype {
            sin.clone()
        } else {
            sin.to_dtype(out_dtype)?
        };
        (x, cos, sin)
    };

    let rotated = rotate_half(&x)?;
    let x_cos = x.broadcast_mul(&cos)?;
    let rot_sin = rotated.broadcast_mul(&sin)?;
    let out = x_cos.add(&rot_sin)?;
    if low_precision {
        Ok(out.to_dtype(out_dtype)?)
    } else {
        Ok(out)
    }
}

fn rotate_half(x: &Tensor) -> Result<Tensor> {
    let last = x.dim(D::Minus1)?;
    ensure!(
        last % 2 == 0,
        "rotate_half expects even dimension, got {last}"
    );
    let left = x.narrow(D::Minus1, 0, last / 2)?;
    let right = x.narrow(D::Minus1, last / 2, last / 2)?;
    let neg_right = right.neg()?;
    Ok(Tensor::cat(&[neg_right, left], D::Minus1)?)
}

/// Construct a padding mask from per-batch sequence lengths.
///
/// Returns a tensor of shape `(batch, seq_len)` with `1.0` for real tokens and `0.0` for padding.
pub fn lengths_to_padding_mask(
    lengths: &[usize],
    seq_len: usize,
    device: &Device,
) -> Result<Tensor> {
    let batch = lengths.len();
    let mut data = vec![0f32; batch * seq_len];
    for (batch_idx, &len) in lengths.iter().enumerate() {
        ensure!(
            len <= seq_len,
            "length {} exceeds sequence dimension {}",
            len,
            seq_len
        );
        for pos in 0..len {
            data[batch_idx * seq_len + pos] = 1.0;
        }
    }
    Ok(Tensor::from_vec(data, (batch, seq_len), device)?)
}

fn mask_fill_value(dtype: DType) -> f32 {
    match dtype {
        DType::F16 | DType::BF16 => -1e4f32,
        _ => -1e9f32,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_moe_routing_work, is_expert_linear_label, run_dense_mlp_with_policy,
        run_packed_dense_mlp_with_policy, run_packed_mlp_by_ids, shared_expert_f32_compute_enabled,
        skip_shared_experts_enabled,
    };
    use crate::{
        config::DeepseekV2Config,
        transformer::weights::{
            DenseMlpWeights, LinearWeights, MoeExecutionBackend, MoeMetalFastExpertPack,
        },
    };
    use candle_core::{Device, Tensor};
    use std::{
        sync::{Mutex, OnceLock},
        time::Instant,
    };

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn test_config() -> DeepseekV2Config {
        serde_json::from_value(serde_json::json!({
            "vocab_size": 32,
            "hidden_size": 2,
            "intermediate_size": 3,
            "num_hidden_layers": 1,
            "num_attention_heads": 1,
            "max_position_embeddings": 32,
            "hidden_act": "relu"
        }))
        .expect("minimal config should parse")
    }

    fn test_moe_config() -> DeepseekV2Config {
        serde_json::from_value(serde_json::json!({
            "vocab_size": 32,
            "hidden_size": 2,
            "intermediate_size": 3,
            "num_hidden_layers": 1,
            "num_attention_heads": 1,
            "max_position_embeddings": 32,
            "hidden_act": "relu",
            "n_routed_experts": 2,
            "num_experts_per_tok": 1
        }))
        .expect("minimal moe config should parse")
    }

    fn test_linear(label: &str, out_dim: usize, in_dim: usize, data: &[f32]) -> LinearWeights {
        LinearWeights {
            weight: Some(
                Tensor::from_vec(data.to_vec(), (out_dim, in_dim), &Device::Cpu)
                    .expect("test tensor"),
            ),
            weight_f32: None,
            bias: None,
            qmatmul: None,
            out_dim,
            in_dim,
            label: label.to_string(),
        }
    }

    fn test_dense_weights() -> DenseMlpWeights {
        DenseMlpWeights {
            gate_proj: test_linear("gate", 3, 2, &[1.0, 0.5, 0.0, 1.0, 0.5, -0.5]),
            up_proj: test_linear("up", 3, 2, &[0.25, 1.0, 1.0, -0.5, 0.5, 0.5]),
            down_proj: test_linear("down", 2, 3, &[1.0, 0.0, 0.5, 0.0, 1.0, -0.5]),
        }
    }

    fn test_dense_weights_alt() -> DenseMlpWeights {
        DenseMlpWeights {
            gate_proj: test_linear("gate_alt", 3, 2, &[0.5, -0.5, 1.0, 0.25, 0.75, 0.0]),
            up_proj: test_linear("up_alt", 3, 2, &[1.0, 0.0, 0.5, 0.5, 0.25, 1.0]),
            down_proj: test_linear("down_alt", 2, 3, &[0.5, 1.0, 0.0, -0.25, 0.75, 1.0]),
        }
    }

    fn test_packed_weights() -> MoeMetalFastExpertPack {
        let dense = test_dense_weights();
        MoeMetalFastExpertPack {
            gate_proj: Tensor::stack(&[dense.gate_proj.weight.as_ref().unwrap()], 0)
                .expect("stack gate"),
            up_proj: Tensor::stack(&[dense.up_proj.weight.as_ref().unwrap()], 0).expect("stack up"),
            down_proj: Tensor::stack(&[dense.down_proj.weight.as_ref().unwrap()], 0)
                .expect("stack down"),
            expert_count: 1,
            hidden_size: 2,
            intermediate_size: 3,
        }
    }

    fn test_packed_weights_pair() -> MoeMetalFastExpertPack {
        let dense_a = test_dense_weights();
        let dense_b = test_dense_weights_alt();
        MoeMetalFastExpertPack {
            gate_proj: Tensor::stack(
                &[
                    dense_a.gate_proj.weight.as_ref().unwrap(),
                    dense_b.gate_proj.weight.as_ref().unwrap(),
                ],
                0,
            )
            .expect("stack gate"),
            up_proj: Tensor::stack(
                &[
                    dense_a.up_proj.weight.as_ref().unwrap(),
                    dense_b.up_proj.weight.as_ref().unwrap(),
                ],
                0,
            )
            .expect("stack up"),
            down_proj: Tensor::stack(
                &[
                    dense_a.down_proj.weight.as_ref().unwrap(),
                    dense_b.down_proj.weight.as_ref().unwrap(),
                ],
                0,
            )
            .expect("stack down"),
            expert_count: 2,
            hidden_size: 2,
            intermediate_size: 3,
        }
    }

    #[test]
    fn expert_linear_trace_matches_moe_expert_labels() {
        assert!(is_expert_linear_label(
            "model.layers.1.mlp.experts.2.gate_proj.weight"
        ));
    }

    #[test]
    fn expert_linear_trace_matches_shared_expert_labels() {
        assert!(is_expert_linear_label(
            "model.layers.1.mlp.shared_experts.gate_proj.weight"
        ));
    }

    #[test]
    fn expert_linear_trace_ignores_non_expert_labels() {
        assert!(!is_expert_linear_label(
            "model.layers.1.self_attn.q_proj.weight"
        ));
    }

    #[test]
    fn packed_dense_mlp_matches_dense_mlp_for_single_expert() {
        let cfg = test_config();
        let dense = test_dense_weights();
        let packed = test_packed_weights();
        let input = Tensor::from_vec(vec![1.0f32, 2.0, 3.0, 4.0], (2, 2), &Device::Cpu)
            .expect("input tensor");

        for use_f32 in [false, true] {
            let dense_out = run_dense_mlp_with_policy(&input, &dense, &cfg, use_f32)
                .expect("dense mlp should run")
                .hidden_states
                .to_vec2::<f32>()
                .expect("dense output");
            let packed_out = run_packed_dense_mlp_with_policy(&input, &packed, 0, &cfg, use_f32)
                .expect("packed mlp should run")
                .hidden_states
                .to_vec2::<f32>()
                .expect("packed output");
            assert_eq!(dense_out, packed_out);
        }
    }

    #[test]
    fn packed_token_major_mlp_matches_per_token_expert_execution() {
        let cfg = test_config();
        let packed = test_packed_weights_pair();
        let input = Tensor::from_vec(vec![1.0f32, 2.0, 3.0, 4.0], (2, 2), &Device::Cpu)
            .expect("input tensor");
        let expert_ids =
            Tensor::from_vec(vec![0u32, 1u32], (2,), &Device::Cpu).expect("expert ids");

        for use_f32 in [false, true] {
            let token0 = input.narrow(0, 0, 1).expect("token0");
            let token1 = input.narrow(0, 1, 1).expect("token1");
            let ref0 = run_packed_dense_mlp_with_policy(&token0, &packed, 0, &cfg, use_f32)
                .expect("ref0")
                .hidden_states;
            let ref1 = run_packed_dense_mlp_with_policy(&token1, &packed, 1, &cfg, use_f32)
                .expect("ref1")
                .hidden_states;
            let expected = Tensor::cat(&[&ref0, &ref1], 0)
                .expect("cat refs")
                .to_vec2::<f32>()
                .expect("expected");
            let actual = run_packed_mlp_by_ids(&input, &packed, &expert_ids, &cfg, use_f32)
                .expect("token-major packed output")
                .to_vec2::<f32>()
                .expect("actual");
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn metal_fast_routing_work_skips_slow_staging() {
        let cfg = test_moe_config();
        let hidden_states = Tensor::from_vec(vec![1.0f32, 2.0, 3.0, 4.0], (1, 2, 2), &Device::Cpu)
            .expect("hidden states");
        let gate_weight =
            Tensor::from_vec(vec![1.0f32, 0.0, 0.0, 1.0], (2, 2), &Device::Cpu).expect("gate");
        let started = Instant::now();

        let work = build_moe_routing_work(
            0,
            &hidden_states,
            &gate_weight,
            None,
            &cfg,
            MoeExecutionBackend::MetalFast,
            false,
            2,
            &started,
        )
        .expect("metal fast routing work");

        assert!(work.slow_staging.is_none());
        assert_eq!(work.topk_indices.shape().dims(), &[2, 1]);
    }

    #[test]
    fn slow_routing_work_builds_slow_staging() {
        let cfg = test_moe_config();
        let hidden_states = Tensor::from_vec(vec![1.0f32, 2.0, 3.0, 4.0], (1, 2, 2), &Device::Cpu)
            .expect("hidden states");
        let gate_weight =
            Tensor::from_vec(vec![1.0f32, 0.0, 0.0, 1.0], (2, 2), &Device::Cpu).expect("gate");
        let started = Instant::now();

        let work = build_moe_routing_work(
            0,
            &hidden_states,
            &gate_weight,
            None,
            &cfg,
            MoeExecutionBackend::Slow,
            true,
            2,
            &started,
        )
        .expect("slow routing work");

        let slow = work.slow_staging.as_ref().expect("slow staging");
        assert_eq!(slow.assignment_count, 2);
        assert_eq!(slow.tokens_per_expert.iter().sum::<usize>(), 2);
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
            std::env::set_var("XIUXIAN_VISION_SKIP_SHARED_EXPERTS", "on");
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
    fn shared_expert_compute_flag_defaults_to_routed_policy() {
        let _guard = env_lock().lock().expect("env lock poisoned");
        // SAFETY: tests serialize access to this process-wide env var through env_lock().
        unsafe {
            std::env::remove_var("XIUXIAN_VISION_SHARED_EXPERT_F32_COMPUTE");
        }
        assert!(shared_expert_f32_compute_enabled(true));
        assert!(!shared_expert_f32_compute_enabled(false));
        // SAFETY: tests serialize access to this process-wide env var through env_lock().
        unsafe {
            std::env::set_var("XIUXIAN_VISION_SHARED_EXPERT_F32_COMPUTE", "off");
        }
        assert!(!shared_expert_f32_compute_enabled(true));
        // SAFETY: tests serialize access to this process-wide env var through env_lock().
        unsafe {
            std::env::set_var("XIUXIAN_VISION_SHARED_EXPERT_F32_COMPUTE", "on");
        }
        assert!(shared_expert_f32_compute_enabled(false));
        // SAFETY: tests serialize access to this process-wide env var through env_lock().
        unsafe {
            std::env::remove_var("XIUXIAN_VISION_SHARED_EXPERT_F32_COMPUTE");
        }
    }
}

pub fn build_attention_bias(
    pad_mask: Option<&Tensor>,
    batch: usize,
    q_len: usize,
    k_len: usize,
    past_len: usize,
    dtype: DType,
    device: &Device,
) -> Result<Option<Tensor>> {
    let mut bias: Option<Tensor> = None;

    if past_len == 0 && q_len == k_len && q_len > 1 {
        let rows = Tensor::arange(0i64, q_len as i64, device)?.reshape((q_len, 1))?;
        let cols = Tensor::arange(0i64, k_len as i64, device)?.reshape((1, k_len))?;
        let mask = cols.broadcast_gt(&rows)?;
        let mask = mask.to_dtype(dtype)?;
        let fill =
            Tensor::full(mask_fill_value(dtype), mask.shape().clone(), device)?.to_dtype(dtype)?;
        let causal = mask.mul(&fill)?;
        let causal = causal.reshape((1, 1, q_len, k_len))?;
        let causal = causal.expand((batch, 1, q_len, k_len))?;
        bias = Some(causal);
    }

    if let Some(mask) = pad_mask {
        let (b, s) = mask.shape().dims2()?;
        ensure!(
            b == batch,
            "padding mask batch {} does not match input batch {}",
            b,
            batch
        );
        ensure!(
            s == k_len,
            "padding mask seq {} does not match key length {}",
            s,
            k_len
        );
        let mask = if mask.dtype() == dtype {
            mask.clone()
        } else {
            mask.to_dtype(dtype)?
        };
        let ones = Tensor::full(1f32, (batch, k_len), device)?.to_dtype(dtype)?;
        let inv = ones.sub(&mask)?;
        let inv = inv.reshape((batch, 1, 1, k_len))?;
        let fill =
            Tensor::full(mask_fill_value(dtype), inv.shape().clone(), device)?.to_dtype(dtype)?;
        let pad_bias = inv.mul(&fill)?;
        bias = Some(if let Some(existing) = bias {
            existing.broadcast_add(&pad_bias)?
        } else {
            pad_bias
        });
    }

    Ok(bias)
}
