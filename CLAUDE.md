# MCP Context Rust - Project Instructions

## What is this project

A multi-language MCP (Model Context Protocol) server written in Rust that provides intelligent code analysis, pattern training, and contextual suggestions for AI assistants. Supports Rust, Node.js, Python, Go, Java, PHP, and .NET projects.

## Architecture

```
src/
  main.rs           - Entry point (MCP server + read-context subcommand)
  lib.rs            - Library root with clippy lints
  config.rs         - Configuration (Config, ServerConfig, StorageConfig)
  types.rs          - Core types (Project, CodePattern, AnalysisResult, etc.)
  error.rs          - Custom error types (McpError, TrainingError)
  utils/mod.rs      - Utility functions (hash, truncate)
  analyzer/
    mod.rs           - Analyzer module exports
    detector.rs      - ProjectDetector: detects project type from config files
    generic.rs       - GenericAnalyzer: multi-language project analysis
    project.rs       - Legacy .NET project analyzer (.csproj)
    csharp.rs        - C# tree-sitter parser
  context/mod.rs     - ContextBuilder: generates AI context with patterns & suggestions
  training/mod.rs    - TrainingManager: pattern CRUD, search with scoring, indexes
  mcp/mod.rs         - MCP Server: JSON-RPC 2.0 over stdio, 8 tools
  observations.rs    - ObservationStore: disk-backed archive for Endless Mode
  rustscp.rs         - .rustscp file generation for project context caching
data/patterns/       - Built-in pattern JSON files (grouped by framework)
tests/               - Integration tests
```

## Key commands

```bash
# Build
cargo build --release

# Run all tests
cargo test

# Run specific test module
cargo test analyzer
cargo test training

# Format check and fix
cargo fmt --check
cargo fmt

# Lint
cargo clippy --all-targets -- -D warnings

# Security audit (requires cargo-audit)
cargo audit

# Run MCP server (stdio transport)
cargo run --release

# Read cached project context
cargo run --release -- read-context /path/to/project
```

## MCP Tools (8 total)

| Tool | Purpose |
|------|---------|
| `analyze-project` | Analyze any project and return structured context |
| `get-patterns` | Get patterns by framework/category |
| `search-patterns` | Advanced multi-criteria pattern search with scoring |
| `train-pattern` | Add new code patterns to the database |
| `get-statistics` | Pattern database statistics |
| `get-help` | Usage guide for the MCP server |
| `set-endless-mode` | Toggle compact output (~95% token reduction) |
| `get-observation` | Retrieve archived full output by obs_id |

## Pattern files

Patterns are stored as JSON in `data/patterns/`. Each file groups patterns by framework:
- File naming: `{framework}-{category}.json` or `{framework}-patterns.json`
- Schema: `{ "patterns": [{ id, category, framework, version, title, description, code, tags, usage_count, relevance_score, created_at, updated_at }] }`
- IDs must be globally unique across all pattern files
- Framework names: only alphanumeric, hyphens, underscores (validated by `sanitize_framework_name`)

## Code conventions

- Rust edition 2021, MSRV 1.70+
- Error handling: `anyhow::Result` for application errors, `thiserror` for library errors
- Async runtime: tokio (full features)
- Logging: `tracing` crate, output to stderr only (stdout reserved for MCP protocol)
- Clippy: pedantic lints enabled in `lib.rs`
- All MCP communication uses JSON-RPC 2.0 over stdio
- Security: path traversal protection on all user-supplied paths/names

## Testing protocol for MCP

To test the MCP server manually:
```bash
echo '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}' | cargo run --release 2>/dev/null
```

## Custom skills available

- `/quality-check` - Full quality pipeline (fmt, clippy, build, test, audit, pattern validation)
- `/add-pattern` - Interactive pattern creation wizard
- `/test-mcp` - MCP protocol integration tests
- `/pattern-audit` - Pattern quality and coverage audit
- `/release` - Release preparation workflow
