//! Tests to validate the Endless Mode token reduction claim (~95%).
//!
//! Compares `build_generic_context_string` (full output) vs
//! `build_compact_context_string` (endless mode output) across
//! multiple realistic scenarios.

use chrono::Utc;
use mcp_context_rust::context::ContextBuilder;
use mcp_context_rust::types::{
    AnalysisResult, CodePattern, Dependency, Project, ProjectMetadata, ProjectType, SeverityLevel,
    SourceFile, Statistics, Suggestion, Symbol, SymbolKind,
};
use std::path::PathBuf;

/// Minimum acceptable token reduction for the ~95% claim.
/// We use 90% as a conservative threshold — anything above this
/// validates the claim; below it means the compact format needs work.
const MIN_REDUCTION_PERCENT: f64 = 90.0;

// ---------------------------------------------------------------------------
// Helper: build a realistic project fixture
// ---------------------------------------------------------------------------

fn make_dependency(name: &str, version: &str, dev: bool) -> Dependency {
    Dependency {
        name: name.to_string(),
        version: version.to_string(),
        dev_only: dev,
    }
}

fn make_source_file(path: &str, lang: &str, symbols: Vec<Symbol>) -> SourceFile {
    SourceFile {
        path: PathBuf::from(path),
        language: lang.to_string(),
        size_bytes: 1024,
        symbols,
    }
}

fn make_symbol(name: &str, kind: SymbolKind) -> Symbol {
    Symbol {
        name: name.to_string(),
        kind,
        modifiers: vec!["public".to_string()],
        children: vec![],
    }
}

fn make_pattern(id: &str, title: &str, category: &str, framework: &str) -> CodePattern {
    let now = Utc::now();
    CodePattern {
        id: id.to_string(),
        category: category.to_string(),
        framework: framework.to_string(),
        version: "10.0".to_string(),
        title: title.to_string(),
        description: format!("Best practice pattern for {} in {}", category, framework),
        code: "public class Example\n{\n    // sample code\n}".to_string(),
        tags: vec![category.to_string(), framework.to_string()],
        usage_count: 5,
        relevance_score: 0.85,
        created_at: now,
        updated_at: now,
    }
}

fn make_suggestion(severity: SeverityLevel, category: &str, message: &str) -> Suggestion {
    Suggestion {
        severity,
        category: category.to_string(),
        message: message.to_string(),
        file: Some(PathBuf::from("src/Example.cs")),
        line: Some(42),
    }
}

/// Calculates reduction percentage and prints diagnostics.
fn assert_reduction(label: &str, full: &str, compact: &str) {
    let full_len = full.len();
    let compact_len = compact.len();
    let reduction = (1.0 - (compact_len as f64 / full_len as f64)) * 100.0;

    println!("\n=== {} ===", label);
    println!("  Full output:    {} chars", full_len);
    println!("  Compact output: {} chars", compact_len);
    println!("  Reduction:      {:.1}%", reduction);

    assert!(
        reduction >= MIN_REDUCTION_PERCENT,
        "[{}] Expected ≥{:.0}% reduction, got {:.1}% (full={}, compact={})",
        label,
        MIN_REDUCTION_PERCENT,
        reduction,
        full_len,
        compact_len
    );
}

// ---------------------------------------------------------------------------
// Scenario 1: Minimal project (few deps, no patterns, no suggestions)
// ---------------------------------------------------------------------------
#[test]
fn test_endless_reduction_minimal_project() {
    let analysis = AnalysisResult {
        project: Project {
            path: PathBuf::from("/tmp/my-project"),
            name: "my-project".to_string(),
            project_type: ProjectType::Rust,
            version: Some("0.1.0".to_string()),
            dependencies: vec![
                make_dependency("tokio", "1.38", false),
                make_dependency("serde", "1.0", false),
            ],
            files: vec![
                make_source_file(
                    "src/main.rs",
                    "rs",
                    vec![make_symbol("main", SymbolKind::Function)],
                ),
                make_source_file(
                    "src/lib.rs",
                    "rs",
                    vec![make_symbol("Config", SymbolKind::Struct)],
                ),
            ],
            metadata: ProjectMetadata {
                rust_edition: Some("2021".to_string()),
                entry_point: Some("src/main.rs".to_string()),
                ..Default::default()
            },
        },
        patterns: vec![],
        suggestions: vec![],
        statistics: Statistics {
            total_files: 2,
            total_classes: 2,
            total_methods: 0,
            total_lines: 150,
            framework_version: "2021".to_string(),
            package_count: 2,
        },
    };

    let builder = ContextBuilder::new();
    let full = builder.build_generic_context_string(&analysis);
    let compact = builder.build_compact_context_string(&analysis);

    assert_reduction("Minimal Rust project", &full, &compact);
}

