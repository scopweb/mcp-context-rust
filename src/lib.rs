//! MCP Rust Context Server
//!
//! A specialized Model Context Protocol (MCP) server that provides intelligent
//! context analysis and code pattern training for multiple programming languages.
//!
//! ## Supported Languages
//!
//! - **Rust** - Cargo.toml projects
//! - **Node.js** - package.json projects
//! - **Python** - pyproject.toml, setup.py, requirements.txt
//! - **.NET** - .csproj, .sln files
//! - **Go** - go.mod projects
//! - **Java** - pom.xml, build.gradle
//! - **PHP** - composer.json (Laravel, Symfony, etc.)
//!
//! ## Features
//!
//! - Automatic project type detection
//! - Dependency analysis
//! - Code pattern training and search
//! - Framework-specific suggestions
//!
//! ## Example
//!
//! ```no_run
//! use mcp_context_rust::{Config, Server};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = Config::load()?;
//!     let server = Server::new(config).await?;
//!     server.run().await
//! }
//! ```

// Clippy lints for production quality
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
// Note: Disabled nursery lints as they're experimental and change frequently
// #![warn(clippy::nursery)]
//
// Allow pedantic lints that are too strict for this codebase
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::cognitive_complexity)]
#![allow(clippy::significant_drop_tightening)]
#![allow(clippy::unused_async)] // Async for API consistency
#![allow(clippy::similar_names)] // Domain names like has_getter/has_setter
#![allow(clippy::unused_self)] // Methods may use self in future
#![allow(clippy::if_not_else)] // Readability preference
#![allow(clippy::struct_excessive_bools)] // Legacy .NET types
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::match_same_arms)] // Readability in match statements
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::uninlined_format_args)] // Style preference
#![allow(clippy::unnecessary_wraps)] // API consistency
#![allow(clippy::items_after_statements)] // Local structs in functions
#![allow(clippy::format_push_string)] // push_str + format! is fine
#![allow(clippy::map_unwrap_or)] // map().unwrap_or() is readable
#![allow(clippy::option_if_let_else)] // Style preference
#![allow(clippy::return_self_not_must_use)] // Builder pattern
#![allow(clippy::assigning_clones)] // Clone assignment is clear
#![allow(clippy::ignored_unit_patterns)] // Style preference
#![allow(clippy::doc_markdown)] // Skip doc backticks check
#![allow(clippy::use_debug)] // Debug formatting is fine in tracing

pub mod analyzer;
pub mod config;
pub mod context;
pub mod error;
pub mod mcp;
pub mod observations;
pub mod training;
pub mod types;
pub mod utils;

pub use config::Config;
pub use error::{AnalysisError, ConfigError, McpError, TrainingError};
pub use mcp::Server;
pub use training::TrainingManager;
pub use types::{AnalysisResult, CodePattern, Project, ProjectType};
