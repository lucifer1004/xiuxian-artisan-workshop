//! Advanced Hybrid PPR Kernel for Wendao.
//! Implements `HippoRAG` 2 mixed directed graph (P-E topology).

use petgraph::stable_graph::StableGraph;
use petgraph::visit::EdgeRef;
use std::collections::HashMap;

/// Types of nodes in the `HippoRAG` 2 mixed graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    /// Atomic knowledge entity (Extracted from `OpenIE` triples).
    Entity,
    /// Contextual passage node (Contains text blocks).
    Passage,
}

/// The state of a node within the PPR iteration.
#[derive(Debug, Clone)]
pub struct NodeData {
    /// Unique node identifier.
    pub id: String,
    /// Node semantic type in the mixed graph.
    pub node_type: NodeType,
    /// Current rank value during / after PPR iteration.
    pub rank: f64,
    /// Saliency prior from Hebbian learning.
    pub saliency: f64,
}

/// `HippoRAG` 2 hybrid PPR implementation.
pub struct HybridPprKernel {
    /// Directed weighted graph storage.
    pub graph: StableGraph<NodeData, f32>,
    /// Node id to graph index lookup.
    pub id_to_idx: HashMap<String, petgraph::prelude::NodeIndex>,
}

impl Default for HybridPprKernel {
    fn default() -> Self {
        Self::new()
    }
}

impl HybridPprKernel {
    /// Create an empty hybrid PPR kernel.
    #[must_use]
    pub fn new() -> Self {
        Self {
            graph: StableGraph::new(),
            id_to_idx: HashMap::new(),
        }
    }

    /// Adds a node if not exists.
    pub fn add_node(&mut self, id: &str, node_type: NodeType, saliency: f64) {
        if !self.id_to_idx.contains_key(id) {
            let idx = self.graph.add_node(NodeData {
                id: id.to_string(),
                node_type,
                rank: 0.0,
                saliency,
            });
            self.id_to_idx.insert(id.to_string(), idx);
        }
    }

    /// Adds a weighted edge.
    pub fn add_edge(&mut self, from: &str, to: &str, weight: f32) {
        if let (Some(&f), Some(&t)) = (self.id_to_idx.get(from), self.id_to_idx.get(to)) {
            self.graph.add_edge(f, t, weight);
        }
    }

    /// Run non-uniform PPR.
    pub fn run(&mut self, seeds: &HashMap<String, f64>, alpha: f64, iterations: usize) {
        // 1. Initialize ranks from seeds
        for (id, &val) in seeds {
            if let Some(&idx) = self.id_to_idx.get(id) {
                self.graph[idx].rank = val;
            }
        }

        // 2. Power iteration
        for _ in 0..iterations {
            let mut next_ranks = vec![0.0; self.graph.node_count()];

            // Collect next ranks (immutable phase)
            for idx in self.graph.node_indices() {
                let current_rank = self.graph[idx].rank;
                let out_edges: Vec<_> = self.graph.edges(idx).collect();

                if !out_edges.is_empty() {
                    let total_weight: f32 = out_edges.iter().map(|e| *e.weight()).sum();
                    for edge in out_edges {
                        let target = edge.target();
                        let weight = *edge.weight();
                        next_ranks[target.index()] +=
                            current_rank * (f64::from(weight) / f64::from(total_weight));
                    }
                }
            }

            // Apply damping and seed teleportation (mutable phase)
            let indices: Vec<_> = self.graph.node_indices().collect();
            for idx in indices {
                let seed_prob = seeds.get(&self.graph[idx].id).copied().unwrap_or(0.0);
                let current_saliency = self.graph[idx].saliency;
                let teleport_prob = (seed_prob + current_saliency / 10.0).min(1.0);

                self.graph[idx].rank =
                    (1.0 - alpha) * next_ranks[idx.index()] + alpha * teleport_prob;
            }
        }
    }

    /// Extract top-K nodes.
    #[must_use]
    pub fn top_k(&self, k: usize) -> Vec<(String, f64)> {
        let mut results: Vec<_> = self
            .graph
            .node_indices()
            .map(|idx| (self.graph[idx].id.clone(), self.graph[idx].rank))
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(k);
        results
    }
}
