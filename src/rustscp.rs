//! `.rustscp` project context file management.
//!
//! Generates and reads `.rustscp` files that capture project context
//! (structure, dependencies, detected patterns) so any AI session
//! can bootstrap with full project awareness.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const RUSTSCP_FILENAME: &str = ".rustscp";

/// Persistent project context saved to `.rustscp`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    /// Schema version for forward compatibility
    pub version: u8,
    /// Project name
    pub name: String,
    /// Detected project type (dotnet, rust, node, etc.)
    pub project_type: String,
    /// Target framework / edition / runtime version
    pub framework: Option<String>,
    /// Production dependencies
    pub dependencies: Vec<DepSummary>,
    /// File statistics
    pub stats: FileStats,
    /// Detected patterns from training DB that match this project
    pub matched_patterns: Vec<PatternRef>,
    /// When the context was first created
    pub created_at: DateTime<Utc>,
    /// When the context was last updated
    pub updated_at: DateTime<Utc>,
}

/// Lightweight dependency reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepSummary {
    pub name: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub dev: bool,
}

/// File count statistics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileStats {
    pub total_files: usize,
    /// Breakdown by extension: {"cs": 263, "razor": 310}
    pub by_extension: std::collections::HashMap<String, usize>,
}

/// Reference to a matched pattern (id + title, not full code).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternRef {
    pub id: String,
    pub title: String,
    pub category: String,
    pub score: f32,
}

impl ProjectContext {
    /// Build a `ProjectContext` from an `AnalysisResult`.
    pub fn from_analysis(analysis: &crate::types::AnalysisResult) -> Self {
        let project = &analysis.project;

        // Build dependency list
        let dependencies: Vec<DepSummary> = project
            .dependencies
            .iter()
            .map(|d| DepSummary {
                name: d.name.clone(),
                version: d.version.clone(),
                dev: d.dev_only,
            })
            .collect();

        // Build file stats
        let mut by_extension: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for file in &project.files {
            *by_extension.entry(file.language.clone()).or_default() += 1;
        }
        let stats = FileStats {
            total_files: project.files.len(),
            by_extension,
        };

        // Build pattern refs from matched patterns
        let matched_patterns: Vec<PatternRef> = analysis
            .patterns
            .iter()
            .map(|p| PatternRef {
                id: p.id.clone(),
                title: p.title.clone(),
                category: p.category.clone(),
                score: p.relevance_score,
            })
            .collect();

        // Determine framework string
        let framework = project
            .metadata
            .target_framework
            .clone()
            .or_else(|| project.metadata.rust_edition.clone())
            .or_else(|| project.metadata.node_version.clone())
            .or_else(|| project.metadata.python_version.clone());

        let now = Utc::now();

        Self {
            version: 1,
            name: project.name.clone(),
            project_type: project.project_type.as_str().to_string(),
            framework,
            dependencies,
            stats,
            matched_patterns,
            created_at: now,
            updated_at: now,
        }
    }

    /// Save `.rustscp` to the given project directory.
    /// Preserves `created_at` from existing file if present.
    pub fn save(&mut self, project_dir: &Path) -> Result<PathBuf> {
        let file_path = project_dir.join(RUSTSCP_FILENAME);

        // Preserve created_at from existing file
        if let Some(existing) = Self::load(project_dir)? {
            self.created_at = existing.created_at;
        }
        self.updated_at = Utc::now();

        let json = serde_json::to_string_pretty(self)
            .context("Failed to serialize ProjectContext")?;
        std::fs::write(&file_path, &json)
            .with_context(|| format!("Failed to write {}", file_path.display()))?;

        tracing::info!(path = %file_path.display(), "Saved .rustscp");
        Ok(file_path)
    }

    /// Load `.rustscp` from a project directory. Returns `None` if not found.
    pub fn load(project_dir: &Path) -> Result<Option<Self>> {
        let file_path = project_dir.join(RUSTSCP_FILENAME);
        if !file_path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read {}", file_path.display()))?;
        let ctx: Self = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse {}", file_path.display()))?;

        Ok(Some(ctx))
    }

    /// Format context as Markdown for Claude / human consumption.
    pub fn format_for_claude(&self) -> String {
        let mut out = String::with_capacity(2048);

        out.push_str(&format!("# Project: {}\n\n", self.name));
        out.push_str(&format!(
            "**Type:** {} | **Framework:** {}\n",
            self.project_type,
            self.framework.as_deref().unwrap_or("n/a")
        ));
        out.push_str(&format!(
            "**Files:** {} total",
            self.stats.total_files
        ));

        // File breakdown
        if !self.stats.by_extension.is_empty() {
            let mut exts: Vec<_> = self.stats.by_extension.iter().collect();
            exts.sort_by(|a, b| b.1.cmp(a.1));
            let breakdown: Vec<String> = exts
                .iter()
                .map(|(ext, count)| format!(".{}: {}", ext, count))
                .collect();
            out.push_str(&format!(" ({})\n", breakdown.join(", ")));
        } else {
            out.push('\n');
        }

        // Dependencies
        let prod_deps: Vec<_> = self.dependencies.iter().filter(|d| !d.dev).collect();
        if !prod_deps.is_empty() {
            out.push_str(&format!("\n## Dependencies ({})\n\n", prod_deps.len()));
            for dep in &prod_deps {
                out.push_str(&format!("- {} ({})\n", dep.name, dep.version));
            }
        }

        // Matched patterns
        if !self.matched_patterns.is_empty() {
            out.push_str(&format!(
                "\n## Matched Patterns ({})\n\n",
                self.matched_patterns.len()
            ));
            for p in &self.matched_patterns {
                out.push_str(&format!(
                    "- **{}** [{}] (score: {:.2})\n",
                    p.title, p.category, p.score
                ));
            }
        }

        out.push_str(&format!(
            "\n---\n_Last updated: {}_\n",
            self.updated_at.format("%Y-%m-%d %H:%M UTC")
        ));

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_serialize() {
        let ctx = ProjectContext {
            version: 1,
            name: "TestProject".to_string(),
            project_type: "dotnet".to_string(),
            framework: Some("net10.0".to_string()),
            dependencies: vec![DepSummary {
                name: "Dapper".to_string(),
                version: "2.1.66".to_string(),
                dev: false,
            }],
            stats: FileStats {
                total_files: 100,
                by_extension: [("cs".to_string(), 60), ("razor".to_string(), 40)]
                    .into_iter()
                    .collect(),
            },
            matched_patterns: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&ctx).unwrap();
        let parsed: ProjectContext = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "TestProject");
        assert_eq!(parsed.stats.total_files, 100);
    }
}
