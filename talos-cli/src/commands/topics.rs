use talos_common::protocol::messages::{Request, Response};
use talos_common::session::ProtocolClient;

pub async fn list<C: ProtocolClient>(client: &mut C) -> Result<(), Box<dyn std::error::Error>> {
    let response = client.request(Request::ListTopics).await?;
    handle_response(response)
}

fn handle_response(response: Response) -> Result<(), Box<dyn std::error::Error>> {
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
        let err = handle_response(Response::Error("boom".into()))
            .expect_err("topic list error response should fail");
        assert_eq!(err.to_string(), "boom");
    }
}
