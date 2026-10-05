//! Persistent memory store for Phase 1 Memory Core.
//!
//! Provides `remember`, `recall`, and `get-memory` capabilities so the MCP server
//! can act as a true long-term memory engine for AI coding sessions.
//!
//! Design decisions (recorded 2026):
//! - Storage: JSON files first (single `memories.json`), SQLite later if needed (hybrid approach).
//! - Scope: Global + per-project supported from day one.
//! - Content: Structured facts/decisions/gotchas/etc. (no full transcripts in v1).
//! - API parity: Tool names and core shapes chosen with mcp-go-context compatibility in mind.

use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use crate::types::{Memory, MemoryScope, MemorySearchCriteria, RememberInput};

/// In-memory + JSON-backed store for persistent memories.
#[derive(Clone)]
pub struct MemoryStore {
    memories: Vec<Memory>,
    storage_path: PathBuf,
    /// Index: scope_key -> list of indices in `memories`
    scope_index: HashMap<String, Vec<usize>>,
    category_index: HashMap<String, Vec<usize>>,
}

impl MemoryStore {
    pub fn new(storage_path: impl Into<PathBuf>) -> Self {
        Self {
            memories: vec![],
            storage_path: storage_path.into(),
            scope_index: HashMap::new(),
            category_index: HashMap::new(),
        }
    }

    /// Load all memories from disk (single JSON file for v1).
    pub async fn load(&mut self) -> Result<()> {
        self.memories.clear();
        self.scope_index.clear();
        self.category_index.clear();

        let file_path = self.storage_path.join("memories.json");

        if !file_path.exists() {
            tracing::debug!(path = %file_path.display(), "No memories file yet (fresh start)");
            return Ok(());
        }

        #[derive(serde::Deserialize)]
        struct MemoriesFile {
            memories: Vec<Memory>,
        }

        let content = fs::read_to_string(&file_path).context("Failed to read memories.json")?;

        let file: MemoriesFile =
            serde_json::from_str(&content).context("Failed to parse memories.json")?;

        let mut seen_ids: HashSet<String> = HashSet::new();
        for memory in file.memories {
            if seen_ids.insert(memory.id.clone()) {
                self.memories.push(memory);
            } else {
                tracing::warn!(id = %memory.id, "Duplicate memory id skipped during load");
            }
        }

        self.rebuild_indexes();

        tracing::info!(
            count = self.memories.len(),
            path = %self.storage_path.display(),
            "Loaded memories"
        );
        Ok(())
    }

    fn rebuild_indexes(&mut self) {
        self.scope_index.clear();
        self.category_index.clear();

        for (idx, mem) in self.memories.iter().enumerate() {
            let scope_key = Self::scope_key(&mem.scope);
            self.scope_index.entry(scope_key).or_default().push(idx);

            self.category_index
                .entry(mem.category.clone())
                .or_default()
                .push(idx);
        }
    }

    fn scope_key(scope: &MemoryScope) -> String {
        match scope {
            MemoryScope::Global => "global".to_string(),
            MemoryScope::Project { path } => path.clone(),
        }
    }

    /// Persist current memories to disk.
    pub async fn save(&self) -> Result<()> {
        fs::create_dir_all(&self.storage_path)
            .context("Failed to create memories storage directory")?;

        let file_path = self.storage_path.join("memories.json");

        #[derive(serde::Serialize)]
        struct MemoriesFile<'a> {
            memories: &'a [Memory],
        }

        let file = MemoriesFile {
            memories: &self.memories,
        };

        let json = serde_json::to_string_pretty(&file).context("Failed to serialize memories")?;

        fs::write(&file_path, json).context(format!("Failed to write {}", file_path.display()))?;

