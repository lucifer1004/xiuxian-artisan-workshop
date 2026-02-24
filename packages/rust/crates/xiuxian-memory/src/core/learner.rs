//! MemRL Q-Learning implementation.

use crate::core::types::{MemoryAction, MemoryState};
use std::collections::HashMap;

/// The "Account Book" (Q-Table) and its "Accounting Rules" (Q-Learning).
pub struct MemRLCortex {
    /// Mapping: (State, Action) -> Expected Utility (Q-Value).
    pub q_table: HashMap<(MemoryState, MemoryAction), f64>,
    /// Learning rate (α).
    pub alpha: f64,
    /// Discount factor (γ).
    pub gamma: f64,
}

impl Default for MemRLCortex {
    fn default() -> Self {
        Self::new()
    }
}

impl MemRLCortex {
    /// Initialize a new cortex with standard H-MAC parameters.
    #[must_use]
    pub fn new() -> Self {
        Self {
            q_table: HashMap::new(),
            alpha: 0.1,
            gamma: 0.9,
        }
    }

    /// Selects the best action for a given state using an epsilon-greedy policy.
    #[must_use]
    pub fn decide(&self, state: &MemoryState) -> MemoryAction {
        MemoryAction::all()
            .into_iter()
            .max_by(|a, b| {
                let q_a = self.q_table.get(&(state.clone(), *a)).unwrap_or(&0.0);
                let q_b = self.q_table.get(&(state.clone(), *b)).unwrap_or(&0.0);
                q_a.partial_cmp(q_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(MemoryAction::Retain)
    }

    /// The core Q-Learning update: Q(s,a) = Q(s,a) + α[R + γ*maxQ(s',a') - Q(s,a)]
    pub fn update(&mut self, s: MemoryState, a: MemoryAction, reward: f64, s_next: &MemoryState) {
        let old_q = *self.q_table.get(&(s.clone(), a)).unwrap_or(&0.0);

        // Find max Q(s', a')
        let max_next_q = MemoryAction::all()
            .into_iter()
            .map(|a_next| *self.q_table.get(&(s_next.clone(), a_next)).unwrap_or(&0.0))
            .fold(f64::NEG_INFINITY, f64::max);

        let max_next_q = if max_next_q.is_infinite() {
            0.0
        } else {
            max_next_q
        };

        // Bellman update
        let new_q = old_q + self.alpha * (reward + self.gamma * max_next_q - old_q);

        self.q_table.insert((s, a), new_q);
    }
}
