use ros_env::rcl_interfaces;

use talos_common::protocol::messages::Response;
use tracing::{info, warn};

use super::parameters::{call_service, normalize_node_name, wait_for_service};
use crate::GraphHandle;

/// Fetch the level of `logger` (default: the node's own logger) on `node`.
pub(super) async fn get_logger_level(
    node: &str,
    logger: &str,
    graph_handle: &GraphHandle,
) -> Response {
    let ros_node = match graph_handle.lock().await.clone() {
        Some(n) => n,
        None => return Response::Error("ROS 2 node not available yet".into()),
    };
    let base = match normalize_node_name(node) {
        Some(b) => b,
        None => return Response::Error("node name must not be empty".into()),
    };
    let logger = resolve_logger(&base, logger);

    let service = format!("{base}/get_logger_levels");
    let client =
        match ros_node.create_client::<rcl_interfaces::srv::GetLoggerLevels>(service.as_str()) {
            Ok(c) => c,
            Err(e) => return Response::Error(format!("failed to create logger client: {e}")),
        };
    if !wait_for_service(&client).await {
        return Response::Error(unavailable(&base));
    }

    let req = rcl_interfaces::srv::GetLoggerLevels_Request {
        names: vec![logger.clone()],
    };
    match call_service(&client, req).await {
        Ok(resp) => match resp.levels.into_iter().next() {
            Some(l) => Response::LoggerLevel {
                node: base,
                logger,
                level: l.level,
            },
            None => Response::Error("get_logger_levels returned no level".into()),
        },
        Err(e) => Response::Error(e),
    }
}

/// Set the level of `logger` (default: the node's own logger) on `node`.
pub(super) async fn set_logger_level(
    node: &str,
    logger: &str,
    level: u32,
    graph_handle: &GraphHandle,
) -> Response {
    let ros_node = match graph_handle.lock().await.clone() {
        Some(n) => n,
        None => return Response::Error("ROS 2 node not available yet".into()),
    };
    let base = match normalize_node_name(node) {
        Some(b) => b,
        None => return Response::Error("node name must not be empty".into()),
    };
    let logger = resolve_logger(&base, logger);

    let service = format!("{base}/set_logger_levels");
    let client =
        match ros_node.create_client::<rcl_interfaces::srv::SetLoggerLevels>(service.as_str()) {
            Ok(c) => c,
            Err(e) => return Response::Error(format!("failed to create logger client: {e}")),
        };
    if !wait_for_service(&client).await {
        return Response::Error(unavailable(&base));
    }

    let req = rcl_interfaces::srv::SetLoggerLevels_Request {
        levels: vec![rcl_interfaces::msg::LoggerLevel {
            name: logger.clone(),
            level,
        }],
    };
    let resp = match call_service(&client, req).await {
        Ok(r) => r,
        Err(e) => return Response::Error(e),
    };
    match resp.results.into_iter().next() {
        Some(result) => {
            if result.successful {
                info!(node = %base, logger = %logger, level, "logger level set");
            } else {
                warn!(node = %base, logger = %logger, reason = %result.reason, "logger level set rejected");
            }
            Response::LoggerLevelSet {
                node: base,
                logger,
                successful: result.successful,
                reason: result.reason,
            }
        }
        None => Response::Error("set_logger_levels returned no result".into()),
    }
}

fn unavailable(base: &str) -> String {
    format!(
        "logger services for '{base}' are not available \
         (the node must enable them, e.g. rclcpp NodeOptions().enable_logger_service(true))"
    )
}

/// `logger`, or the node's own logger name when empty: the fully-qualified
/// node name without the leading `/`, with the remaining `/` replaced by `.`
/// (`/ns/foo` becomes `ns.foo`).
fn resolve_logger(base: &str, logger: &str) -> String {
    let logger = logger.trim();
    if logger.is_empty() {
        base.trim_start_matches('/').replace('/', ".")
    } else {
        logger.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_logger_follows_node_name() {
        assert_eq!(resolve_logger("/foo", ""), "foo");
        assert_eq!(resolve_logger("/ns/sub/foo", " "), "ns.sub.foo");
        assert_eq!(resolve_logger("/ns/foo", "rclcpp"), "rclcpp");
    }
}
