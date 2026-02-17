use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod analyzer;
mod config;
mod context;
mod mcp;
mod observations;
mod rustscp;
mod training;
mod types;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Subcommand: read-context <dir>
    if args.len() >= 3 && args[1] == "read-context" {
        let dir = std::path::Path::new(&args[2]);
        match rustscp::ProjectContext::load(dir) {
            Ok(Some(ctx)) => {
                print!("{}", ctx.format_for_claude());
                return Ok(());
            }
            Ok(None) => {
                // No .rustscp found — silent exit (expected for non-analyzed projects)
                return Ok(());
            }
            Err(e) => {
                eprintln!("Error reading .rustscp: {}", e);
                std::process::exit(1);
            }
        }
    }

    // Default: run MCP server
    // Initialize tracing - ONLY to stderr, no ANSI colors for MCP compatibility
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "mcp_context_rust=error".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false) // Disable ANSI color codes
                .with_writer(std::io::stderr), // Force stderr
        )
        .init();

    tracing::info!("🦀 MCP Context Rust Server v0.1.0 starting...");

    // Load configuration
    let config = config::Config::load()?;
    tracing::info!("✅ Configuration loaded");

    // Initialize MCP server
    let server = mcp::Server::new(config).await?;
    tracing::info!("🚀 Server initialized");

    // Start server (stdio transport for Claude Desktop)
    server.run().await?;

    Ok(())
}
