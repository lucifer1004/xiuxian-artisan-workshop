use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    convert::TryFrom,
};

use anyhow::{Context, Result, ensure};
use candle_core::{DType, Tensor};
use rand::{
    SeedableRng,
    distributions::{Distribution, WeightedIndex},
    rngs::StdRng,
};

/// Parameters shared by decoding backends for token selection.
pub trait TokenSelectionParams {
    fn do_sample(&self) -> bool;
    fn temperature(&self) -> f64;
    fn top_p(&self) -> Option<f64>;
    fn top_k(&self) -> Option<usize>;
    fn repetition_penalty(&self) -> f32;
    fn no_repeat_ngram_size(&self) -> Option<usize>;
    fn prefer_digit_first_token(&self) -> bool {
        false
    }
    fn preferred_first_visible_text(&self) -> Option<&str> {
        None
    }
}

/// Create a deterministic RNG when a seed is provided.
pub fn init_rng(seed: Option<u64>) -> StdRng {
    match seed {
        Some(value) => StdRng::seed_from_u64(value),
        None => StdRng::from_entropy(),
    }
}

/// Select the next token id using the configured sampling strategy.
pub fn select_token_id<P: TokenSelectionParams>(
    logits: &Tensor,
    params: &P,
    context: &[i64],
    rng: &mut StdRng,
) -> Result<i64> {
    let logits = logits
        .to_dtype(DType::F32)?
        .to_vec1::<f32>()
        .context("failed to extract logits for token selection")?;
    select_token_id_from_logits_values(&logits, params, context, rng)
}

/// Select the next token id from an already-materialized logits vector.
pub fn select_token_id_from_logits_values<P: TokenSelectionParams>(
    logits: &[f32],
    params: &P,
    context: &[i64],
    rng: &mut StdRng,
) -> Result<i64> {
    ensure!(!logits.is_empty(), "logits tensor is empty");

    let mut adjusted = logits.to_vec();
    apply_repetition_penalty(&mut adjusted, context, params.repetition_penalty());

    let filtered_storage =
        filter_logits_for_ngram_blocking(&adjusted, context, params.no_repeat_ngram_size());
    let filtered = filtered_storage.as_deref().unwrap_or(&adjusted);

    if params.do_sample() && params.temperature() > 0.0 {
        let mut logits64: Vec<f64> = filtered
            .iter()
            .map(|&v| (v as f64) / params.temperature())
            .collect();
        if let Some(k) = params.top_k()
            && k > 0
            && k < logits64.len()
        {
            apply_top_k(&mut logits64, k);
        }
        if let Some(top_p) = params.top_p()
            && (0.0..1.0).contains(&top_p)
        {
            apply_top_p(&mut logits64, top_p);
        }
        if let Some(sampled) = sample_from_logits(&logits64, rng) {
            return Ok(sampled as i64);
        }
    }

    if let Some(best) = argmax_index(&filtered) {
        return Ok(best as i64);
    }
    if let Some(best) = argmax_index(&adjusted) {
        return Ok(best as i64);
    }
    if let Some(best) = argmax_index(&logits) {
        return Ok(best as i64);
    }
    Ok(0)
}

fn filter_logits_for_ngram_blocking(
    adjusted: &[f32],
    context: &[i64],
    no_repeat_ngram_size: Option<usize>,
) -> Option<Vec<f32>> {
    let Some(ngram) = no_repeat_ngram_size else {
        return None;
    };
    if ngram <= 1 {
        return None;
    }

    let banned = banned_ngram_tokens(context, ngram);
    if banned.is_empty() {
        return None;
    }

    let mut filtered = adjusted.to_vec();
    for token in banned {
        if let Ok(index) = usize::try_from(token)
            && index < filtered.len()
        {
            filtered[index] = f32::NEG_INFINITY;
        }
    }

    has_valid_logits(&filtered).then_some(filtered)
}

fn has_valid_logits(values: &[f32]) -> bool {
    values
        .iter()
        .any(|v| v.is_finite() && *v > f32::NEG_INFINITY)
}

fn argmax_index(values: &[f32]) -> Option<usize> {
    // Match torch.argmax tie-breaking: return the first index for equal maxima.
    let mut best: Option<(usize, f32)> = None;
    for (idx, &value) in values.iter().enumerate() {
        if !value.is_finite() || value <= f32::NEG_INFINITY {
            continue;
        }
        match best {
            None => best = Some((idx, value)),
            Some((_, current)) if value > current => best = Some((idx, value)),
            _ => {}
        }
    }
    best.map(|(idx, _)| idx)
}

fn apply_repetition_penalty(scores: &mut [f32], context: &[i64], penalty: f32) {
    if penalty <= 0.0 || (penalty - 1.0).abs() <= f32::EPSILON {
        return;
    }
    let penalty = penalty.max(f32::MIN_POSITIVE);
    let mut seen = HashSet::new();
    for &token in context {
        if let Ok(index) = usize::try_from(token)
            && index < scores.len()
            && seen.insert(index)
        {
            let entry = &mut scores[index];
            if *entry > 0.0 {
                *entry /= penalty;
            } else {
                *entry *= penalty;
            }
        }
    }
}

