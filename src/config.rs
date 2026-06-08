use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub analyzer: AnalyzerConfig,
    pub training: TrainingConfig,
    pub storage: StorageConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub name: String,
    pub version: String,
    pub transport: String, // "stdio"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzerConfig {
    pub target_frameworks: Vec<String>,
    pub ignore_patterns: Vec<String>,
    pub max_file_size_mb: usize,
    pub analyze_dependencies: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    pub enabled: bool,
    pub auto_extract_patterns: bool,
    pub min_pattern_occurrences: usize,
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub base_path: PathBuf,
    pub patterns_file: String,
    pub cache_dir: String,
    #[serde(default = "default_memories_dir")]
    pub memories_dir: String,
}

fn default_memories_dir() -> String {
    "memories".to_string()
}

impl Default for Config {
    fn default() -> Self {
        // Try to get patterns path from environment variable first
        let base_path = std::env::var("MCP_PATTERNS_PATH")
            .ok()
            .map(PathBuf::from)
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .or_else(|| {
                // Fallback: get the directory where the executable is located
                std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                    .map(|exe_path| exe_path.join("..").join("..").join("..").join("data"))
            })
            .unwrap_or_else(|| PathBuf::from("data"));

        Self {
            server: ServerConfig {
                name: "mcp-rust-context".to_string(),
                version: "0.1.0".to_string(),
                transport: "stdio".to_string(),
            },
            analyzer: AnalyzerConfig {
                target_frameworks: vec![
                    "net10.0".to_string(),
                    "net9.0".to_string(),
                    "net8.0".to_string(),
                ],
                ignore_patterns: vec![
                    "bin/**".to_string(),
                    "obj/**".to_string(),
                    "node_modules/**".to_string(),
                    ".git/**".to_string(),
                ],
                max_file_size_mb: 10,
                analyze_dependencies: true,
            },
            training: TrainingConfig {
                enabled: true,
                auto_extract_patterns: true,
                min_pattern_occurrences: 3,
                categories: vec![
                    "patterns".to_string(),
                    "best-practices".to_string(),
                    "performance".to_string(),
                    "security".to_string(),
                    "blazor-server".to_string(),
                ],
            },
            storage: StorageConfig {
                base_path,
                patterns_file: "patterns".to_string(), // Directory name, not file
                cache_dir: "cache".to_string(),
                memories_dir: "memories".to_string(),
            },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        // Try to load from config file, otherwise use defaults
        let config_path =
            dirs::config_dir().map(|p| p.join("mcp-rust-context").join("config.toml"));

        if let Some(path) = config_path {
            if path.exists() {
                let content = std::fs::read_to_string(&path)?;
                return Ok(toml::from_str(&content)?);
            }
        }

        Ok(Self::default())
    }
}
