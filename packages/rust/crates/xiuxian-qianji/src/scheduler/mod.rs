//! Asynchronous synaptic-flow scheduler.

use crate::contracts::{FlowInstruction, NodeStatus};
use crate::engine::QianjiEngine;
use crate::error::QianjiError;
use petgraph::Direction;
use petgraph::visit::EdgeRef;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Drives the parallel execution of the Qianji Box mechanisms.
pub struct QianjiScheduler {
    /// Thread-safe access to the underlying graph.
    engine: Arc<RwLock<QianjiEngine>>,
    /// Maximum total execution steps to prevent runaway loops.
    max_total_steps: u32,
}

impl QianjiScheduler {
    /// Creates a new scheduler for the given engine.
    #[must_use]
    pub fn new(engine: QianjiEngine) -> Self {
        Self {
            engine: Arc::new(RwLock::new(engine)),
            max_total_steps: 100, // Default artisan threshold
        }
    }

    /// Execute the graph following Synaptic-Flow: entropy-aware dependency resolution.
    ///
    /// # Errors
    ///
    /// Returns [`QianjiError`] when execution exceeds the configured step budget, a
    /// mechanism aborts explicitly, or runtime execution fails.
    #[allow(clippy::too_many_lines)]
    pub async fn run(
        &self,
        initial_context: serde_json::Value,
    ) -> Result<serde_json::Value, QianjiError> {
        let mut context = initial_context;
        let mut active_branches: HashSet<String> = HashSet::new();
        let mut total_steps = 0;

        loop {
            total_steps += 1;
            if total_steps > self.max_total_steps {
                return Err(QianjiError::DriftError(
                    "Maximum execution steps exceeded (Potential infinite loop)".to_string(),
                ));
            }

            let mut pending_nodes = Vec::new();

            // 1. Scan for ready nodes
            {
                let engine = self.engine.read().await;
                for node_idx in engine.graph.node_indices() {
                    let node = &engine.graph[node_idx];
                    if node.status == NodeStatus::Idle {
                        let all_parents_done = engine
                            .graph
                            .edges_directed(node_idx, Direction::Incoming)
                            .all(|edge| {
                                let parent_idx = edge.source();
                                let edge_data = edge.weight();

                                let done = engine.graph[parent_idx].status == NodeStatus::Completed;
                                let branch_match = if let Some(ref label) = edge_data.label {
                                    active_branches.contains(label)
                                } else {
                                    true
                                };

                                done && branch_match
                            });

                        if all_parents_done {
                            pending_nodes.push(node_idx);
                        }
                    }
                }
            }

            if pending_nodes.is_empty() {
                let engine = self.engine.read().await;
                if engine
                    .graph
                    .node_weights()
                    .all(|n| n.status == NodeStatus::Completed || n.status == NodeStatus::Idle)
                {
                    break;
                }
                break;
            }

            // 2. Parallel Execution
            let mut tasks = Vec::new();
            for idx in pending_nodes {
                let engine_clone = self.engine.clone();
                let context_clone = context.clone();

                tasks.push(tokio::spawn(async move {
                    let mechanism = {
                        let mut engine = engine_clone.write().await;
                        engine.graph[idx].status = NodeStatus::Executing;
                        engine.graph[idx].mechanism.clone()
                    };

                    let result = mechanism.execute(&context_clone).await;

                    let mut engine = engine_clone.write().await;
                    match result {
                        Ok(out) => {
                            engine.graph[idx].status = NodeStatus::Completed;
                            Ok(out)
                        }
                        Err(e) => {
                            engine.graph[idx].status = NodeStatus::Failed(e.clone());
                            Err(e)
                        }
                    }
                }));
            }

            let results = futures::future::join_all(tasks).await;
            for res in results {
                if let Ok(Ok(output)) = res {
                    // Handle Instructions (Synapse-Audit & Probabilistic Branching)
                    match output.instruction {
                        FlowInstruction::SelectBranch(branch) => {
                            active_branches.insert(branch);
                        }
                        FlowInstruction::RetryNodes(node_ids) => {
                            let mut engine = self.engine.write().await;
                            let mut to_reset = std::collections::HashSet::new();

                            let initial_indices: Vec<_> = engine
                                .graph
                                .node_indices()
                                .filter(|&idx| node_ids.contains(&engine.graph[idx].id))
                                .collect();

                            for start_idx in initial_indices {
                                let mut bfs = petgraph::visit::Bfs::new(&engine.graph, start_idx);
                                while let Some(visited) = bfs.next(&engine.graph) {
                                    to_reset.insert(visited);
                                }
                            }

                            for idx in to_reset {
                                engine.graph[idx].status = NodeStatus::Idle;
                            }
                        }
                        FlowInstruction::Abort(reason) => {
                            return Err(QianjiError::ExecutionError(reason));
                        }
                        FlowInstruction::Continue => {}
                    }

                    // Merge Data
                    if let Some(obj) = output.data.as_object() {
                        for (k, v) in obj {
                            context[k] = v.clone();
                        }
                    }
                }
            }
        }

        Ok(context)
    }
}
