use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use uuid::Uuid;

/// Persisted observation record stored on disk
#[derive(Debug, Serialize, Deserialize)]
struct ObservationRecord {
    obs_id: String,
    tool: String,
    created_at: String,
    content: String,
}

/// Two-tier storage for Endless Mode.
///
/// Full tool outputs are archived here using a UUID key.
/// The active context receives only the compact summary + the UUID,
/// allowing retrieval on demand via `get-observation`.
pub struct ObservationStore {
    cache_dir: PathBuf,
}

impl ObservationStore {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// Archive a full tool output and return a unique observation ID.
    pub async fn save(&self, tool_name: &str, full_output: &str) -> Result<String> {
        fs::create_dir_all(&self.cache_dir).await?;

        let obs_id = Uuid::new_v4().to_string();
        let record = ObservationRecord {
            obs_id: obs_id.clone(),
            tool: tool_name.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            content: full_output.to_string(),
        };

        let file_path = self.cache_dir.join(format!("{}.json", obs_id));
        let json = serde_json::to_string(&record)?;
        fs::write(&file_path, json).await?;

        tracing::debug!(obs_id = %obs_id, tool = %tool_name, "Archived full observation");
        Ok(obs_id)
    }

    /// Retrieve a previously archived full output by its ID.
    ///
    /// Returns `None` if no observation with that ID exists.
    /// Returns an error if the ID is not a valid UUID (prevents path traversal).
    pub async fn get(&self, obs_id: &str) -> Result<Option<String>> {
        // Validate: obs_id must be a valid UUID (no slashes, no path traversal)
        Uuid::parse_str(obs_id)
            .map_err(|_| anyhow!("Invalid obs_id: must be a valid UUID (e.g. 550e8400-e29b-41d4-a716-446655440000)"))?;

        let file_path = self.cache_dir.join(format!("{}.json", obs_id));

        if !file_path.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&file_path).await?;
        let record: ObservationRecord = serde_json::from_str(&json)?;
        Ok(Some(record.content))
    }
}
