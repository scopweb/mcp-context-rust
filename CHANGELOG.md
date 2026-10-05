# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

> **📋 Note:** Phase 0 complete (security + MCP). **Phase 1 Memory Core implemented** (persistent `remember`/`recall`/`get-memory` + auto-surfacing in analysis). See ROADMAP.md for remaining phases.

## [Unreleased]

### Added (Phase 2: Context Unification + Pattern Renaissance - STARTED)
- New unified `get-context` tool (the recommended primary tool)
  - Combines in one call: full project analysis + relevant memories (global + project, with optional `task` for relevance ranking) + best matching patterns + suggestions
  - Reduces friction: one call instead of analyze + get-memory + search-patterns
  - Supports `task` param to make memory and pattern retrieval task-aware
  - Respects Endless Mode (compact + obs_id archiving)
  - Also saves .rustscp like analyze
- Added `get-context` to all tool lists, help text, server info, README, and test-mcp skill
- Updated help guide with recommended flow using `get-context`
- Still 9 pattern files (all Blazor), expansion planned in this phase

### Added (Phase 1: Memory Core - Persistent Memory Engine)
- **New `src/memory/mod.rs`** module
  - `MemoryStore` (in-memory + JSON-backed, modeled after TrainingManager for consistency)
  - Indexes by scope + category for fast lookup
  - Secure persistence to `data/memories/memories.json` (single file for v1; hybrid JSON-first per roadmap decisions)
  - `remember()`, `recall()` (scored search), `get_relevant_for_project()`, `mark_recalled()`
- **New core types** (types.rs)
  - `MemoryScope` enum: `Global` | `Project { path }` (canonical paths for stable scoping)
  - `Memory` struct: id (UUID), scope, category, title, content, tags, importance, recall_count, timestamps
  - `RememberInput`, `MemorySearchCriteria`
- **3 new MCP tools** (mcp/mod.rs) — now **11 tools total**
  - `remember`: Store decisions, conventions, gotchas, architecture notes, preferences (global or per-project)
  - `recall`: Advanced search across memories (query, scope, category, tags, min_score, max_results) with relevance scoring
  - `get-memory`: High-level retrieval of most relevant memories (global + project) for a task/project; supports `task` hint for ranking
- **Deep integration in `analyze-project`**
  - Automatically surfaces relevant memories (global + matching project) as `## Relevant Persistent Memories` section
  - Bumps `recall_count` / `last_recalled_at` for surfaced items + persists stats
  - Works with Endless Mode (full memories archived via obs_id; compact notes count)
  - Canonical path normalization for reliable project scoping
- **Server + Config updates**
  - `memory_store` initialized in `Server::new()` (loaded at startup like patterns)
  - `StorageConfig.memories_dir` (default `memories`, respects `MCP_*` env + config)
  - `tool_remember` (mut), `tool_recall`, `tool_get_memory` with full Endless Mode + obs archiving support
  - Updated `tools/list`, `initialize` capabilities, and `get-help` documentation
- **Roadmap alignment**
  - Decisions captured: JSON-first storage, global+per-project scope, structured facts only (no transcripts in v1), Go-sibling API parity in mind (UUID ids, tool names)
  - Follows "Memory is the product" + leverage existing (ObservationStore patterns, TrainingManager style, security hygiene)
  - Unit tests in `memory::tests` (roundtrip, project scope, relevance)
- **Exports + wiring**
  - `pub mod memory;` + re-exports in lib.rs (`MemoryStore`, `Memory`, `MemoryScope`, `RememberInput`)
  - `mod memory;` added to main.rs for binary target
  - All quality gates: `cargo fmt`, `cargo clippy -D warnings`, `cargo test` (memory tests + full suite green)

### Added (Endless Mode - ~95% Token Reduction)
- **`set-endless-mode` tool** (mcp/mod.rs)
  - Toggle compact output at runtime (`{"enabled": true/false}`)
  - State resets on server restart (by design)
- **`get-observation` tool** (mcp/mod.rs)
  - Retrieve full archived output by `obs_id` UUID on demand
- **`ObservationStore`** (observations.rs)
  - UUID v4 keys persisted as JSON files under `data/cache/observations/`
  - UUID validation prevents path traversal before file access