fn banned_ngram_tokens(sequence: &[i64], ngram: usize) -> HashSet<i64> {
    let mut banned = HashSet::new();
    if ngram <= 1 || sequence.len() < ngram - 1 {
        return banned;
    }

    let mut history: HashMap<Vec<i64>, HashSet<i64>> = HashMap::new();
    for window in sequence.windows(ngram) {
        let prefix = window[..ngram - 1].to_vec();
        let next = window[ngram - 1];
        history.entry(prefix).or_default().insert(next);
    }
    let prefix = &sequence[sequence.len() - (ngram - 1)..];
    if let Some(tokens) = history.get(prefix) {
        banned.extend(tokens.iter().copied());
    }
    banned
}

fn apply_top_k(logits: &mut [f64], top_k: usize) {
    if top_k == 0 || logits.is_empty() {
        return;
    }
    let mut indices: Vec<usize> = (0..logits.len())
        .filter(|&idx| logits[idx].is_finite())
        .collect();
    if indices.len() <= top_k {
        return;
    }
    indices.sort_by(|&a, &b| logits[b].partial_cmp(&logits[a]).unwrap_or(Ordering::Equal));
    for &idx in indices.iter().skip(top_k) {
        logits[idx] = f64::NEG_INFINITY;
    }
}

fn apply_top_p(logits: &mut [f64], top_p: f64) {
    if !(0.0..1.0).contains(&top_p) || logits.is_empty() {
        return;
    }
    let mut pairs: Vec<(usize, f64)> = logits
        .iter()
        .enumerate()
        .filter_map(|(idx, value)| value.is_finite().then_some((idx, *value)))
        .collect();
    if pairs.is_empty() {
        return;
    }
    pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
    let max_logit = pairs[0].1;
    let mut exp_scores = Vec::with_capacity(pairs.len());
    let mut total = 0.0;
    for (_, logit) in &pairs {
        let weight = (logit - max_logit).exp();
        exp_scores.push(weight);
        total += weight;
    }
    if total <= 0.0 {
        return;
    }
    let mut cumulative = 0.0;
    let mut keep = pairs.len();
    for (idx, weight) in exp_scores.iter().enumerate() {
        cumulative += *weight / total;
        if cumulative > top_p {
            keep = idx + 1;
            break;
        }
    }
    if keep == 0 {
        keep = 1;
    }
    let mut mask = vec![false; logits.len()];
    for (idx, (token_idx, _)) in pairs.iter().enumerate() {
        if idx < keep {
            mask[*token_idx] = true;
        }
    }
    for (idx, keep) in mask.into_iter().enumerate() {
        if !keep {
            logits[idx] = f64::NEG_INFINITY;
        }
    }
}

fn sample_from_logits(logits: &[f64], rng: &mut StdRng) -> Option<usize> {
    let indices: Vec<usize> = (0..logits.len())
        .filter(|&idx| logits[idx].is_finite() && logits[idx] > f64::NEG_INFINITY)
        .collect();
    if indices.is_empty() {
        return None;
    }
    let max_logit = indices
        .iter()
        .map(|&idx| logits[idx])
        .fold(f64::NEG_INFINITY, f64::max);
    if !max_logit.is_finite() {
        return None;
    }
    let mut weights = Vec::with_capacity(indices.len());
    for &idx in &indices {
        let weight = (logits[idx] - max_logit).exp();
        weights.push(if weight.is_finite() && weight > 0.0 {
            weight
        } else {
            0.0
        });
    }
    if weights.iter().all(|w| *w <= 0.0) {
        return indices
            .iter()
            .copied()
            .max_by(|&a, &b| logits[a].partial_cmp(&logits[b]).unwrap_or(Ordering::Equal));
    }
    let dist = WeightedIndex::new(&weights).ok()?;
    indices.get(dist.sample(rng)).copied()
}

#[cfg(test)]
mod tests {
    use super::{TokenSelectionParams, init_rng, select_token_id_from_logits_values};

    struct DummyParams;

    impl TokenSelectionParams for DummyParams {
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

    #[test]
    fn select_token_id_from_logits_values_picks_argmax() {
        let logits = vec![0.1_f32, 3.0, 1.5];
        let mut rng = init_rng(Some(7));
        let token = select_token_id_from_logits_values(&logits, &DummyParams, &[], &mut rng)
            .expect("selection should succeed");
        assert_eq!(token, 1);
    }

    #[test]
    fn select_token_id_from_logits_values_respects_pre_filtered_logits() {
        let logits = vec![f32::NEG_INFINITY, 3.0_f32, 1.5];
        let mut rng = init_rng(Some(7));
        let token = select_token_id_from_logits_values(&logits, &DummyParams, &[], &mut rng)
            .expect("selection should succeed");
        assert_eq!(token, 1);
    }
}
