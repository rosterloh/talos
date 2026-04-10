use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use talos_agent::router::TopicRouter;
use talos_agent::server::RouterHandle;
use talos_agent::JointPublisher;
use talos_common::config::AgentConfig;
use tokio::sync::Mutex;
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "talos-agent", about = "ROS 2 bridge agent for Talos")]
struct Cli {
    #[arg(short, long, help = "Path to agent config file")]
    config: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "talos_agent=info".into()),
        )
        .init();

    let cli = Cli::parse();
    let config = AgentConfig::load_or_default(cli.config.as_deref())?;
    let config = Arc::new(config);

    let has_uds = config.transport.uds.is_some();
    let has_quic = config.transport.quic.is_some();

    info!(
        uds = has_uds,
        quic = has_quic,
        subscriptions = config.subscriptions.len(),
        "starting talos-agent"
    );

    if !has_uds && !has_quic {
        error!("no transport configured — set [transport.uds] or [transport.quic] in config");
        return Ok(());
    }

    let router: RouterHandle = Arc::new(Mutex::new(TopicRouter::new()));
    let joint_publisher: JointPublisher = Arc::new(Mutex::new(None));

    let shutdown = tokio::signal::ctrl_c();

    // ── UDS listener ──────────────────────────────────────────────────────────
    let uds_handle = if has_uds {
        let config = Arc::clone(&config);
        let router = Arc::clone(&router);
        let joint_pub = Arc::clone(&joint_publisher);
        let h = tokio::spawn(async move {
            if let Err(e) = talos_agent::server::run(config, router, joint_pub).await {
                error!("UDS server error: {e}");
            }
        });
        Some(h)
    } else {
        None
    };

    // ── QUIC listener ─────────────────────────────────────────────────────────
    #[cfg(feature = "quic")]
    let quic_handle = if has_quic {
        let config = Arc::clone(&config);
        let router = Arc::clone(&router);
        let joint_pub = Arc::clone(&joint_publisher);
        let h = tokio::spawn(async move {
            if let Err(e) = talos_agent::server::run_quic(config, router, joint_pub).await {
                error!("QUIC server error: {e}");
            }
        });
        Some(h)
    } else {
        None
    };

    #[cfg(not(feature = "quic"))]
    let quic_handle: Option<tokio::task::JoinHandle<()>> = None;

    // ── ROS 2 bridge ─────────────────────────────────────────────────────────
    let bridge_handle = {
        let config = Arc::clone(&config);
        let router = Arc::clone(&router);
        let joint_pub = Arc::clone(&joint_publisher);
        tokio::spawn(async move {
            if let Err(e) = talos_agent::bridge::run(config, router, joint_pub).await {
                error!("bridge error: {e}");
            }
        })
    };

    // ── Wait for first exit condition ─────────────────────────────────────────
    tokio::select! {
        _ = shutdown => {
            info!("received shutdown signal");
        }
        _ = async {
            if let Some(h) = uds_handle {
                let _ = h.await;
            } else {
                std::future::pending::<()>().await;
            }
        } => {
            error!("UDS server exited unexpectedly");
        }
        _ = async {
            if let Some(h) = quic_handle {
                let _ = h.await;
            } else {
                std::future::pending::<()>().await;
            }
        } => {
            error!("QUIC server exited unexpectedly");
        }
        _ = bridge_handle => {
            error!("bridge exited unexpectedly");
        }
    }

    info!("talos-agent shut down");
    Ok(())
}
