use talos_common::protocol::messages::{Request, Response};
use talos_common::protocol::types::{logger_level_name, parse_logger_level};
use talos_common::session::ProtocolClient;

/// Print the level of a node's logger, or set it when `level` is given.
pub async fn run<C: ProtocolClient>(
    client: &mut C,
    node: String,
    level: Option<String>,
    logger: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let request = match level {
        Some(level) => Request::SetLoggerLevel {
            node,
            logger,
            level: parse_logger_level(&level).ok_or_else(|| {
                format!(
                    "unknown level '{level}' (expected unset, debug, info, warn, error or fatal)"
                )
            })?,
        },
        None => Request::GetLoggerLevel { node, logger },
    };
    handle_response(client.request(request).await?)
}

fn handle_response(response: Response) -> Result<(), Box<dyn std::error::Error>> {
    match response {
        Response::LoggerLevel { logger, level, .. } => {
            println!("{logger}: {}", level_label(level));
            Ok(())
        }
        Response::LoggerLevelSet {
            logger,
            successful: true,
            ..
        } => {
            println!("set {logger}");
            Ok(())
        }
        Response::LoggerLevelSet {
            logger,
            successful: false,
            reason,
            ..
        } => Err(format!("failed to set '{logger}': {reason}").into()),
        Response::Error(e) => Err(e.into()),
        _ => Err("unexpected response".into()),
    }
}

fn level_label(level: u32) -> String {
    logger_level_name(level).map_or_else(|| level.to_string(), str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejected_set_returns_error() {
        let err = handle_response(Response::LoggerLevelSet {
            node: "/n".into(),
            logger: "n".into(),
            successful: false,
            reason: "bad level".into(),
        })
        .expect_err("rejected set should fail");
        assert_eq!(err.to_string(), "failed to set 'n': bad level");
    }
}
