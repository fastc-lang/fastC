use clap::Parser;
use fastc_playground::run_server;
use std::net::SocketAddr;

#[derive(Parser)]
#[command(name = "fastc-playground")]
#[command(about = "Browser-based IDE and playground for FastC")]
#[command(version)]
struct Cli {
    /// Port to listen on
    #[arg(short, long, default_value = "3000")]
    port: u16,

    /// Host to bind to
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    host: String,

    /// Open browser automatically
    #[arg(short, long)]
    open: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "fastc_playground=info,tower_http=info".into()),
        )
        .init();

    let cli = Cli::parse();
    let addr: SocketAddr = format!("{}:{}", cli.host, cli.port).parse()?;

    // Open browser if requested
    if cli.open {
        let url = format!("http://{}:{}", cli.host, cli.port);
        tracing::info!("Opening browser at {}", url);
        if let Err(e) = open::that(&url) {
            tracing::warn!("Failed to open browser: {}", e);
        }
    }

    run_server(addr).await
}
