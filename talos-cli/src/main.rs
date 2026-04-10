use std::process;

use clap::{Parser, Subcommand};
use talos_common::protocol::messages::{Request, Response};
use talos_common::protocol::types::DynValue;
use talos_common::session::uds::UdsProtocolClient;
use talos_common::session::ProtocolClient;

#[derive(Parser)]
#[command(name = "talos", about = "CLI for the Talos ROS 2 bridge")]
struct Cli {
    /// Path to the agent Unix socket (mutually exclusive with --remote)
    #[arg(long, default_value = "/tmp/talos.sock", global = true, conflicts_with = "remote")]
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
        Command::ListTopics => {
            let response = client.request(Request::ListTopics).await?;
            match response {
                Response::TopicList(topics) => {
                    println!("{:<30} {:<35} {:>4} {:>4}", "TOPIC", "TYPE", "PUB", "SUB");
                    println!("{}", "-".repeat(75));
                    for t in &topics {
                        println!(
                            "{:<30} {:<35} {:>4} {:>4}",
                            t.name, t.type_name, t.publisher_count, t.subscriber_count
                        );
                    }
                    println!("\n{} topic(s)", topics.len());
                }
                Response::Error(e) => eprintln!("error: {e}"),
                _ => eprintln!("unexpected response"),
            }
        }
        Command::ListNodes => {
            let response = client.request(Request::ListNodes).await?;
            match response {
                Response::NodeList(nodes) => {
                    println!("{:<30} {:<20}", "NODE", "NAMESPACE");
                    println!("{}", "-".repeat(52));
                    for n in &nodes {
                        println!("{:<30} {:<20}", n.name, n.namespace);
                    }
                    println!("\n{} node(s)", nodes.len());
                }
                Response::Error(e) => eprintln!("error: {e}"),
                _ => eprintln!("unexpected response"),
            }
        }
        Command::Echo { topic, count } => {
            // Subscribe to the specific topic before listening
            match client.subscribe(&[topic.clone()]).await {
                Ok(subs) if subs.is_empty() => {
                    eprintln!("warning: agent did not confirm subscription to '{topic}'");
                    eprintln!("(the agent may not be subscribed to this topic)");
                }
                Err(e) => {
                    return Err(format!("failed to subscribe to '{topic}': {e}").into());
                }
                _ => {}
            }

            let mut received = 0usize;
            loop {
                let (recv_topic, frame) = client.recv_data().await?;
                if recv_topic == topic {
                    print_dynvalue(&frame.data, 0);
                    println!("---");
                    received += 1;
                    if count > 0 && received >= count {
                        break;
                    }
                }
            }

            if received == 0 {
                eprintln!("no data received for topic '{topic}'");
                eprintln!("(the agent may not be subscribed to this topic)");
            }
        }
    }

    Ok(())
}

fn print_dynvalue(value: &DynValue, indent: usize) {
    let pad = "  ".repeat(indent);
    match value {
        DynValue::Bool(v) => println!("{pad}{v}"),
        DynValue::I8(v) => println!("{pad}{v}"),
        DynValue::U8(v) => println!("{pad}{v}"),
        DynValue::I16(v) => println!("{pad}{v}"),
        DynValue::U16(v) => println!("{pad}{v}"),
        DynValue::I32(v) => println!("{pad}{v}"),
        DynValue::U32(v) => println!("{pad}{v}"),
        DynValue::I64(v) => println!("{pad}{v}"),
        DynValue::U64(v) => println!("{pad}{v}"),
        DynValue::F32(v) => println!("{pad}{v}"),
        DynValue::F64(v) => println!("{pad}{v}"),
        DynValue::String(v) => println!("{pad}\"{v}\""),
        DynValue::Bytes(v) => println!("{pad}[{} bytes]", v.len()),
        DynValue::Array(arr) => {
            println!("{pad}[");
            for item in arr {
                print_dynvalue(item, indent + 1);
            }
            println!("{pad}]");
        }
        DynValue::Struct { type_name, fields } => {
            println!("{pad}{type_name} {{");
            for (name, val) in fields {
                print!("{pad}  {name}: ");
                match val {
                    DynValue::Struct { .. } | DynValue::Array(_) => {
                        println!();
                        print_dynvalue(val, indent + 2);
                    }
                    _ => print_dynvalue(val, 0),
                }
            }
            println!("{pad}}}");
        }
    }
}
