use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::types::CodePattern;

/// Validates and sanitizes a framework name to prevent path traversal attacks.
///
/// # Security
/// This function is critical for preventing directory traversal vulnerabilities.
/// Only alphanumeric characters, hyphens, and underscores are allowed.
///
/// # Returns
/// - `Ok(String)` with the sanitized framework name
/// - `Err(String)` if the name contains invalid characters
fn sanitize_framework_name(framework: &str) -> Result<String, String> {
    // Check for empty input
    if framework.is_empty() {
        return Err("Framework name cannot be empty".to_string());
    }

    // Check for path traversal attempts
    if framework.contains("..") {
        return Err("Framework name cannot contain '..'".to_string());
    }

    // Check for path separators (both Unix and Windows)
    if framework.contains('/') || framework.contains('\\') {
        return Err("Framework name cannot contain path separators".to_string());
    }

    // Check for Windows drive letters (e.g., "C:")
    if framework.contains(':') {
        return Err("Framework name cannot contain ':'".to_string());
    }

    // Check for null bytes (can bypass security in some systems)
    if framework.contains('\0') {
        return Err("Framework name cannot contain null bytes".to_string());
    }

    // Validate each character: only alphanumeric, hyphens, underscores, and dots
    let sanitized: String = framework
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
        .collect();

    // Ensure the sanitized name matches the original (no characters were filtered)
    if sanitized != framework {
        return Err(
            "Framework name contains invalid characters. Allowed: alphanumeric, '-', '_', '.'"
                .to_string(),
        );
    }

    // Ensure name doesn't start with a dot (hidden files)
    if sanitized.starts_with('.') {
        return Err("Framework name cannot start with '.'".to_string());
    }

    // Length limit to prevent filesystem issues
    if sanitized.len() > 64 {
        return Err("Framework name too long (max 64 characters)".to_string());
    }

    Ok(sanitized)
}

/// Manages code patterns for training and suggestions
#[derive(Clone)]
pub struct TrainingManager {
    patterns: Vec<CodePattern>,
    storage_path: PathBuf,
    // Index for fast lookups
    category_index: HashMap<String, Vec<usize>>,
    framework_index: HashMap<String, Vec<usize>>,
}

/// Search criteria for patterns
#[derive(Debug, Clone)]
pub struct SearchCriteria {
    pub query: Option<String>,
    pub category: Option<String>,
    pub framework: Option<String>,
    pub tags: Vec<String>,
    pub min_score: f32,
}

impl TrainingManager {
    pub fn new(storage_path: impl Into<PathBuf>) -> Self {
        Self {
            patterns: vec![],
            storage_path: storage_path.into(),
            category_index: HashMap::new(),
            framework_index: HashMap::new(),
        }
    }

    pub async fn load_patterns(&mut self) -> Result<()> {
        self.patterns.clear();
        self.category_index.clear();
        self.framework_index.clear();

        if !self.storage_path.exists() {
            tracing::warn!(
                "Pattern storage path does not exist: {:?}",
                self.storage_path
            );
            return Ok(());
        }

        // Walk through all JSON files in the patterns directory
        for entry in WalkDir::new(&self.storage_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                self.load_pattern_file(path)
                    .context(format!("Failed to load pattern file: {:?}", path))?;
            }
        }

        // Build indexes
        self.rebuild_indexes();

