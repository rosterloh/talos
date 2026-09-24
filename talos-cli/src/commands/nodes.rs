use talos_common::protocol::messages::{Request, Response};
use talos_common::session::ProtocolClient;

pub async fn list<C: ProtocolClient>(
    client: &mut C,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let response = client.request(Request::ListNodes).await?;
    handle_response(response, json)
}

fn handle_response(response: Response, json: bool) -> Result<(), Box<dyn std::error::Error>> {
    match response {
        Response::NodeList(nodes) if json => {
            println!("{}", serde_json::to_string(&nodes)?);
            Ok(())
        }
        Response::NodeList(nodes) => {
            println!("{:<30} {:<20}", "NODE", "NAMESPACE");
            println!("{}", "-".repeat(52));
            for n in &nodes {
                println!("{:<30} {:<20}", n.name, n.namespace);
            }
            println!("\n{} node(s)", nodes.len());
            Ok(())
        }
        Response::Error(e) => Err(e.into()),
        _ => Err("unexpected response".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_response_returns_error() {
        let err = handle_response(Response::Error("missing graph".into()), false)
            .expect_err("node list error response should fail");
        assert_eq!(err.to_string(), "missing graph");
    }
}