- **Compact output mode** for all major tools (mcp/mod.rs, context/mod.rs)
  - `analyze-project`: full output archived, returns ~150-char summary + `obs_id`
  - `get-patterns`: list condensed to `title[tag,score]` entries + `obs_id`
  - `search-patterns`: results condensed to `[score]title|category|fw` + `obs_id`
  - `get-statistics`: single-line `DB: N patterns across M frameworks` + `obs_id`
  - `get-help`: tool list on one line + `obs_id`
- **`build_compact_context_string()`** (context/mod.rs)
  - Single-line project summary: type, name, version, files, deps, edition, entry point, patterns, suggestions

### Fixed
- **`get-observation` error message** (mcp/mod.rs)
  - Message incorrectly said observations are "stored in memory"; they are
    persisted to disk at `data/cache/observations/` and survive across requests

### Added (Phase 3 - Production Polishing)
- **Custom Error Types** (error.rs)
  - `McpError` - Main error type with variants for Analysis, Training, Config, IO, JSON
  - `AnalysisError` - Project analysis errors (PathNotFound, NotADirectory, ParseError)
  - `TrainingError` - Pattern management errors (InvalidFrameworkName, PathTraversal)
  - `ConfigError` - Configuration errors (FileNotFound, InvalidFormat)
  - Proper error propagation with `thiserror`
  - Type-safe `Result` aliases

- **Code Quality Improvements** (lib.rs)
  - Enabled `clippy::pedantic` lints for production quality
  - Configured appropriate `#![allow(...)]` for acceptable patterns
  - UTF-8 safe `truncate_string()` function (utils/mod.rs)
  - Comprehensive Unicode/emoji test coverage

- **Structured Logging** (mcp/mod.rs, analyzer/generic.rs)
  - Replaced all `eprintln!` with `tracing` macros
  - `tracing::info!` for operational messages
  - `tracing::debug!` for request details
  - `tracing::error!` for error conditions
  - Structured fields for better log analysis

- **Performance Optimizations** (training/mod.rs)
  - Optimized `score_pattern()` to avoid HashSet allocations
  - Direct iteration for tag matching
  - Reduced memory allocations in hot path

### Added
- **Multi-Language Project Support** (analyzer/generic.rs, analyzer/detector.rs)
  - Automatic project type detection based on config files
  - **Rust**: Cargo.toml (detects actix-web, axum, tokio)
  - **Node.js**: package.json (detects React, Vue, Next.js, Express, Svelte)
  - **Python**: pyproject.toml, requirements.txt, setup.py (detects Django, Flask, FastAPI)
  - **Go**: go.mod (detects Gin, Fiber)
  - **Java**: pom.xml, build.gradle (detects Spring)
  - **PHP**: composer.json (detects Laravel, Symfony, WordPress, CodeIgniter, Yii, CakePHP, Slim, Drupal)
  - **.NET**: .csproj, .fsproj, .sln (detects Blazor, ASP.NET Core)
  
- **PHP Framework Detection** (analyzer/generic.rs)
  - Laravel: artisan file, laravel/framework dependency
  - Symfony: symfony/framework-bundle
  - WordPress: wp-config.php, wp-content directory
  - Full composer.json parsing with require/require-dev
  - PHP version requirement extraction
  
- **Frontend Integration for PHP** (analyzer/generic.rs)
  - Vue.js detection from package.json
  - React detection (Inertia.js support)
  - Vite bundler detection
  - Laravel Mix detection
  
- **PHP-specific Suggestions** (context/mod.rs)
  - Missing .env file warning for Laravel
  - Laravel 8.x upgrade suggestions
  - Inertia.js best practices
  - Security package recommendations

- **Generic Project Types** (types.rs)
  - New `Project` struct for multi-language support
  - `ProjectType` enum: DotNet, Rust, Node, Python, Go, Java, Php, Unknown
  - `Dependency` struct with dev_only flag
  - `SourceFile` with language detection
  - `ProjectMetadata` for framework-specific info