        tracing::debug!(
            count = self.memories.len(),
            path = %file_path.display(),
            "Saved memories"
        );
        Ok(())
    }

    /// Store a new memory (generates id + timestamps if missing).
    ///
    /// The input's scope.project path (if any) should preferably be a canonical path.
    pub fn remember(&mut self, input: RememberInput) -> Result<Memory, String> {
        Self::validate_input(&input)?;

        let now = Utc::now();

        let id = if input.title.is_empty() {
            // Fallback id from content prefix + time (rare)
            format!("mem-{}", Utc::now().timestamp_millis())
        } else {
            // Prefer UUID for global uniqueness and cross-impl compatibility
            uuid::Uuid::new_v4().to_string()
        };

        let memory = Memory {
            id,
            scope: input.scope,
            category: input.category,
            title: input.title,
            content: input.content,
            tags: input.tags,
            importance: input.importance.clamp(0.0, 1.0),
            recall_count: 0,
            created_at: now,
            updated_at: now,
            last_recalled_at: None,
        };

        let idx = self.memories.len();
        let scope_key = Self::scope_key(&memory.scope);
        self.memories.push(memory.clone());

        self.scope_index.entry(scope_key).or_default().push(idx);
        self.category_index
            .entry(memory.category.clone())
            .or_default()
            .push(idx);

        Ok(memory)
    }

    fn validate_input(input: &RememberInput) -> Result<(), String> {
        if input.category.is_empty() {
            return Err("category cannot be empty".to_string());
        }
        if input.category.len() > 64 {
            return Err("category too long (max 64)".to_string());
        }
        // Light sanitization - prevent obvious path tricks even if not used in FS name yet
        if input.category.contains("..")
            || input.category.contains('/')
            || input.category.contains('\\')
            || input.category.contains('\0')
        {
            return Err("category contains invalid characters".to_string());
        }

        if input.title.is_empty() {
            return Err("title cannot be empty".to_string());
        }
        if input.title.len() > 200 {
            return Err("title too long (max 200)".to_string());
        }

        if input.content.trim().is_empty() {
            return Err("content cannot be empty".to_string());
        }
        if input.content.len() > 16_000 {
            return Err("content too long (max 16k chars for v1)".to_string());
        }

        if input.importance < 0.0 || input.importance > 1.0 {
            return Err("importance must be between 0.0 and 1.0".to_string());
        }

        for tag in &input.tags {
            if tag.len() > 64 {
                return Err(format!("tag too long: {}", tag));
            }
        }

        Ok(())
    }

    /// Recall (search) memories with scoring.
    ///
    /// Returns owned (memory, score) pairs sorted by descending score.
    /// We return owned values to keep call sites simple (no borrow conflicts when mutating after recall).
    pub fn recall(&self, criteria: &MemorySearchCriteria) -> Vec<(Memory, f32)> {
        let mut candidates: Vec<usize> = if let Some(ref scope) = criteria.scope {
            if let Some(indices) = self.scope_index.get(&Self::scope_key(scope)) {
                indices.clone()
            } else {
                return vec![];
            }
        } else {
            (0..self.memories.len()).collect()
        };

        // Category filter
        if let Some(ref cat) = criteria.category {
            if let Some(cat_indices) = self.category_index.get(cat) {
                let cat_set: HashSet<usize> = cat_indices.iter().copied().collect();
                candidates.retain(|i| cat_set.contains(i));
            } else {
                return vec![];
            }
        }

        // Score + filter
        let mut scored: Vec<(Memory, f32)> = candidates
            .iter()
            .filter_map(|&idx| {
                let mem = &self.memories[idx];
                let score = self.score_memory(mem, criteria);
                if score >= criteria.min_score {
                    Some((mem.clone(), score))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let limit = criteria.max_results.unwrap_or(20);
        scored.truncate(limit);
        scored
    }

    fn score_memory(&self, mem: &Memory, criteria: &MemorySearchCriteria) -> f32 {
        let mut score = mem.importance; // base

        // Strong boost for explicit recall history (the ones we use are probably important)
        if mem.recall_count > 0 {
            #[allow(clippy::cast_precision_loss)]
            let rec_boost = (mem.recall_count as f32).log10() * 0.08;
            score += rec_boost.min(0.25);
        }

        // Query matching
        if let Some(ref q) = criteria.query {
            let ql = q.to_lowercase();
            let mut matched = false;

            if mem.title.to_lowercase().contains(&ql) {
                score += 0.35;
                matched = true;
            }
            if mem.content.to_lowercase().contains(&ql) {
                score += 0.20;
                matched = true;
            }
            if mem.category.to_lowercase().contains(&ql) {
                score += 0.10;
                matched = true;
            }
            if mem.tags.iter().any(|t| t.to_lowercase().contains(&ql)) {
                score += 0.15;
                matched = true;
            }

            if !matched {
                score *= 0.35;
            }
        }

        // Tag filter boost
        if !criteria.tags.is_empty() {
            let match_count = criteria
                .tags
                .iter()
                .filter(|t| mem.tags.contains(t))
                .count();
            #[allow(clippy::cast_precision_loss)]
            let tag_boost = (match_count as f32 / criteria.tags.len() as f32) * 0.25;
            score += tag_boost;
        }

        // Slight recency preference
        let age_days = (Utc::now() - mem.updated_at).num_days();
        if age_days < 14 {
            score += 0.03;
        }

        score.min(2.0) // cap
    }

    /// Get memories relevant for a given project context + optional task description.
    ///
    /// Always includes globals + the project's own memories.
    /// Used by analyze-project and get-memory tool.
    /// Returns owned values for easy use in callers that also mutate the store.
    pub fn get_relevant_for_project(
        &self,
        project_path: Option<&str>,
        task_hint: Option<&str>,
    ) -> Vec<(Memory, f32)> {
        let mut results = Vec::new();

        // Global always
        if let Some(global_idxs) = self.scope_index.get("global") {
            for &idx in global_idxs {
                let mem = &self.memories[idx];
                let mut s = mem.importance + 0.1; // globals have slight base priority
                if let Some(hint) = task_hint {
                    if mem.title.to_lowercase().contains(&hint.to_lowercase())
                        || mem.content.to_lowercase().contains(&hint.to_lowercase())
                    {
                        s += 0.25;
                    }
                }
                results.push((mem.clone(), s));
            }
        }

        // Project specific
        if let Some(pp) = project_path {
            let key = pp.to_string();
            if let Some(proj_idxs) = self.scope_index.get(&key) {
                for &idx in proj_idxs {
                    let mem = &self.memories[idx];
                    let mut s = mem.importance + 0.15; // project memories usually most relevant
                    if let Some(hint) = task_hint {
                        if mem.title.to_lowercase().contains(&hint.to_lowercase())
                            || mem.content.to_lowercase().contains(&hint.to_lowercase())
                        {
                            s += 0.30;
                        }
                    }
                    results.push((mem.clone(), s));
                }
            }
        }

        // Sort + unique + limit
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(15);
        results
    }

    /// Mark a memory as recalled (bumps counters). Callers should save afterwards if persistence desired.
    pub fn mark_recalled(&mut self, id: &str) -> Result<()> {
        if let Some(mem) = self.memories.iter_mut().find(|m| m.id == id) {
            mem.recall_count += 1;
            mem.last_recalled_at = Some(Utc::now());
            mem.updated_at = Utc::now();
            Ok(())
        } else {
            anyhow::bail!("Memory not found: {}", id)
        }
    }

    /// Get a single memory by id.
    #[allow(dead_code)]
    pub fn get_by_id(&self, id: &str) -> Option<&Memory> {
        self.memories.iter().find(|m| m.id == id)
    }

    /// Total count.
    pub fn len(&self) -> usize {
        self.memories.len()
    }

    /// True if no memories loaded.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.memories.is_empty()
    }

    /// All memories (for stats, export, etc).
    #[allow(dead_code)]
    pub fn get_all(&self) -> &[Memory] {
        &self.memories
    }

    /// Basic stats (used by get-statistics and future memory stats tool).
    #[allow(dead_code)]
    pub fn get_statistics(&self) -> serde_json::Value {
        let total = self.memories.len();
        let globals = self.memories.iter().filter(|m| m.scope.is_global()).count();
        let projects = total - globals;
        let total_recalls: usize = self.memories.iter().map(|m| m.recall_count).sum();

        let mut cats: HashMap<String, usize> = HashMap::new();
        for m in &self.memories {
            *cats.entry(m.category.clone()).or_default() += 1;
        }

        serde_json::json!({
            "total_memories": total,
            "global": globals,
            "project_scoped": projects,
            "total_recalls": total_recalls,
            "categories": cats,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RememberInput;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_remember_and_recall_global() {
        let dir = tempdir().unwrap();
        let mut store = MemoryStore::new(dir.path());

        let input = RememberInput {
            scope: MemoryScope::Global,
            category: "convention".to_string(),
            title: "Always use Result".to_string(),
            content: "Prefer Result<T,E> over unwrap in library code.".to_string(),
            tags: vec!["rust".to_string(), "error-handling".to_string()],
            importance: 0.9,
        };

        let mem = store.remember(input).unwrap();
        assert!(!mem.id.is_empty());
        store.save().await.unwrap();

        let mut store2 = MemoryStore::new(dir.path());
        store2.load().await.unwrap();

        let results = store2.recall(&MemorySearchCriteria {
            query: Some("unwrap".to_string()),
            scope: None,
            ..Default::default()
        });
        assert!(!results.is_empty());
        assert!(results[0].1 > 0.5);
    }

    #[test]
    fn test_project_scope_and_get_relevant() {
        let mut store = MemoryStore::new("/tmp/test-mem");

        let p = "/home/user/myproj".to_string();
        let input = RememberInput {
            scope: MemoryScope::for_project(&p),
            category: "decision".to_string(),
            title: "Use Axum not Actix".to_string(),
            content: "We chose Axum for this service after perf comparison.".to_string(),
            tags: vec!["web".to_string()],
            importance: 0.85,
        };
        store.remember(input).unwrap();

        let relevant = store.get_relevant_for_project(Some(&p), Some("web framework"));
        assert!(!relevant.is_empty());
        assert!(relevant.iter().any(|(m, _)| m.title.contains("Axum")));
    }
}
