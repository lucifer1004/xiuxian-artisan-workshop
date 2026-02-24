use chrono::Utc;

use crate::types::KnowledgeEntry;

use super::KnowledgeStorage;

impl KnowledgeStorage {
    /// Initialize the storage (validate Valkey connectivity).
    ///
    /// # Errors
    ///
    /// Returns an error when `Valkey` connectivity check fails.
    #[allow(clippy::unused_async)]
    pub async fn init(&self) -> Result<(), Box<dyn std::error::Error>> {
        let client = self.redis_client()?;
        let mut conn = client.get_connection()?;
        let _pong: String = redis::cmd("PING").query(&mut conn)?;
        Ok(())
    }

    /// Upsert a knowledge entry.
    ///
    /// # Errors
    ///
    /// Returns an error when serialization fails or `Valkey` operations fail.
    #[allow(clippy::unused_async)]
    pub async fn upsert(&self, entry: &KnowledgeEntry) -> Result<(), Box<dyn std::error::Error>> {
        self.init().await?;
        let client = self.redis_client()?;
        let mut conn = client.get_connection()?;
        let entries_key = self.entries_key();
        let existing_raw: Option<String> = redis::cmd("HGET")
            .arg(&entries_key)
            .arg(&entry.id)
            .query(&mut conn)?;
        let existing = existing_raw
            .as_deref()
            .map(serde_json::from_str::<KnowledgeEntry>)
            .transpose()?;

        let now = Utc::now();
        let (created_at, version) = if let Some(found) = existing {
            (found.created_at, found.version + 1)
        } else {
            (now, entry.version.max(1))
        };

        let mut to_store = entry.clone();
        to_store.created_at = created_at;
        to_store.updated_at = now;
        to_store.version = version;
        let payload = serde_json::to_string(&to_store)?;

        let _: i64 = redis::cmd("HSET")
            .arg(entries_key)
            .arg(&to_store.id)
            .arg(payload)
            .query(&mut conn)?;
        Ok(())
    }

    /// Count total entries.
    ///
    /// # Errors
    ///
    /// Returns an error when `Valkey` operations fail.
    #[allow(clippy::unused_async)]
    pub async fn count(&self) -> Result<i64, Box<dyn std::error::Error>> {
        let client = self.redis_client()?;
        let mut conn = client.get_connection()?;
        let total: i64 = redis::cmd("HLEN")
            .arg(self.entries_key())
            .query(&mut conn)?;
        Ok(total)
    }

    /// Delete an entry by ID.
    ///
    /// # Errors
    ///
    /// Returns an error when `Valkey` operations fail.
    #[allow(clippy::unused_async)]
    pub async fn delete(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let client = self.redis_client()?;
        let mut conn = client.get_connection()?;
        let _: i64 = redis::cmd("HDEL")
            .arg(self.entries_key())
            .arg(id)
            .query(&mut conn)?;
        Ok(())
    }

    /// Clear all entries.
    ///
    /// # Errors
    ///
    /// Returns an error when `Valkey` operations fail.
    #[allow(clippy::unused_async)]
    pub async fn clear(&self) -> Result<(), Box<dyn std::error::Error>> {
        let client = self.redis_client()?;
        let mut conn = client.get_connection()?;
        let _: i64 = redis::cmd("DEL").arg(self.entries_key()).query(&mut conn)?;
        Ok(())
    }
}
