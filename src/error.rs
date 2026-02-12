//! Custom error types for the MCP Context Server.
//!
//! This module provides typed errors using `thiserror` for better
//! error handling and propagation throughout the application.

use std::path::PathBuf;
use thiserror::Error;

/// Main error type for the MCP Context Server.
#[derive(Error, Debug)]
pub enum McpError {
    /// Error during project analysis
    #[error("Analysis error: {0}")]
    Analysis(#[from] AnalysisError),

    /// Error during pattern training/management
    #[error("Training error: {0}")]
    Training(#[from] TrainingError),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Generic error (fallback)
    #[error("{0}")]
    Other(String),
}

/// Errors that can occur during project analysis.
#[derive(Error, Debug)]
pub enum AnalysisError {
    /// Project path does not exist
    #[error("Project path does not exist: {0}")]
    PathNotFound(PathBuf),

    /// Path is not a directory
    #[error("Path is not a directory: {0}")]
    NotADirectory(PathBuf),

    /// No project file found (Cargo.toml, package.json, etc.)
    #[error("No project file found in {path}. Expected one of: {expected}")]
    NoProjectFile { path: PathBuf, expected: String },

    /// Failed to parse project file
    #[error("Failed to parse {file_type} file at {path}: {reason}")]
    ParseError {
        file_type: String,
        path: PathBuf,
        reason: String,
    },

    /// File read error
    #[error("Failed to read file {path}: {reason}")]
    FileReadError { path: PathBuf, reason: String },

    /// Unsupported project type
    #[error("Unsupported project type: {0}")]
    UnsupportedType(String),
}

/// Errors that can occur during pattern training/management.
#[derive(Error, Debug)]
pub enum TrainingError {
    /// Invalid framework name (security validation failed)
    #[error("Invalid framework name '{name}': {reason}")]
    InvalidFrameworkName { name: String, reason: String },

    /// Invalid pattern ID
    #[error("Invalid pattern ID '{id}': {reason}")]
    InvalidPatternId { id: String, reason: String },

    /// Invalid category name
    #[error("Invalid category name '{name}': {reason}")]
    InvalidCategory { name: String, reason: String },

    /// Pattern not found
    #[error("Pattern not found: {0}")]
    PatternNotFound(String),

    /// Duplicate pattern ID
    #[error("Pattern with ID '{0}' already exists")]
    DuplicatePattern(String),

    /// Failed to save patterns
    #[error("Failed to save patterns to {path}: {reason}")]
    SaveError { path: PathBuf, reason: String },

    /// Failed to load patterns
    #[error("Failed to load patterns from {path}: {reason}")]
    LoadError { path: PathBuf, reason: String },

    /// Path traversal attempt detected
    #[error("Security error: Path traversal attempt detected for '{0}'")]
    PathTraversal(String),
}

/// Configuration errors.
#[derive(Error, Debug)]
pub enum ConfigError {
    /// Config file not found
    #[error("Configuration file not found: {0}")]
    FileNotFound(PathBuf),

    /// Invalid configuration format
    #[error("Invalid configuration format: {0}")]
    InvalidFormat(String),

    /// Missing required field
    #[error("Missing required configuration field: {0}")]
    MissingField(String),

    /// Invalid value for field
    #[error("Invalid value for '{field}': {reason}")]
    InvalidValue { field: String, reason: String },
}

/// Result type alias using our custom error.
pub type Result<T> = std::result::Result<T, McpError>;

/// Result type alias for analysis operations.
pub type AnalysisResult<T> = std::result::Result<T, AnalysisError>;

/// Result type alias for training operations.
pub type TrainingResult<T> = std::result::Result<T, TrainingError>;

impl From<String> for McpError {
    fn from(s: String) -> Self {
        Self::Other(s)
    }
}

impl From<&str> for McpError {
    fn from(s: &str) -> Self {
        Self::Other(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analysis_error_display() {
        let err = AnalysisError::PathNotFound(PathBuf::from("/test/path"));
        assert!(err.to_string().contains("/test/path"));
    }

    #[test]
    fn test_training_error_display() {
        let err = TrainingError::InvalidFrameworkName {
            name: "test..name".to_string(),
            reason: "contains ..".to_string(),
        };
        assert!(err.to_string().contains("test..name"));
    }

    #[test]
    fn test_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let mcp_err: McpError = io_err.into();
        assert!(matches!(mcp_err, McpError::Io(_)));
    }
}
