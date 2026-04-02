use std::process;

use clap::{Parser, Subcommand};
use futures_util::{SinkExt, StreamExt};
use tokio::net::UnixStream;
use tokio_util::codec::{FramedRead, FramedWrite};

use talos_common::protocol::codec::BincodeCodec;
use talos_common::protocol::messages::{Request, Response};
use talos_common::protocol::types::DynValue;

#[derive(Parser)]
#[command(name = "talos", about = "CLI for the Talos ROS 2 bridge")]
struct Cli {
    /// Path to the agent Unix socket
    #[arg(long, default_value = "/tmp/talos.sock", global = true)]
    socket: String,

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
    let stream = UnixStream::connect(&cli.socket).await?;
    let (read_half, write_half) = stream.into_split();
    let mut reader = FramedRead::new(read_half, BincodeCodec::<Response>::new());
    let mut writer = FramedWrite::new(write_half, BincodeCodec::<Request>::new());

    match cli.command {
        Command::ListTopics => {
            writer.send(Request::ListTopics).await?;
            if let Some(Ok(response)) = reader.next().await {
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
        }
        Command::ListNodes => {
            writer.send(Request::ListNodes).await?;
            if let Some(Ok(response)) = reader.next().await {
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
        }
        Command::Echo { topic, count } => {
            // We just listen to the broadcast stream from the agent.
            // The agent already subscribes to configured topics,
            // so we receive TopicData for matching topics.
            let mut received = 0usize;
            while let Some(Ok(response)) = reader.next().await {
                if let Response::TopicData {
                    topic: ref t,
                    data: ref value,
                    ..
                } = response
                {
                    if *t == topic {
                        print_dynvalue(value, 0);
                        println!("---");
                        received += 1;
                        if count > 0 && received >= count {
                            break;
                        }
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
