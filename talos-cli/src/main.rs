mod commands;

use std::process;

use clap::{Parser, Subcommand};
use talos_common::session::ProtocolClient;
use talos_common::session::uds::UdsProtocolClient;

#[derive(Parser)]
#[command(name = "talos", about = "CLI for the Talos ROS 2 bridge")]
struct Cli {
    /// Path to the agent Unix socket (mutually exclusive with --remote)
    #[arg(
        long,
        default_value = "/tmp/talos.sock",
        global = true,
        conflicts_with = "remote"
    )]
    socket: String,

    /// Remote agent address for QUIC transport, e.g. 192.168.1.50:4433
    /// (mutually exclusive with --socket)
    #[arg(long, global = true)]
    remote: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List topics the agent is subscribed to
    ListTopics,
    /// List ROS 2 nodes
    ListNodes,
    /// Echo live data from a topic
    Echo {
        /// Topic name to echo
        topic: String,
        /// Number of messages to print (0 = unlimited)
        #[arg(short, long, default_value_t = 0)]
        count: usize,
    },
    /// List a node's parameters with their current values
    ListParams {
        /// Fully-qualified node name, e.g. /talos_agent
        node: String,
    },
    /// Get specific parameter values from a node
    GetParam {
        /// Fully-qualified node name, e.g. /talos_agent
        node: String,
        /// One or more parameter names
        #[arg(required = true, num_args = 1..)]
        names: Vec<String>,
    },
    /// Set a parameter value on a node (type is inferred from the value)
    SetParam {
        /// Fully-qualified node name, e.g. /talos_agent
        node: String,
        /// Parameter name
        name: String,
        /// New value, e.g. true, 42, 3.14, hello, "[1, 2, 3]"
        value: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli).await {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "quic")]
    if let Some(ref addr) = cli.remote {
        let client = talos_common::session::QuicProtocolClient::connect(addr).await?;
        return run_with_client(client, cli.command).await;
    }

    #[cfg(not(feature = "quic"))]
    if cli.remote.is_some() {
        return Err("this build was compiled without QUIC support (--remote not available)".into());
    }

    let client = UdsProtocolClient::connect(&cli.socket).await?;
    run_with_client(client, cli.command).await
}

async fn run_with_client<C: ProtocolClient>(
    mut client: C,
    command: Command,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        Command::ListTopics => commands::topics::list(&mut client).await?,
        Command::ListNodes => commands::nodes::list(&mut client).await?,
        Command::Echo { topic, count } => commands::echo::run(&mut client, topic, count).await?,
        Command::ListParams { node } => commands::parameters::list(&mut client, node).await?,
        Command::GetParam { node, names } => {
            commands::parameters::get(&mut client, node, names).await?
        }
        Command::SetParam { node, name, value } => {
            commands::parameters::set(&mut client, node, name, value).await?
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use talos_common::error::Error;
    use talos_common::protocol::messages::{Request, Response};
    use talos_common::protocol::types::{TopicFrame, TopicSub};

    struct FakeClient {
        response: Option<Response>,
    }

    impl FakeClient {
        fn with_response(response: Response) -> Self {
            Self {
                response: Some(response),
            }
        }
    }

    impl ProtocolClient for FakeClient {
        async fn request(&mut self, _req: Request) -> Result<Response, Error> {
            self.response
                .take()
                .ok_or_else(|| Error::Config("no response queued".into()))
        }

        async fn subscribe(&mut self, _topics: &[String]) -> Result<Vec<TopicSub>, Error> {
            Ok(Vec::new())
        }

        async fn unsubscribe(&mut self, _topics: &[String]) -> Result<Vec<String>, Error> {
            Ok(Vec::new())
        }

        async fn recv_data(&mut self) -> Result<(String, TopicFrame), Error> {
            Err(Error::Config("no topic data queued".into()))
        }
    }

    #[tokio::test]
    async fn list_topics_error_response_returns_error() {
        let err = run_with_client(
            FakeClient::with_response(Response::Error("boom".into())),
            Command::ListTopics,
        )
        .await
        .expect_err("list topics error response should fail");

        assert_eq!(err.to_string(), "boom");
    }

    #[tokio::test]
    async fn list_nodes_error_response_returns_error() {
        let err = run_with_client(
            FakeClient::with_response(Response::Error("missing graph".into())),
            Command::ListNodes,
        )
        .await
        .expect_err("list nodes error response should fail");

        assert_eq!(err.to_string(), "missing graph");
    }
}
