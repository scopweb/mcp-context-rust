//! Core types for the MCP Context Server.
//!
//! This module contains all the data structures used throughout the application,
//! including project types, patterns, and analysis results.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

// ============================================================================
// Generic Multi-Language Project Types
// ============================================================================

/// Detected project type based on configuration files.
///
/// The project type is automatically detected by looking for specific
/// configuration files in the project directory.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ProjectType {
    /// .NET projects (.csproj, .sln)
    DotNet,
    /// Rust projects (Cargo.toml)
    Rust,
    /// Node.js projects (package.json)
    Node,
    /// Python projects (pyproject.toml, setup.py, requirements.txt)
    Python,
    /// Go projects (go.mod)
    Go,
    /// Java projects (pom.xml, build.gradle)
    Java,
    /// PHP projects (composer.json)
    Php,
    /// Unknown project type
    Unknown,
}

impl ProjectType {
    /// Returns the lowercase string representation of the project type.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::DotNet => "dotnet",
            Self::Rust => "rust",
            Self::Node => "node",
            Self::Python => "python",
            Self::Go => "go",
            Self::Java => "java",
            Self::Php => "php",
            Self::Unknown => "unknown",
        }
    }
}

impl fmt::Display for ProjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Default for ProjectType {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Generic project representation that works for any language.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Path to the project root directory
    pub path: PathBuf,
    /// Project name
    pub name: String,
    /// Detected project type
    pub project_type: ProjectType,
    /// Project version (if available)
    pub version: Option<String>,
    /// Project dependencies
    pub dependencies: Vec<Dependency>,
    /// Source files in the project
    pub files: Vec<SourceFile>,
    /// Language-specific metadata
    pub metadata: ProjectMetadata,
}

/// Generic dependency representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// Package/crate name
    pub name: String,
    /// Version specifier
    pub version: String,
    /// Whether this is a development-only dependency
    pub dev_only: bool,
}

/// Generic source file representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFile {
    /// Path to the file
    pub path: PathBuf,
    /// Programming language (file extension)
    pub language: String,
    /// File size in bytes
    pub size_bytes: u64,
    /// Extracted symbols (classes, functions, etc.)
    pub symbols: Vec<Symbol>,
}

/// Generic symbol (class, function, interface, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    /// Symbol name
    pub name: String,
    /// Symbol kind
    pub kind: SymbolKind,
    /// Visibility/access modifiers
    pub modifiers: Vec<String>,
    /// Child symbols (methods, fields, etc.)
    pub children: Vec<Symbol>,
}

/// Kind of symbol in source code.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SymbolKind {
    Class,
    Interface,
    Function,
    Method,
    Property,
    Field,
    Enum,
    Struct,
    Module,
    /// Rust trait
    Trait,
    /// Rust impl block
    Impl,
    /// React/Vue/Blazor component
    Component,
    /// Other symbol type
    Other(String),
}

impl fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Class => write!(f, "class"),
            Self::Interface => write!(f, "interface"),
            Self::Function => write!(f, "function"),
            Self::Method => write!(f, "method"),
            Self::Property => write!(f, "property"),
            Self::Field => write!(f, "field"),
            Self::Enum => write!(f, "enum"),
            Self::Struct => write!(f, "struct"),
            Self::Module => write!(f, "module"),
            Self::Trait => write!(f, "trait"),
            Self::Impl => write!(f, "impl"),
            Self::Component => write!(f, "component"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

/// Language-specific metadata for projects.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectMetadata {
    /// For .NET: target framework (net8.0, etc.)
    pub target_framework: Option<String>,
    /// For Node: node version
    pub node_version: Option<String>,
    /// For Python: python version
    pub python_version: Option<String>,
    /// For Rust: edition (2021, etc.)
    pub rust_edition: Option<String>,
    /// Entry point file
    pub entry_point: Option<String>,
    /// Build command
    pub build_command: Option<String>,
    /// Additional key-value metadata
    pub extra: std::collections::HashMap<String, String>,
}

// ============================================================================
// Legacy .NET-specific types (kept for compatibility)
// ============================================================================

/// Represents a .NET project (legacy type for backwards compatibility).
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DotNetProject {
    pub path: PathBuf,
    pub name: String,
    pub target_framework: String,
    pub language_version: String,
    pub packages: Vec<NuGetPackage>,
    pub project_references: Vec<PathBuf>,
    pub files: Vec<CSharpFile>,
}

/// NuGet package reference.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NuGetPackage {
    pub name: String,
    pub version: String,
}

/// C# source file information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CSharpFile {
    pub path: PathBuf,
    pub namespace: Option<String>,
    pub usings: Vec<String>,
    pub classes: Vec<ClassInfo>,
    pub interfaces: Vec<InterfaceInfo>,
}

