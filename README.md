# 🦀 MCP Context Rust

> A multi-language Model Context Protocol (MCP) server written in Rust that provides intelligent context analysis, code pattern training, **and persistent memory** for AI assistants. Supports Rust, Node.js, Python, Go, Java, PHP, and .NET projects.

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square)](https://opensource.org/licenses/MIT)
[![MCP](https://img.shields.io/badge/MCP-2024--11--05-blue.svg?style=flat-square)](https://modelcontextprotocol.io)
[![Status](https://img.shields.io/badge/Status-Beta-green.svg?style=flat-square)](https://github.com/scopweb/mcp-context-rust)

---

## 📋 Project Status

> **This is a functional MCP server for context reinforcement, code pattern training, and persistent memory for Claude Desktop.**
>
> ✅ **Phase 0 Complete:** Security hardened, MCP protocol compliant.
>
> ✅ **Phase 1 Memory Core:** Persistent `remember` / `recall` / `get-memory` tools + auto-surfacing in analysis. Memories (global + per-project) survive sessions.
>
> The project explores advanced context management patterns and training mechanisms for AI assistants. Use it as a reference for MCP implementations or adapt the concepts to your own projects.
>
> 📊 **Honest Assessment:** [Does this actually save time?](HONEST_ASSESSMENT.md) | 🛣️ **Future Plans:** [See Roadmap](ROADMAP.md)

## ✨ Features

### Core Functionality
- 🌐 **Multi-Language Support**: Analyze projects in 7+ languages
  - **Rust** (Cargo.toml) - actix-web, axum, tokio
  - **Node.js** (package.json) - React, Vue, Next.js, Express, Svelte
  - **Python** (pyproject.toml) - Django, Flask, FastAPI
  - **Go** (go.mod) - Gin, Fiber
  - **Java** (pom.xml) - Spring, Gradle
  - **PHP** (composer.json) - Laravel, Symfony, WordPress
  - **.NET** (.csproj) - Blazor, ASP.NET Core
- 🔍 **Deep Code Analysis**: Parse project files, analyze code with tree-sitter, detect dependencies
- 📚 **27+ Built-in Patterns**: Best practices for various development scenarios
  - 🔄 Lifecycle (6 patterns)
  - ⚡ Performance (5 patterns)
  - 🌐 JavaScript Interop (4 patterns)
  - 📡 Data & APIs (4 patterns)
  - 🔒 Security (4 patterns)
  - 💉 Dependency Injection (2 patterns)
  - 📦 State Management (2 patterns)
- 🎓 **Pattern Training**: Incremental learning system - add your own patterns
- 🎯 **Context-Aware**: Intelligent suggestions based on project analysis
- 🧠 **Persistent Memory (Phase 1)**: Long-term memory across sessions
  - `remember` important decisions, conventions, gotchas, architecture notes, preferences
  - `recall` with advanced search + scoring
  - `get-memory` for task-aware retrieval (global + project memories)
  - Automatically surfaced by `analyze-project`
  - Stored as JSON (data/memories/) with indexes; hybrid approach (SQLite later)
- 🦀 **Rust Performance**: 10x faster than Python equivalents
- 🔌 **MCP Native**: Works with Claude Desktop and other MCP clients

### Security & Quality
- 🔒 **Automated Security Scanning**: cargo-audit integration with RustSec Database (861 advisories)
- ✅ **Zero Known Vulnerabilities**: 159 dependencies verified, 0 issues found
- 🔍 **Unsafe Code Detection**: cargo-geiger monitoring in CI/CD
- 📋 **Continuous Integration**: GitHub Actions workflow with security, lint, format, and test checks
- 📊 **Code Quality**: Clippy pedantic lints, 42 tests passing, structured tracing logging
- 🛡️ **Input Validation**: Path traversal protection, sanitized inputs

## 🦀 Why Rust?

| Feature | Rust Implementation | Python Equivalent |
|---------|-------------------|-------------------|
| Startup Time | 50ms | 300ms |
| Analysis Time | 120ms | 1.2s |
| Memory Usage | 8MB | 45MB |
| Binary Size | 3MB | 40MB+ deps |

- ⚡ **10x Faster** than Python implementations
- 🔒 **Memory Safe** - zero crashes or leaks
- 📦 **Single Binary** - no runtime dependencies
- 🚀 **Concurrent** - efficient async request handling
- 🎯 **Native Parsing** - tree-sitter integration

## 🚀 Quick Start

### Prerequisites

- **Rust 1.70+** ([Install Rust](https://rustup.rs/))

### Installation

```bash
# Clone the repository
git clone https://github.com/scopweb/mcp-context-rust.git
cd mcp-context-rust

# Build release binary
cargo build --release

# Binary location:
# Windows: target/release/mcp-context-rust.exe
# Linux/Mac: target/release/mcp-context-rust
```

### Configuration for Claude Desktop

#### Windows

Edit: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "context-rust": {
      "command": "C:\\path\\to\\mcp-context-rust\\target\\release\\mcp-context-rust.exe",
      "args": [],
      "env": {
        "MCP_PATTERNS_PATH": "C:\\path\\to\\mcp-context-rust\\data\\patterns"
      }
    }
  }
}
```

#### Linux/Mac

Edit: `~/.config/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "context-rust": {
      "command": "/path/to/mcp-context-rust/target/release/mcp-context-rust",
      "args": [],
      "env": {
        "MCP_PATTERNS_PATH": "/path/to/mcp-context-rust/data/patterns"
      }
    }
  }
}
```

**Important:** Use absolute paths in the configuration.

### Restart Claude Desktop

Close and reopen Claude Desktop to load the MCP server.

---

## 📖 Usage

### Analyze a Project

```
You: Analyze my project at C:\Projects\MyLaravelApp

Claude → calls analyze-project tool
Server → detects PHP/Laravel, parses composer.json, finds Vue frontend
Claude → shows structure, dependencies, framework-specific suggestions
```

**Supported projects:**
- Rust, Node.js, Python, Go, Java, PHP, .NET
- Auto-detects framework (Laravel, React, Django, Spring, etc.)

### Get Code Patterns

```
You: Show me lifecycle patterns

Claude → calls get-patterns tool
Server → returns relevant patterns with code examples
Claude → explains best practices
```

### Search Patterns

```
You: Find patterns for async initialization

Claude → calls search-patterns tool
Server → intelligent search with scoring
Claude → shows most relevant patterns
```

### Train New Patterns

```
You: Save this as a best practice for error handling:
[your code example]

Claude → calls train-pattern tool
Server → stores pattern with metadata
Claude → confirms pattern saved
```

### Get Statistics

```
You: Show pattern database stats

Claude → calls get-statistics tool
Server → returns total patterns, categories, frameworks
```

### Use Persistent Memory (Phase 1)

```
You: Remember that we decided to use Axum after benchmarking

Claude → calls remember tool
Server → stores in project-scoped memory (persists across sessions)

You: Analyze my project at C:\Projects\MyService

Claude → calls analyze-project
Server → surfaces "We decided to use Axum..." automatically in context

You: What did we decide about the web framework?

Claude → calls get-memory or recall
Server → returns relevant memories with scores
```

- Memories are **global** (user-wide) or **per-project** (tied to canonical root path)
- Auto-surfaced by `analyze-project`
- Use `recall` for explicit search; `get-memory` for task-aware "what I need to know now"

### Get Unified Context (Phase 2 - recommended)

```
You: Give me the full context for my Rust project, I'm adding JWT authentication

Claude → calls get-context { "project_path": "...", "task": "adding JWT authentication" }
Server → returns combined: project structure + relevant past decisions (memories) + matching patterns + suggestions, all in one rich response.
```

Use this as the primary tool to get everything without multiple calls.

---

## 🛠️ Available Tools

**12 tools total** (Phase 2: `get-context` unificado añadido).

| Tool | Description | Parameters |
|------|-------------|------------|
| `analyze-project` | Analyze any project + auto-surface relevant persistent memories | `project_path` (string) |
| `get-context` | **UNIFIED (Phase 2)**: Un solo llamado para contexto completo = análisis + memorias (con task para ranking) + mejores patrones + sugerencias | `project_path`, `task` (opcional, para mejorar relevancia) |
| `get-patterns` | Get patterns by framework/category | `framework` (string), `category` (optional) |
| `search-patterns` | Advanced pattern search | `query`, `framework`, `category`, `tags`, `min_score` |
| `train-pattern` | Add custom pattern | `id`, `category`, `framework`, `title`, `description`, `code`, `tags` |
| `get-statistics` | Database statistics | None |
| `get-help` | Usage guide | None |
| `set-endless-mode` | Toggle ~95% token reduction compact mode (full output via `get-observation`) | `enabled` (bool) |
| `get-observation` | Retrieve full archived output by obs_id (for Endless Mode) | `obs_id` (string) |
| `remember` | **(Memory)** Store decision/gotcha/convention/architecture note/preference | `scope` ("global"\|"project"), `project_path` (req for project), `category`, `title`, `content`, `tags`, `importance` |
| `recall` | **(Memory)** Search memories (query, scope, category, tags) with scoring | `query`, `scope`, `project_path`, `category`, `tags`, `min_score`, `max_results` |
| `get-memory` | **(Memory)** Get most relevant memories for current project/task | `project_path`, `task` (hint), `max_results` |

---

## 🏗️ Architecture

```
mcp-context-rust/
├── src/
│   ├── main.rs              # Entry point
│   ├── lib.rs               # Library root (clippy lints, exports)
│   ├── config.rs            # Configuration
│   ├── types.rs             # Shared types (Project, Dependency, etc.)
│   ├── error.rs             # Custom error types (McpError, TrainingError)
│   ├── utils/
│   │   └── mod.rs           # Utility functions (hash, truncate)
│   ├── analyzer/
│   │   ├── mod.rs           # Analyzer module
│   │   ├── detector.rs      # Project type detection
│   │   ├── generic.rs       # Multi-language analyzer
│   │   ├── project.rs       # Legacy .NET analyzer
│   │   └── csharp.rs        # C# tree-sitter parser
│   ├── context/             # Context generation
│   ├── training/            # Pattern management
│   │   └── mod.rs           # Training system
│   ├── memory/              # Persistent memory (Phase 1)
│   │   └── mod.rs           # MemoryStore, remember/recall/get-memory tools
│   └── mcp/                 # MCP protocol
│       └── mod.rs           # Server implementation (12 tools, incl. get-context unificado)
├── data/
│   └── patterns/            # Built-in patterns (JSON)
├── tests/                   # Integration + unit tests (memory, training, analyzer, endless)
├── docs/                    # Technical documentation
├── Cargo.toml
├── README.md
├── CHANGELOG.md
└── LICENSE
```

---

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run specific test module
cargo test analyzer

# Test with output
cargo test -- --nocapture

# Manual test (stdio)
echo '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}' | cargo run --release
```

---

## 📚 Documentation

### Setup & Usage
- **[MCP Setup Guide](docs/MCP_SETUP_GUIDE.md)** (ES) / **[English](docs/MCP_SETUP_GUIDE.en.md)** - Detailed configuration instructions
- **[Usage Examples](docs/USAGE_EXAMPLES.md)** (ES) / **[English](docs/USAGE_EXAMPLES.en.md)** - Practical examples and scenarios

### Development
- **[Development Guide](docs/CLAUDE.md)** - Project architecture and development workflow
- **[Creating MCPs with Rust](docs/CrearUnMcpConRust.md)** - Complete guide to building MCP servers
- **[Pattern Catalog](docs/PATTERNS_CATALOG.md)** - All 27+ built-in patterns
- **[Changelog](CHANGELOG.md)** - Version history

### Project Status & Planning
- **[Honest Assessment](HONEST_ASSESSMENT.md)** - Does this actually save time? (Truthful evaluation)
- **[Roadmap](ROADMAP.md)** - From PoC to production-ready tool
- **[Security Audit](docs/SECURITY_AUDIT.md)** - How cargo-audit works and continuous scanning
- **[Security Report](SECURITY_AUDIT_REPORT.md)** - Latest audit results: 0 vulnerabilities ✅

---

## 🔒 Security Status

This project implements comprehensive security scanning:

```
✅ Automated dependency scanning with cargo-audit
✅ 159 dependencies verified against 861 RustSec advisories
✅ Zero known vulnerabilities found
✅ Continuous monitoring on every push (GitHub Actions)
✅ Daily security checks (2 AM UTC)
✅ Unsafe code detection with cargo-geiger
✅ Code quality enforcement (clippy, fmt)
✅ Comprehensive CI/CD pipeline
```

See [SECURITY_AUDIT_REPORT.md](SECURITY_AUDIT_REPORT.md) for detailed results.

---

## 🤝 Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Adding Patterns

To contribute new patterns:

1. Add JSON file to `data/patterns/`
2. Follow the pattern schema
3. Test with `get-statistics` tool
4. Submit PR with pattern details

---

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🙏 Acknowledgments

- Built with [Rust](https://www.rust-lang.org/) 🦀
- Code parsing with [tree-sitter](https://tree-sitter.github.io/)
- MCP Protocol by [Anthropic](https://www.anthropic.com/)

---

## 🐛 Troubleshooting

### Server not connecting

1. Check Claude Desktop logs: `%APPDATA%\Claude\logs\mcp-server-context-rust.log`
2. Verify executable path is absolute
3. Ensure `MCP_PATTERNS_PATH` points to correct directory
4. Try rebuilding: `cargo clean && cargo build --release`

### No patterns loaded

```bash
# Check patterns directory exists
ls data/patterns/*.json

# Verify environment variable
echo $MCP_PATTERNS_PATH  # Linux/Mac
echo %MCP_PATTERNS_PATH%  # Windows
```

### Parse errors

- Check files are UTF-8 encoded
- Verify project structure is correct

For more help, see [MCP_SETUP_GUIDE.md](docs/MCP_SETUP_GUIDE.md) or open an issue.

---

## 📬 Contact

- **Issues**: [GitHub Issues](https://github.com/scopweb/mcp-context-rust/issues)
- **Discussions**: [GitHub Discussions](https://github.com/scopweb/mcp-context-rust/discussions)

---

<div align="center">

**Made with 🦀 Rust**

[⭐ Star this repository](https://github.com/scopweb/mcp-context-rust) if you find it useful!

</div>