### Security
- **CRITICAL: Fixed Path Traversal Vulnerability** (training/mod.rs)
  - Added `sanitize_framework_name()` function to validate framework names
  - Rejects path separators (`/`, `\`, `..`), drive letters (`:`), and null bytes
  - Only allows alphanumeric characters, hyphens, underscores, and dots
  - Added canonical path verification in `save_patterns()`
  - Added validation in `add_pattern()` before storing patterns
  - Maximum length limit (64 chars for framework, 128 for ID)

- **CRITICAL: Implemented MCP Protocol Framing** (mcp/mod.rs)
  - Added proper Content-Length header parsing for incoming messages
  - Added Content-Length header to outgoing responses
  - Maintains backwards compatibility with legacy newline-delimited JSON
  - Server now works correctly with Claude Desktop and standard MCP clients

### Fixed
- **ProjectAnalyzer now returns analyzed C# files** (analyzer/project.rs)
  - `analyze()` method now uses `CSharpAnalyzer` to parse each .cs file
  - Files are populated in `DotNetProject.files` instead of always being empty
  - Errors in individual files are logged but don't fail the entire analysis
  - Fixed unreachable pattern warning in XML parsing

- **Tests now compile and pass** (tests/*.rs)
  - Changed imports from `mcp_dotnet_context` to `mcp_context_rust`
  - Updated `add_pattern` calls to handle new `Result` return type
  - All 42 tests passing (unit tests + integration tests)

### Changed
- **Documentation Cleanup**
  - Renamed project from "MCP .NET Context" to "MCP Context Rust"
  - Removed incorrect .NET/Blazor references from documentation
  - Updated all repository URLs to use `mcp-context-rust`
  - Fixed project name in Cargo.toml metadata

### Added (Phase 2 - Documentation & Security)
- **Security Auditing**
  - cargo-audit integration for dependency vulnerability scanning
  - RustSec Advisory Database monitoring (861 advisories)
  - Automated security checks in GitHub Actions
  - cargo-geiger for unsafe code detection
  - Daily security scans (2 AM UTC)
  - Security audit report showing 0 vulnerabilities ✅

- **Documentation**
  - English translations: MCP_SETUP_GUIDE.en.md, USAGE_EXAMPLES.en.md
  - Security audit guide: docs/SECURITY_AUDIT.md
  - Honest assessment: HONEST_ASSESSMENT.md (does it save time?)
  - Roadmap: ROADMAP.md (path to production)
  - Security audit report: SECURITY_AUDIT_REPORT.md
  - Bilingual documentation (ES/EN)

- **CI/CD Pipeline**
  - GitHub Actions workflow: .github/workflows/security-audit.yml
  - Automated security scanning on every push
  - Scheduled daily vulnerability checks
  - cargo fmt enforcement (code formatting)
  - cargo clippy enforcement (linting)
  - cargo test enforcement (unit tests)
  - Build verification

- **Metadata**
  - `.gitignore` improvements
  - Updated `Cargo.toml` with proper metadata
  - `LICENSE` (MIT)
  - Complete project structure documentation

### Core Features (Phase 1)
- MCP protocol implementation (2024-11-05)
- Project analysis with tree-sitter
- Code parsing (classes, methods, properties, interfaces)
- Project file parsing with dependency detection
- 27+ built-in patterns
  - Lifecycle patterns (6)
  - Performance patterns (5)
  - JavaScript Interop patterns (4)
  - Data & APIs patterns (4)
  - Security patterns (4)
  - Dependency Injection patterns (2)
  - State Management patterns (2)
- Pattern training system with incremental learning
- Pattern search with intelligent scoring
- Context-aware suggestions
- 5 MCP tools:
  - `analyze-project` - Full project analysis
  - `get-patterns` - Retrieve patterns by framework/category
  - `search-patterns` - Advanced pattern search
  - `train-pattern` - Add custom patterns
  - `get-statistics` - Pattern database statistics

### Technical Details
- Written in Rust for performance (10x faster than Python)
- Async/await with Tokio runtime
- Tree-sitter for accurate code parsing
- JSON-RPC 2.0 over stdio transport
- Environment variable configuration support
- GitHub Actions CI/CD
- Automated security scanning with cargo-audit

## [0.1.0] - 2025-10-25

### Added
- Initial project structure
- Basic MCP server implementation

---

[Unreleased]: https://github.com/scopweb/mcp-context-rust/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/scopweb/mcp-context-rust/releases/tag/v0.1.0