// ---------------------------------------------------------------------------
// Scenario 2: Medium .NET project with patterns and suggestions
// ---------------------------------------------------------------------------
#[test]
fn test_endless_reduction_medium_dotnet_project() {
    let deps: Vec<Dependency> = vec![
        make_dependency("Microsoft.AspNetCore.Components", "10.0.0", false),
        make_dependency("Microsoft.EntityFrameworkCore", "10.0.0", false),
        make_dependency("Microsoft.EntityFrameworkCore.SqlServer", "10.0.0", false),
        make_dependency("Dapper", "2.1.35", false),
        make_dependency("Serilog", "4.0.0", false),
        make_dependency("AutoMapper", "13.0.1", false),
        make_dependency("FluentValidation", "11.9.0", false),
        make_dependency("xunit", "2.7.0", true),
        make_dependency("Moq", "4.20.0", true),
    ];

    let files: Vec<SourceFile> = (0..25)
        .map(|i| {
            make_source_file(
                &format!("src/Components/Component{}.razor.cs", i),
                "cs",
                vec![
                    make_symbol(&format!("Component{}", i), SymbolKind::Class),
                    make_symbol("OnInitializedAsync", SymbolKind::Method),
                ],
            )
        })
        .collect();

    let patterns = vec![
        make_pattern(
            "blazor-lifecycle-001",
            "Blazor Lifecycle Best Practices",
            "lifecycle",
            "blazor-server",
        ),
        make_pattern(
            "ef-query-001",
            "EF Core Query Optimization",
            "data-access",
            "blazor-server",
        ),
        make_pattern(
            "di-scoped-001",
            "Scoped Service Registration",
            "dependency-injection",
            "blazor-server",
        ),
    ];

    let suggestions = vec![
        make_suggestion(
            SeverityLevel::Warning,
            "blazor-lifecycle",
            "Component5 uses synchronous OnInitialized()",
        ),
        make_suggestion(
            SeverityLevel::Info,
            "architecture",
            "Large project with 25 files. Consider modular organization.",
        ),
        make_suggestion(
            SeverityLevel::Error,
            "security",
            "Potential SQL injection in raw query",
        ),
    ];

    let analysis = AnalysisResult {
        project: Project {
            path: PathBuf::from("/projects/crm"),
            name: "CRM-Blazor".to_string(),
            project_type: ProjectType::DotNet,
            version: Some("2.5.0".to_string()),
            dependencies: deps,
            files,
            metadata: ProjectMetadata {
                target_framework: Some("net10.0".to_string()),
                entry_point: Some("Program.cs".to_string()),
                ..Default::default()
            },
        },
        patterns,
        suggestions,
        statistics: Statistics {
            total_files: 25,
            total_classes: 50,
            total_methods: 200,
            total_lines: 5000,
            framework_version: "net10.0".to_string(),
            package_count: 9,
        },
    };

    let builder = ContextBuilder::new();
    let full = builder.build_generic_context_string(&analysis);
    let compact = builder.build_compact_context_string(&analysis);

    assert_reduction("Medium .NET/Blazor project", &full, &compact);
}

