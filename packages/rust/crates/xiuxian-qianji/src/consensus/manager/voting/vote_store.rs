use super::super::keys::{VoteKeys, VoteSnapshot};
use super::super::{CONSENSUS_VOTE_TTL_SECONDS, ConsensusManager};
use crate::consensus::models::AgentVote;
use anyhow::Result;

impl ConsensusManager {
    pub(super) async fn record_vote(
        &self,
        keys: VoteKeys,
        vote: &AgentVote,
    ) -> Result<VoteSnapshot> {
        let payload = serde_json::to_string(vote)?;

        let _: i64 = self
            .run_command("consensus_store_vote_payload", || {
                let mut command = redis::cmd("HSET");
                command
                    .arg(&keys.votes_hash)
                    .arg(&vote.agent_id)
                    .arg(&payload);
                command
            })
            .await?;

        let new_weight: f64 = self
            .run_command("consensus_increment_hash_weight", || {
                let mut command = redis::cmd("HINCRBYFLOAT");
                command
                    .arg(&keys.weight_counter)
                    .arg(&vote.output_hash)
                    .arg(vote.weight);
                command
            })
            .await?;

        let total_agents: usize = self
            .run_command("consensus_read_total_agents", || {
                let mut command = redis::cmd("HLEN");
                command.arg(&keys.votes_hash);
                command
            })
            .await?;

        let vote_ts = u64::try_from(vote.timestamp_ms).unwrap_or(u64::MAX);
        let _: i64 = self
            .run_command("consensus_set_first_seen_if_absent", || {
                let mut command = redis::cmd("SETNX");
                command.arg(&keys.first_seen_marker).arg(vote_ts);
                command
            })
            .await?;

        self.refresh_ttls(&keys).await?;

        Ok(VoteSnapshot {
            total_agents,
            hash_weight: new_weight,
        })
    }

    async fn refresh_ttls(&self, keys: &VoteKeys) -> Result<()> {
        let ttl = CONSENSUS_VOTE_TTL_SECONDS;
        let _: bool = self
            .run_command("consensus_expire_votes", || {
                let mut command = redis::cmd("EXPIRE");
                command.arg(&keys.votes_hash).arg(ttl);
                command
            })
            .await?;
        let _: bool = self
            .run_command("consensus_expire_counts", || {
                let mut command = redis::cmd("EXPIRE");
                command.arg(&keys.weight_counter).arg(ttl);
                command
            })
            .await?;
        let _: bool = self
            .run_command("consensus_expire_first_seen", || {
                let mut command = redis::cmd("EXPIRE");
                command.arg(&keys.first_seen_marker).arg(ttl);
                command
            })
            .await?;
        let _: bool = self
            .run_command("consensus_expire_winner", || {
                let mut command = redis::cmd("EXPIRE");
                command.arg(&keys.winner_marker).arg(ttl);
                command
            })
            .await?;
        let _: bool = self
            .run_command("consensus_expire_outputs", || {
                let mut command = redis::cmd("EXPIRE");
                command.arg(&keys.output_payloads).arg(ttl);
                command
            })
            .await?;
        Ok(())
    }

    pub(super) async fn store_output_payload(
        &self,
        keys: &VoteKeys,
        output_hash: &str,
        payload: &str,
    ) -> Result<()> {
        let _: i64 = self
            .run_command("consensus_store_output_payload", || {
                let mut command = redis::cmd("HSET");
                command
                    .arg(&keys.output_payloads)
                    .arg(output_hash)
                    .arg(payload);
                command
            })
            .await?;
        Ok(())
    }
}
