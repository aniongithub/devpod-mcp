mod tools;

use clap::{Parser, Subcommand};
use rmcp::transport::stdio;
use rmcp::ServiceExt;

#[derive(Parser)]
#[command(name = "devcontainer-mcp")]
#[command(about = "MCP server and CLI for managing DevContainers")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the MCP server over stdio
    Serve,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve => {
            tracing::info!("Starting devcontainer-mcp MCP server over stdio");
            let service = tools::DevContainerMcp::new();
            let server = service.serve(stdio()).await?;
            server.waiting().await?;
        }
    }

    Ok(())
}
