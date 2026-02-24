//! xiuxian-memory: `MemRL` self-evolving memory system.

pub mod core;

pub use core::learner::MemRLCortex;
pub use core::types::{MemoryAction, MemoryState};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_q_learning_evolution() {
        let mut cortex = MemRLCortex::new();

        let s1 = MemoryState {
            context_entropy: 5, // High uncertainty
            persona_hash: 123,
            task_kind: "Research".to_string(),
        };

        let action = MemoryAction::Promote;
        let reward = 10.0; // Significant success

        let s2 = MemoryState {
            context_entropy: 1, // Success led to low uncertainty
            persona_hash: 123,
            task_kind: "Research".to_string(),
        };

        // 1. Initial Q-Value should be 0
        let initial_q = *cortex.q_table.get(&(s1.clone(), action)).unwrap_or(&0.0);
        assert!((initial_q - 0.0).abs() < f64::EPSILON);

        // 2. Perform several updates (learning cycle)
        for _ in 0..5 {
            cortex.update(s1.clone(), action, reward, &s2);
        }

        // 3. Q-Value should have risen significantly
        let Some(evolved_q) = cortex.q_table.get(&(s1.clone(), action)) else {
            panic!("expected learned Q-value to be present");
        };
        let evolved_q = *evolved_q;
        println!("Evolved Q-Value: {evolved_q}");
        assert!(evolved_q > 4.0);

        // 4. Decision check: In state s1, it should now prefer 'Promote'
        assert_eq!(cortex.decide(&s1), MemoryAction::Promote);
    }
}
