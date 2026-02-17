use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod analyzer;
mod config;
mod context;
mod mcp;
mod observations;
mod training;
mod types;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing - ONLY to stderr, no ANSI colors for MCP compatibility
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "mcp_dotnet_context=error".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false) // Disable ANSI color codes
                .with_writer(std::io::stderr), // Force stderr
        )
        .init();

    tracing::info!("🦀 MCP .NET Context Server v0.1.0 starting...");

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
