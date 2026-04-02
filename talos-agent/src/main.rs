mod bridge;
mod conversions;
mod server;

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use talos_common::config::AgentConfig;
use tokio::sync::{broadcast, Mutex};
use tracing::{error, info};

pub type JointPublisher = Arc<Mutex<Option<rclrs::Publisher<sensor_msgs::msg::JointState>>>>;

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

    info!(
        socket_path = %config.transport.socket_path,
        subscriptions = config.subscriptions.len(),
        "starting talos-agent"
    );

    let (broadcast_tx, _) = broadcast::channel::<talos_common::protocol::messages::Response>(256);
    let joint_publisher: JointPublisher = Arc::new(Mutex::new(None));

    let shutdown = tokio::signal::ctrl_c();

    let server_handle = {
        let config = Arc::clone(&config);
        let broadcast_tx = broadcast_tx.clone();
        let joint_pub = Arc::clone(&joint_publisher);
        tokio::spawn(async move {
            if let Err(e) = server::run(config, broadcast_tx, joint_pub).await {
                error!("server error: {e}");
            }
        })
    };

    let bridge_handle = {
        let config = Arc::clone(&config);
        let broadcast_tx = broadcast_tx.clone();
        let joint_pub = Arc::clone(&joint_publisher);
        tokio::spawn(async move {
            if let Err(e) = bridge::run(config, broadcast_tx, joint_pub).await {
                error!("bridge error: {e}");
            }
        })
    };

    tokio::select! {
        _ = shutdown => {
            info!("received shutdown signal");
        }
        _ = server_handle => {
            error!("server exited unexpectedly");
        }
        _ = bridge_handle => {
            error!("bridge exited unexpectedly");
        }
    }

    info!("talos-agent shut down");
    Ok(())
}
