use talos_common::protocol::types::DynValue;
use talos_common::session::ProtocolClient;

pub async fn run<C: ProtocolClient>(
    client: &mut C,
    topic: String,
    count: usize,
) -> Result<(), Box<dyn std::error::Error>> {
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