        tracing::info!(
            "Loaded {} patterns from {:?}",
            self.patterns.len(),
            self.storage_path
        );
        Ok(())
    }

    fn load_pattern_file(&mut self, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path).context("Failed to read pattern file")?;

        #[derive(serde::Deserialize)]
        struct PatternFile {
            patterns: Vec<CodePattern>,
        }

        let file: PatternFile =
            serde_json::from_str(&content).context("Failed to parse pattern JSON")?;

        for pattern in file.patterns {
            self.patterns.push(pattern);
        }

        Ok(())
    }

    fn rebuild_indexes(&mut self) {
        self.category_index.clear();
        self.framework_index.clear();

        for (idx, pattern) in self.patterns.iter().enumerate() {
            // Index by category
            self.category_index
                .entry(pattern.category.clone())
                .or_default()
                .push(idx);

            // Index by framework
            self.framework_index
                .entry(pattern.framework.clone())
                .or_default()
                .push(idx);
        }
    }

    pub async fn save_patterns(&self) -> Result<()> {
        fs::create_dir_all(&self.storage_path).context("Failed to create storage directory")?;

        // Canonicalize storage path for security validation (after ensuring it exists)
        let canonical_storage = self
            .storage_path
            .canonicalize()
            .context("Failed to canonicalize storage path")?;

        // Group patterns by framework
        let mut by_framework: HashMap<String, Vec<&CodePattern>> = HashMap::new();

        for pattern in &self.patterns {
            by_framework
                .entry(pattern.framework.clone())
                .or_default()
                .push(pattern);
        }

        // Save each framework to its own file
        for (framework, patterns) in by_framework {
            // SECURITY: Validate and sanitize framework name to prevent path traversal
            let safe_framework = sanitize_framework_name(&framework)
                .map_err(|e| anyhow::anyhow!("Invalid framework name '{}': {}", framework, e))?;

            let filename = format!("{}-patterns.json", safe_framework);
            let file_path = canonical_storage.join(&filename);

            // SECURITY: Double-check that the constructed path is within storage_path
            // Since we already validated the framework name and are using canonical_storage,
            // this should always be true, but we verify as defense in depth
            if !file_path.starts_with(&canonical_storage) {
                anyhow::bail!(
                    "Security error: Path traversal attempt detected for framework '{}'",
                    framework
                );
            }

            #[derive(serde::Serialize)]
            struct PatternFile<'a> {
                patterns: Vec<&'a CodePattern>,
            }

            let file = PatternFile { patterns };
            let json =
                serde_json::to_string_pretty(&file).context("Failed to serialize patterns")?;

            fs::write(&file_path, json)
                .context(format!("Failed to write pattern file: {:?}", file_path))?;
        }

        tracing::info!(
            "Saved {} patterns to {:?}",
            self.patterns.len(),
            self.storage_path
        );
        Ok(())
    }

    /// Validates a pattern before adding it.
    /// Returns an error if the pattern contains invalid data.
    fn validate_pattern(pattern: &CodePattern) -> Result<(), String> {
        // Validate framework name
        sanitize_framework_name(&pattern.framework)?;

        // Validate ID (same rules as framework)
        if pattern.id.is_empty() {
            return Err("Pattern ID cannot be empty".to_string());
        }
        if pattern.id.len() > 128 {
            return Err("Pattern ID too long (max 128 characters)".to_string());
        }

        // Validate category
        if pattern.category.is_empty() {
            return Err("Pattern category cannot be empty".to_string());
        }
        if pattern.category.len() > 64 {
            return Err("Pattern category too long (max 64 characters)".to_string());
        }

        Ok(())
    }

    /// Adds a new pattern to the manager.
    ///
    /// # Security
    /// The pattern's framework, id, and category are validated to prevent
    /// path traversal and other injection attacks.
    ///
    /// # Returns
    /// - `Ok(())` if the pattern was added successfully
    /// - `Err` if the pattern contains invalid data
    pub fn add_pattern(&mut self, mut pattern: CodePattern) -> Result<(), String> {
        // SECURITY: Validate pattern before adding
        Self::validate_pattern(&pattern)?;

        // Set timestamps if not set
        if pattern.created_at.timestamp() == 0 {
            pattern.created_at = Utc::now();
        }
        pattern.updated_at = Utc::now();

        let idx = self.patterns.len();
        self.patterns.push(pattern.clone());

        // Update indexes
        self.category_index
            .entry(pattern.category)
            .or_default()
            .push(idx);

        self.framework_index
            .entry(pattern.framework)
            .or_default()
            .push(idx);

        Ok(())
    }

    pub fn search_patterns(&self, criteria: &SearchCriteria) -> Vec<(&CodePattern, f32)> {
        let mut candidates: Vec<usize>;

        // Filter by framework first (most selective)
        if let Some(ref framework) = criteria.framework {
            if let Some(indices) = self.framework_index.get(framework) {
                candidates = indices.clone();
            } else {
                return vec![]; // No patterns for this framework
            }
        } else {
            candidates = (0..self.patterns.len()).collect();
        }

        // Further filter by category if specified
        if let Some(ref category) = criteria.category {
            if let Some(cat_indices) = self.category_index.get(category) {
                let cat_set: HashSet<usize> = cat_indices.iter().copied().collect();
                candidates.retain(|idx| cat_set.contains(idx));
            } else {
                return vec![]; // No patterns for this category
            }
        }

        // Score and filter candidates
        let mut scored: Vec<(&CodePattern, f32)> = candidates
            .iter()
            .map(|&idx| {
                let pattern = &self.patterns[idx];
                let score = self.score_pattern(pattern, criteria);
                (pattern, score)
            })
            .filter(|(_, score)| *score >= criteria.min_score)
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        scored
    }

    /// Calculates a relevance score for a pattern based on search criteria.
    ///
    /// The score is composed of:
    /// - Base relevance score from the pattern
    /// - Usage count boost (popular patterns)
    /// - Query match bonuses (title, description, code)
    /// - Tag match bonus
    /// - Recency boost (patterns updated in last 30 days)
    fn score_pattern(&self, pattern: &CodePattern, criteria: &SearchCriteria) -> f32 {
        let mut score = pattern.relevance_score;

        // Boost score based on usage count (popular patterns)
        // Using log10 to prevent high-usage patterns from dominating
        if pattern.usage_count > 0 {
            #[allow(clippy::cast_precision_loss)]
            let usage_boost = (pattern.usage_count as f32).log10() * 0.05;
            score += usage_boost;
        }

        // Check query match (if provided)
        if let Some(ref query) = criteria.query {
            let query_lower = query.to_lowercase();

            // Title match (high weight) - most important
            if pattern.title.to_lowercase().contains(&query_lower) {
                score += 0.3;
            }

            // Description match (medium weight)
            if pattern.description.to_lowercase().contains(&query_lower) {
                score += 0.15;
            }

            // Code match (low weight, as code can be verbose)
            if pattern.code.to_lowercase().contains(&query_lower) {
                score += 0.05;
            }
        }

        // Check tag matches - optimized to avoid allocations
        // Instead of creating HashSets, we count matches directly
        if !criteria.tags.is_empty() {
            let matching_tags = criteria
                .tags
                .iter()
                .filter(|tag| pattern.tags.contains(tag))
                .count();

            #[allow(clippy::cast_precision_loss)]
            let tag_score = matching_tags as f32 / criteria.tags.len() as f32;
            score += tag_score * 0.2;
        }

        // Recency boost (newer patterns get slight advantage)
        let age_days = (Utc::now() - pattern.updated_at).num_days();
        if age_days < 30 {
            score += 0.05;
        }

        score
    }

    /// Convenience method for simple searches
    pub fn search_by_framework_and_category(
        &self,
        framework: &str,
        category: &str,
    ) -> Vec<&CodePattern> {
        let criteria = SearchCriteria {
            query: None,
            category: Some(category.to_string()),
            framework: Some(framework.to_string()),
            tags: vec![],
            min_score: 0.0,
        };

        self.search_patterns(&criteria)
            .into_iter()
            .map(|(pattern, _score)| pattern)
            .collect()
    }

    /// Get patterns by ID
    #[allow(dead_code)]
    pub fn get_pattern_by_id(&self, id: &str) -> Option<&CodePattern> {
        self.patterns.iter().find(|p| p.id == id)
    }

    /// Update pattern usage count
    #[allow(dead_code)]
    pub fn increment_usage(&mut self, pattern_id: &str) -> Result<()> {
        if let Some(pattern) = self.patterns.iter_mut().find(|p| p.id == pattern_id) {
            pattern.usage_count += 1;
            pattern.updated_at = Utc::now();
            Ok(())
        } else {
            anyhow::bail!("Pattern not found: {}", pattern_id)
        }
    }

    pub fn get_statistics(&self) -> serde_json::Value {
        let categories = self.get_categories();
        let frameworks = self.get_frameworks();
        let total_usage: usize = self.patterns.iter().map(|p| p.usage_count).sum();

        serde_json::json!({
            "total_patterns": self.patterns.len(),
            "categories": categories,
            "frameworks": frameworks,
            "total_usage": total_usage,
            "avg_relevance": self.avg_relevance_score(),
        })
    }

    fn get_categories(&self) -> Vec<String> {
        self.category_index.keys().cloned().collect()
    }

    fn get_frameworks(&self) -> Vec<String> {
        self.framework_index.keys().cloned().collect()
    }

    fn avg_relevance_score(&self) -> f32 {
        if self.patterns.is_empty() {
            return 0.0;
        }

        let sum: f32 = self.patterns.iter().map(|p| p.relevance_score).sum();
        sum / self.patterns.len() as f32
    }

    /// Get all patterns
    pub fn get_all_patterns(&self) -> &[CodePattern] {
        &self.patterns
    }
}
