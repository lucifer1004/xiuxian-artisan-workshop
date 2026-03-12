//! Q-Table implementation for self-evolving memory.
//!
//! Implements Q-Learning algorithm: Q_new = Q_old + α * (r - Q_old)
//! where α is the learning rate and r is the reward.

use dashmap::DashMap;
use std::sync::RwLock;

/// Q-Learning table for episode utility tracking.
///
/// Uses a concurrent hash map for thread-safe updates.
pub struct QTable {
    /// Internal Q-table mapping episode_id -> q_value
    table: RwLock<DashMap<String, f32>>,
    /// Learning rate (α) - typically 0.1-0.3
    learning_rate: f32,
    /// Discount factor (γ) - typically 0.9-0.99
    discount_factor: f32,
}

// Manual Clone implementation - creates a new empty table
impl Clone for QTable {
    fn clone(&self) -> Self {
        Self {
            table: RwLock::new(DashMap::new()),
            learning_rate: self.learning_rate,
            discount_factor: self.discount_factor,
        }
    }
}

impl QTable {
    /// Create a new Q-Table with default parameters.
    ///
    /// Default learning_rate = 0.2
    /// Default discount_factor = 0.95
    pub fn new() -> Self {
        Self::with_params(0.2, 0.95)
    }

    /// Create a new Q-Table with custom parameters.
    ///
    /// # Arguments
    /// * `learning_rate` - α in Q-learning, controls how much new info overrides old
    /// * `discount_factor` - γ in Q-learning, balances immediate vs future reward
    pub fn with_params(learning_rate: f32, discount_factor: f32) -> Self {
        Self {
            table: RwLock::new(DashMap::new()),
            learning_rate,
            discount_factor,
        }
    }

    /// Update Q-value for an episode using Q-learning.
    ///
    /// Q_new = Q_old + α * (reward - Q_old)
    ///
    /// # Arguments
    /// * `episode_id` - The episode identifier
    /// * `reward` - The reward signal (typically 0.0-1.0)
    ///
    /// # Returns
    /// The new Q-value after update
    pub fn update(&self, episode_id: &str, reward: f32) -> f32 {
        let q_old = self.get_q(episode_id);
        let q_new = q_old + self.learning_rate * (reward - q_old);

        // Clamp Q-value to [0.0, 1.0] range
        let q_clamped = q_new.clamp(0.0, 1.0);

        self.table
            .write()
            .unwrap()
            .insert(episode_id.to_string(), q_clamped);

        q_clamped
    }

    /// Get the Q-value for an episode.
    ///
    /// Returns default 0.5 if episode not found (initial Q-value).
    pub fn get_q(&self, episode_id: &str) -> f32 {
        self.table
            .read()
            .unwrap()
            .get(episode_id)
            .map(|v| *v.value())
            .unwrap_or(0.5)
    }

    /// Initialize Q-value for a new episode.
    pub fn init_episode(&self, episode_id: &str) {
        let table = self.table.write().unwrap();
        if !table.contains_key(episode_id) {
            table.insert(episode_id.to_string(), 0.5);
        }
    }

    /// Get multiple Q-values at once.
    pub fn get_batch(&self, episode_ids: &[String]) -> Vec<(String, f32)> {
        let table = self.table.read().unwrap();
        episode_ids
            .iter()
            .map(|id| {
                let q = table.get(id).map(|v| *v.value()).unwrap_or(0.5);
                (id.clone(), q)
            })
            .collect()
    }

    /// Batch update multiple Q-values.
    ///
    /// More efficient than individual updates.
    pub fn update_batch(&self, updates: &[(String, f32)]) -> Vec<(String, f32)> {
        let table = self.table.write().unwrap();
        updates
            .iter()
            .map(|(episode_id, reward)| {
                let q_old = table.get(episode_id).map(|v| *v.value()).unwrap_or(0.5);
                let q_new = q_old + self.learning_rate * (reward - q_old);
                let q_clamped = q_new.clamp(0.0, 1.0);
                table.insert(episode_id.clone(), q_clamped);
                (episode_id.clone(), q_clamped)
            })
            .collect()
    }

    /// Get all episode IDs in the Q-table.
    pub fn get_all_ids(&self) -> Vec<String> {
        self.table
            .read()
            .unwrap()
            .iter()
            .map(|r| r.key().clone())
            .collect()
    }

    /// Get the number of entries in the Q-table.
    pub fn len(&self) -> usize {
        self.table.read().unwrap().len()
    }

    /// Check if the Q-table is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Remove an entry from the Q-table.
    pub fn remove(&self, episode_id: &str) {
        self.table.write().unwrap().remove(episode_id);
    }

    /// Save Q-table to JSON file.
    pub fn save(&self, path: &str) -> Result<(), anyhow::Error> {
        let data: std::collections::HashMap<String, f32> = self
            .table
            .read()
            .unwrap()
            .iter()
            .map(|r| (r.key().clone(), *r.value()))
            .collect();
        let json = serde_json::to_string_pretty(&data)?;
        std::fs::write(path, json)?;
        log::info!("Saved Q-table with {} entries to {}", data.len(), path);
        Ok(())
    }

    /// Load Q-table from JSON file.
    pub fn load(&mut self, path: &str) -> Result<(), anyhow::Error> {
        if !std::path::Path::new(path).exists() {
            log::info!("No existing Q-table file at {}", path);
            return Ok(());
        }
        let json = std::fs::read_to_string(path)?;
        let data: std::collections::HashMap<String, f32> = serde_json::from_str(&json)?;
        let count = data.len();
        *self.table.write().unwrap() = DashMap::from_iter(data);
        log::info!("Loaded {} Q-table entries from {}", count, path);
        Ok(())
    }

    /// Get learning rate.
    pub fn learning_rate(&self) -> f32 {
        self.learning_rate
    }

    /// Get discount factor.
    pub fn discount_factor(&self) -> f32 {
        self.discount_factor
    }

    /// Get a snapshot of the Q-table as a HashMap.
    #[must_use]
    pub fn snapshot_map(&self) -> std::collections::HashMap<String, f32> {
        self.table
            .read()
            .unwrap()
            .iter()
            .map(|r| (r.key().clone(), *r.value()))
            .collect()
    }

    /// Replace the Q-table contents from a HashMap.
    pub fn replace_map(&mut self, data: std::collections::HashMap<String, f32>) {
        *self.table.write().unwrap() = DashMap::from_iter(data);
    }
}

impl Default for QTable {
    fn default() -> Self {
        Self::new()
    }
}
