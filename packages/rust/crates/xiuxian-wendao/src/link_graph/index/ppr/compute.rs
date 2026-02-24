use super::super::{LinkGraphIndex, LinkGraphRelatedPprOptions};
use super::runtime::resolve_related_ppr_runtime;
use super::types::RelatedPprComputation;
use crate::link_graph::runtime_config::resolve_link_graph_related_runtime;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

mod finalize;
mod orchestrate;

use finalize::finalize_related_ppr_result;
use orchestrate::run_related_ppr_orchestration;

#[derive(Debug, Clone)]
pub(super) struct RelatedPprKernelTelemetry {
    pub(super) fused_scores_by_doc_id: HashMap<String, f64>,
    pub(super) iteration_count: usize,
    pub(super) final_residual: f64,
    pub(super) subgraph_count: usize,
    pub(super) partition_sizes: Vec<usize>,
    pub(super) partition_duration_ms: f64,
    pub(super) kernel_duration_ms: f64,
    pub(super) fusion_duration_ms: f64,
    pub(super) timed_out: bool,
}

impl LinkGraphIndex {
    pub(in crate::link_graph::index) fn related_ppr_compute(
        &self,
        seeds: &HashMap<String, f64>,
        max_distance: usize,
        options: Option<&LinkGraphRelatedPprOptions>,
    ) -> Option<RelatedPprComputation> {
        let total_start = Instant::now();
        if seeds.is_empty() {
            return None;
        }

        let seed_ids: HashSet<String> = seeds.keys().cloned().collect();
        let runtime = resolve_link_graph_related_runtime();
        let candidate_cap = runtime.max_candidates.max(1);
        let max_partitions = runtime.max_partitions.max(1);
        let time_budget_ms = runtime.time_budget_ms.max(1.0);
        let budget_duration = Duration::from_secs_f64(time_budget_ms / 1000.0);

        let bounded_distance = max_distance.max(1);
        let raw_horizon_distances =
            self.collect_bidirectional_distance_map(&seed_ids, bounded_distance);
        if raw_horizon_distances.is_empty() {
            return None;
        }
        let raw_candidate_count =
            Self::candidate_count_from_horizon(&raw_horizon_distances, &seed_ids);
        let candidate_capped = raw_candidate_count > candidate_cap;
        let horizon_distances = if candidate_capped {
            self.trim_horizon_candidates(&raw_horizon_distances, &seed_ids, candidate_cap)
        } else {
            raw_horizon_distances
        };

        let (alpha, max_iter, tol, subgraph_mode) = resolve_related_ppr_runtime(options);
        let restrict_to_horizon = match subgraph_mode {
            super::super::LinkGraphPprSubgraphMode::Disabled => false,
            super::super::LinkGraphPprSubgraphMode::Force => true,
            super::super::LinkGraphPprSubgraphMode::Auto => {
                horizon_distances.len() < self.docs_by_id.len()
            }
        };

        let graph_nodes =
            self.build_graph_nodes_for_related_ppr(&horizon_distances, restrict_to_horizon);
        if graph_nodes.is_empty() {
            return None;
        }
        let candidate_count = Self::candidate_count_from_horizon(&horizon_distances, &seed_ids);
        // Time budget is applied to the kernel/orchestration phase, not precompute setup.
        let deadline = Some(Instant::now() + budget_duration);

        let telemetry = run_related_ppr_orchestration(
            self,
            seeds,
            &graph_nodes,
            bounded_distance,
            alpha,
            max_iter,
            tol,
            subgraph_mode,
            restrict_to_horizon,
            max_partitions,
            deadline,
        )?;

        Some(finalize_related_ppr_result(
            self,
            &seed_ids,
            horizon_distances,
            &graph_nodes,
            alpha,
            max_iter,
            tol,
            subgraph_mode,
            restrict_to_horizon,
            candidate_count,
            candidate_cap,
            candidate_capped,
            time_budget_ms,
            &total_start,
            &telemetry,
        ))
    }

    pub(in crate::link_graph::index) fn related_ppr_ranked_doc_ids(
        &self,
        seeds: &HashMap<String, f64>,
        max_distance: usize,
        options: Option<&LinkGraphRelatedPprOptions>,
    ) -> Vec<(String, usize, f64)> {
        self.related_ppr_compute(seeds, max_distance, options)
            .map(|row| row.ranked_doc_ids)
            .unwrap_or_default()
    }
}
