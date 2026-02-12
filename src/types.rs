use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ============================================================================
// Generic Multi-Language Project Types
// ============================================================================

/// Detected project type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProjectType {
    DotNet, // .csproj, .sln
    Rust,   // Cargo.toml
    Node,   // package.json
    Python, // pyproject.toml, setup.py, requirements.txt
    Go,     // go.mod
    Java,   // pom.xml, build.gradle
    Php,    // composer.json
    Unknown,
}

impl ProjectType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectType::DotNet => "dotnet",
            ProjectType::Rust => "rust",
            ProjectType::Node => "node",
            ProjectType::Python => "python",
            ProjectType::Go => "go",
            ProjectType::Java => "java",
            ProjectType::Php => "php",
            ProjectType::Unknown => "unknown",
        }
    }
}

/// Generic project representation (works for any language)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub path: PathBuf,
    pub name: String,
    pub project_type: ProjectType,
    pub version: Option<String>,
    pub dependencies: Vec<Dependency>,
    pub files: Vec<SourceFile>,
    /// Language-specific metadata
    pub metadata: ProjectMetadata,
}

/// Generic dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub dev_only: bool,
}

/// Generic source file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFile {
    pub path: PathBuf,
    pub language: String,
    pub size_bytes: u64,
    /// Extracted symbols (classes, functions, etc.)
    pub symbols: Vec<Symbol>,
}

/// Generic symbol (class, function, interface, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub modifiers: Vec<String>,
    pub children: Vec<Symbol>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    Trait,     // Rust
    Impl,      // Rust
    Component, // React/Vue/Blazor
    Other(String),
}

/// Language-specific metadata
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

/// Represents a .NET project
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

/// NuGet package reference
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NuGetPackage {
    pub name: String,
    pub version: String,
}

/// C# source file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CSharpFile {
    pub path: PathBuf,
    pub namespace: Option<String>,
    pub usings: Vec<String>,
    pub classes: Vec<ClassInfo>,
    pub interfaces: Vec<InterfaceInfo>,
}

/// Class information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInfo {
    pub name: String,
    pub modifiers: Vec<String>,
    pub base_class: Option<String>,
    pub interfaces: Vec<String>,
    pub methods: Vec<MethodInfo>,
    pub properties: Vec<PropertyInfo>,
}

/// Interface information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceInfo {
    pub name: String,
    pub methods: Vec<MethodInfo>,
}

/// Method information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodInfo {
    pub name: String,
    pub return_type: String,
    pub parameters: Vec<Parameter>,
    pub modifiers: Vec<String>,
    pub is_async: bool,
}

/// Parameter information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub param_type: String,
}

/// Property information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyInfo {
    pub name: String,
    pub prop_type: String,
    pub has_getter: bool,
    pub has_setter: bool,
}

/// Code pattern for training
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodePattern {
    pub id: String,
    pub category: String,
    pub framework: String,
    pub version: String,
    pub title: String,
    pub description: String,
    pub code: String,
    pub tags: Vec<String>,
    pub usage_count: usize,
    pub relevance_score: f32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Analysis result (generic)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub project: Project,
    pub patterns: Vec<CodePattern>,
    pub suggestions: Vec<Suggestion>,
    pub statistics: Statistics,
}

/// Legacy analysis result for .NET (kept for compatibility)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DotNetAnalysisResult {
    pub project: DotNetProject,
    pub patterns: Vec<CodePattern>,
    pub suggestions: Vec<Suggestion>,
    pub statistics: Statistics,
}

/// Code suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub severity: SeverityLevel,
    pub category: String,
    pub message: String,
    pub file: Option<PathBuf>,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SeverityLevel {
    Info,
    Warning,
    Error,
}

/// Project statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statistics {
    pub total_files: usize,
    pub total_classes: usize,
    pub total_methods: usize,
    pub total_lines: usize,
    pub framework_version: String,
    pub package_count: usize,
}
