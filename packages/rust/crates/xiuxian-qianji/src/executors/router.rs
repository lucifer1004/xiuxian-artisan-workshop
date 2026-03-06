//! Probabilistic MDP routing mechanism with Detailed Debugging.

use crate::contracts::{FlowInstruction, QianjiMechanism, QianjiOutput};
use async_trait::async_trait;
use log::{info, warn};
use rand::Rng;
use regex::Regex;
use serde_json::json;

pub struct ProbabilisticRouter {
    pub branches: Vec<(String, f32)>,
}

#[async_trait]
impl QianjiMechanism for ProbabilisticRouter {
    async fn execute(&self, context: &serde_json::Value) -> Result<QianjiOutput, String> {
        // 1. Semantic Score Extraction
        let confidence_bias = extract_omega_score(context).unwrap_or(1.0);
        info!("Router: Parsed confidence_bias = {}", confidence_bias);

        if self.branches.is_empty() {
            warn!("Router: No downstream branches defined in topology!");
        }

        let total_weight: f32 = self
            .branches
            .iter()
            .map(|(name, w)| {
                let eff = (*w * confidence_bias).max(0.0);
                info!("Router: Branch '{}' has effective weight {}", name, eff);
                eff
            })
            .sum();

        let selected_branch = if total_weight <= 0.0 {
            let first = self
                .branches
                .first()
                .map(|(n, _)| n.clone())
                .unwrap_or_default();
            warn!(
                "Router: Total weight is zero; falling back to first branch: '{}'",
                first
            );
            first
        } else {
            let mut rng = rand::thread_rng();
            let mut pick = rng.gen_range(0.0..total_weight);
            let mut selected = String::new();
            for (name, weight) in &self.branches {
                let effective_weight = (*weight * confidence_bias).max(0.0);
                pick -= effective_weight;
                if pick <= 0.0 {
                    selected.clone_from(name);
                    break;
                }
            }
            if selected.is_empty() {
                self.branches
                    .last()
                    .map(|(n, _)| n.clone())
                    .unwrap_or_default()
            } else {
                selected
            }
        };

        info!("Router: Final decision -> '{}'", selected_branch);

        Ok(QianjiOutput {
            data: json!({
                "selected_route": selected_branch,
                "confidence_resolved": confidence_bias,
                "available_branches": self.branches.iter().map(|(n,_)| n).collect::<Vec<_>>()
            }),
            instruction: FlowInstruction::SelectBranch(selected_branch),
        })
    }

    fn weight(&self) -> f32 {
        1.0
    }
}

fn extract_omega_score(context: &serde_json::Value) -> Option<f32> {
    // Check multiple keys where the score might be hidden
    let keys = vec![
        "omega_confidence",
        "critic_feedback",
        "critic_raw_reasoning",
    ];

    for key in keys {
        if let Some(raw_val) = context.get(key) {
            if let Some(f) = raw_val.as_f64() {
                return Some(f as f32);
            }
            if let Some(text) = raw_val.as_str() {
                let re = Regex::new(r"(?i)omega_confidence\s*:\s*([0-9]*\.?[0-9]+)").ok()?;
                if let Some(caps) = re.captures(text) {
                    return caps.get(1)?.as_str().parse::<f32>().ok();
                }
            }
        }
    }
    None
}