// ---------------------------------------------------------------------------
// Scenario 3: Large Node.js project (many deps, many files)
// ---------------------------------------------------------------------------
#[test]
fn test_endless_reduction_large_node_project() {
    let deps: Vec<Dependency> = (0..40)
        .map(|i| make_dependency(&format!("package-{}", i), &format!("{}.0.0", i), i >= 30))
        .collect();

    let files: Vec<SourceFile> = (0..80)
        .map(|i| {
            make_source_file(
                &format!("src/modules/module{}/index.ts", i),
                "ts",
                vec![
                    make_symbol(&format!("Module{}", i), SymbolKind::Class),
                    make_symbol(&format!("handler{}", i), SymbolKind::Function),
                ],
            )
        })
        .collect();

    let patterns = vec![
        make_pattern(
            "express-middleware-001",
            "Express Middleware Pattern",
            "middleware",
            "express",
        ),
        make_pattern(
            "express-error-001",
            "Express Error Handling",
            "error-handling",
            "express",
        ),
        make_pattern(
            "express-auth-001",
            "JWT Authentication Middleware",
            "authentication",
            "express",
        ),
        make_pattern(
            "express-validation-001",
            "Request Validation with Zod",
            "validation",
            "express",
        ),
        make_pattern(
            "express-testing-001",
            "Integration Testing with Supertest",
            "testing",
            "express",
        ),
    ];

    let suggestions = vec![
        make_suggestion(
            SeverityLevel::Warning,
            "security",
            "Express 3.x is outdated",
        ),
        make_suggestion(
            SeverityLevel::Info,
            "architecture",
            "Large project with 80 files",
        ),
        make_suggestion(
            SeverityLevel::Info,
            "patterns",
            "No patterns found for framework 'express'",
        ),
    ];

    let analysis = AnalysisResult {
        project: Project {
            path: PathBuf::from("/projects/api-server"),
            name: "api-server".to_string(),
            project_type: ProjectType::Node,
            version: Some("3.1.0".to_string()),
            dependencies: deps,
            files,
            metadata: ProjectMetadata {
                node_version: Some("20.11.0".to_string()),
                entry_point: Some("src/index.ts".to_string()),
                ..Default::default()
            },
        },
        patterns,
        suggestions,
        statistics: Statistics {
            total_files: 80,
            total_classes: 80,
            total_methods: 160,
            total_lines: 15000,
            framework_version: "20.11.0".to_string(),
            package_count: 40,
        },
    };

    let builder = ContextBuilder::new();
    let full = builder.build_generic_context_string(&analysis);
    let compact = builder.build_compact_context_string(&analysis);

    assert_reduction("Large Node.js project", &full, &compact);
}

// ---------------------------------------------------------------------------
// Summary: prints all three scenarios side by side
// ---------------------------------------------------------------------------
#[test]
fn test_endless_reduction_summary() {
    // This test just verifies the compact output stays within a sane range
    // (the other tests validate individual reductions).

    let builder = ContextBuilder::new();

    // Quick minimal analysis
    let analysis = AnalysisResult {
        project: Project {
            path: PathBuf::from("/tmp/test"),
            name: "test-project".to_string(),
            project_type: ProjectType::Rust,
            version: Some("1.0.0".to_string()),
            dependencies: vec![make_dependency("serde", "1.0", false)],
            files: vec![make_source_file("src/main.rs", "rs", vec![])],
            metadata: ProjectMetadata {
                rust_edition: Some("2021".to_string()),
                entry_point: Some("src/main.rs".to_string()),
                ..Default::default()
            },
        },
        patterns: vec![],
        suggestions: vec![],
        statistics: Statistics {
            total_files: 1,
            total_classes: 0,
            total_methods: 0,
            total_lines: 50,
            framework_version: "2021".to_string(),
            package_count: 1,
        },
    };

    let compact = builder.build_compact_context_string(&analysis);

    // The compact output should be roughly 100-200 chars as documented
    println!("\n=== Compact output length check ===");
    println!("  Length: {} chars", compact.len());
    println!("  Content: {}", compact);

    assert!(
        compact.len() <= 500,
        "Compact output should be concise, got {} chars",
        compact.len()
    );
}
