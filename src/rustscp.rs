use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Per-project context saved as `.rustscp` in the analyzed project directory.
///
/// Created or updated every time `analyze-project` is called.
/// Read by the `read-context` subcommand, which is invoked by the
/// Claude Code `SessionStart` hook to inject project memory at session start.
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectContext {
    /// Format version (currently "1")
    pub version: String,
    /// When this project was first analyzed
    pub created_at: DateTime<Utc>,
    /// When this project was last analyzed
    pub updated_at: DateTime<Utc>,
    /// Project name (from Cargo.toml / package.json / etc.)
    pub project_name: String,
    /// Detected project type: rust, node, python, dotnet, go, java, php
    pub project_type: String,
    /// Project version if available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_version: Option<String>,
    /// Detected framework (e.g. "axum", "react", "laravel")
    pub framework: String,
    /// Compact one-line summary produced by build_compact_context_string
    pub summary: String,
    /// Observation UUID if Endless Mode was active during the last analysis
    #[serde(skip_serializing_if = "Option::is_none")]
    pub obs_id: Option<String>,
    /// Suggestion messages from the last analysis
    #[serde(default)]
    pub suggestions: Vec<String>,
}

impl ProjectContext {
    /// Write this context as `.rustscp` into `project_dir`.
    pub fn save(&self, project_dir: &Path) -> Result<()> {
        let path = project_dir.join(".rustscp");
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load `.rustscp` from `project_dir`. Returns `None` if the file does not exist.
    pub fn load(project_dir: &Path) -> Result<Option<Self>> {
        let path = project_dir.join(".rustscp");
        if !path.exists() {
            return Ok(None);
        }
        let json = std::fs::read_to_string(path)?;
        let ctx: Self = serde_json::from_str(&json)?;
        Ok(Some(ctx))
    }

    /// Format this context as human/Claude-readable text for injection at session start.
    pub fn format_for_claude(&self) -> String {
        let mut out = String::new();
        out.push_str("# Project Context (.rustscp)\n\n");
        out.push_str(&format!(
            "**Project:** {} v{}\n",
            self.project_name,
            self.project_version.as_deref().unwrap_or("?")
        ));
        out.push_str(&format!(
            "**Type:** {} | **Framework:** {}\n",
            self.project_type, self.framework
        ));
        out.push_str(&format!(
            "**Last analyzed:** {}\n\n",
            self.updated_at.format("%Y-%m-%d %H:%M UTC")
        ));
        out.push_str(&format!("**Summary:** {}\n", self.summary));
        if !self.suggestions.is_empty() {
            out.push_str("\n**Suggestions from last analysis:**\n");
            for s in &self.suggestions {
                out.push_str(&format!("- {}\n", s));
            }
        }
        if let Some(obs_id) = &self.obs_id {
            out.push_str(&format!(
                "\n*Full analysis available: get-observation {{ \"obs_id\": \"{}\" }}*\n",
                obs_id
            ));
        }
        out
    }
}
