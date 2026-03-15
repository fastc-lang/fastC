use clap::Parser;
use fastc_playground::config::PlaygroundConfig;
use fastc_playground::executor::ExecutorLimits;
use fastc_playground::run_server;
use std::net::SocketAddr;
use std::time::Duration;

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

    /// Require this token for API and WebSocket access.
    #[arg(long, env = "FASTC_PLAYGROUND_AUTH_TOKEN")]
    auth_token: Option<String>,

    /// Comma-separated allowed CORS origins. Empty disables CORS.
    #[arg(long, env = "FASTC_PLAYGROUND_ALLOW_ORIGINS", value_delimiter = ',')]
    allow_origins: Vec<String>,

    /// Maximum HTTP request body size.
    #[arg(long, env = "FASTC_PLAYGROUND_MAX_REQUEST_BYTES", default_value_t = 128 * 1024)]
    max_request_bytes: usize,

    /// Maximum FastC source size accepted by run/compile endpoints.
    #[arg(long, env = "FASTC_PLAYGROUND_MAX_CODE_BYTES", default_value_t = 64 * 1024)]
    max_code_bytes: usize,

    /// Maximum run requests allowed per IP per minute.
    #[arg(
        long,
        env = "FASTC_PLAYGROUND_MAX_RUNS_PER_MINUTE",
        default_value_t = 30
    )]
    max_runs_per_minute: usize,

    /// Maximum number of concurrent executions.
    #[arg(
        long,
        env = "FASTC_PLAYGROUND_MAX_CONCURRENT_RUNS",
        default_value_t = 4
    )]
    max_concurrent_runs: usize,

    /// Execution timeout in seconds.
    #[arg(long, env = "FASTC_PLAYGROUND_RUN_TIMEOUT_SECS", default_value_t = 5)]
    run_timeout_secs: u64,

    /// Native C compilation timeout in seconds.
    #[arg(
        long,
        env = "FASTC_PLAYGROUND_COMPILE_TIMEOUT_SECS",
        default_value_t = 10
    )]
    compile_timeout_secs: u64,

    /// Maximum process output bytes streamed to the client.
    #[arg(long, env = "FASTC_PLAYGROUND_MAX_OUTPUT_BYTES", default_value_t = 64 * 1024)]
    max_output_bytes: usize,

    /// Maximum address-space size for executed user binaries in MB (Linux/Unix).
    #[arg(long, env = "FASTC_PLAYGROUND_MAX_MEMORY_MB", default_value_t = 256)]
    max_memory_mb: u64,

    /// Maximum number of processes/threads for executed user binaries (Linux/Unix).
    #[arg(long, env = "FASTC_PLAYGROUND_MAX_PROCESSES", default_value_t = 32)]
    max_processes: u64,
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
    let config = PlaygroundConfig {
        auth_token: cli.auth_token,
        allowed_origins: cli.allow_origins,
        max_request_body_bytes: cli.max_request_bytes,
        max_code_bytes: cli.max_code_bytes,
        max_runs_per_minute: cli.max_runs_per_minute,
        max_concurrent_runs: cli.max_concurrent_runs,
        executor_limits: ExecutorLimits {
            run_timeout: Duration::from_secs(cli.run_timeout_secs),
            compile_timeout: Duration::from_secs(cli.compile_timeout_secs),
            max_output_bytes: cli.max_output_bytes,
            max_memory_bytes: cli.max_memory_mb * 1024 * 1024,
            max_processes: cli.max_processes,
        },
    };

    if !addr.ip().is_loopback() && config.auth_token.is_none() {
        anyhow::bail!(
            "Refusing to bind non-loopback address {} without --auth-token",
            addr.ip()
        );
    }

    // Open browser if requested
    if cli.open {
        let url = format!("http://{}:{}", cli.host, cli.port);
        tracing::info!("Opening browser at {}", url);
        if let Err(e) = open::that(&url) {
            tracing::warn!("Failed to open browser: {}", e);
        }
    }

    run_server(addr, config).await
}