/// Class information from C# analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInfo {
    pub name: String,
    pub modifiers: Vec<String>,
    pub base_class: Option<String>,
    pub interfaces: Vec<String>,
    pub methods: Vec<MethodInfo>,
    pub properties: Vec<PropertyInfo>,
}

/// Interface information from C# analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceInfo {
    pub name: String,
    pub methods: Vec<MethodInfo>,
}

/// Method information from C# analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodInfo {
    pub name: String,
    pub return_type: String,
    pub parameters: Vec<Parameter>,
    pub modifiers: Vec<String>,
    pub is_async: bool,
}

/// Parameter information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub param_type: String,
}

/// Property information from C# analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyInfo {
    pub name: String,
    pub prop_type: String,
    pub has_getter: bool,
    pub has_setter: bool,
}

// ============================================================================
// Pattern and Training Types
// ============================================================================

/// Code pattern for training and suggestions.
///
/// Patterns are reusable code examples that can be searched and
/// suggested based on the project context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodePattern {
    /// Unique identifier
    pub id: String,
    /// Category (e.g., "lifecycle", "error-handling")
    pub category: String,
    /// Target framework (e.g., "blazor-server", "react")
    pub framework: String,
    /// Framework version
    pub version: String,
    /// Pattern title
    pub title: String,
    /// Pattern description
    pub description: String,
    /// Code example
    pub code: String,
    /// Tags for search
    pub tags: Vec<String>,
    /// Number of times this pattern was used
    pub usage_count: usize,
    /// Relevance score (0.0 - 1.0)
    pub relevance_score: f32,
    /// When the pattern was created
    pub created_at: DateTime<Utc>,
    /// When the pattern was last updated
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// Analysis Result Types
// ============================================================================

/// Result of analyzing a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// The analyzed project
    pub project: Project,
    /// Relevant patterns found
    pub patterns: Vec<CodePattern>,
    /// Suggestions for improvement
    pub suggestions: Vec<Suggestion>,
    /// Project statistics
    pub statistics: Statistics,
}

/// Legacy analysis result for .NET (kept for compatibility).
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DotNetAnalysisResult {
    pub project: DotNetProject,
    pub patterns: Vec<CodePattern>,
    pub suggestions: Vec<Suggestion>,
    pub statistics: Statistics,
}

/// Code suggestion with severity level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    /// Severity level of the suggestion
    pub severity: SeverityLevel,
    /// Category of the suggestion
    pub category: String,
    /// Suggestion message
    pub message: String,
    /// Related file (if applicable)
    pub file: Option<PathBuf>,
    /// Line number (if applicable)
    pub line: Option<usize>,
}

/// Severity level for suggestions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SeverityLevel {
    /// Informational suggestion
    Info,
    /// Warning that should be addressed
    Warning,
    /// Error that must be fixed
    Error,
}

impl fmt::Display for SeverityLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::Error => write!(f, "error"),
        }
    }
}

impl SeverityLevel {
    /// Returns an emoji icon for the severity level.
    ///
    /// Useful for displaying severity in terminal output or markdown.
    #[must_use]
    #[allow(dead_code)] // Utility method for future use
    pub const fn icon(&self) -> &'static str {
        match self {
            Self::Info => "ℹ️",
            Self::Warning => "⚠️",
            Self::Error => "❌",
        }
    }
}

/// Project statistics summary.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Statistics {
    /// Total number of source files
    pub total_files: usize,
    /// Total number of classes/types
    pub total_classes: usize,
    /// Total number of methods/functions
    pub total_methods: usize,
    /// Total lines of code
    pub total_lines: usize,
    /// Framework version
    pub framework_version: String,
    /// Number of dependencies
    pub package_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_type_display() {
        assert_eq!(ProjectType::Rust.to_string(), "rust");
        assert_eq!(ProjectType::Node.to_string(), "node");
        assert_eq!(ProjectType::Unknown.to_string(), "unknown");
    }

    #[test]
    fn test_severity_level_display() {
        assert_eq!(SeverityLevel::Info.to_string(), "info");
        assert_eq!(SeverityLevel::Warning.to_string(), "warning");
        assert_eq!(SeverityLevel::Error.to_string(), "error");
    }

    #[test]
    fn test_severity_level_ordering() {
        assert!(SeverityLevel::Info < SeverityLevel::Warning);
        assert!(SeverityLevel::Warning < SeverityLevel::Error);
    }

    #[test]
    fn test_symbol_kind_display() {
        assert_eq!(SymbolKind::Class.to_string(), "class");
        assert_eq!(SymbolKind::Trait.to_string(), "trait");
        assert_eq!(SymbolKind::Other("custom".to_string()).to_string(), "custom");
    }
}
