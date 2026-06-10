use std::time::Duration;

use talos_common::protocol::messages::Response;
use talos_common::protocol::types::{ParamInfo, ParamValue};
use tracing::{info, warn};

use crate::GraphHandle;
use crate::conversions::{param_value_from_ros, param_value_to_ros};

const PARAM_SERVICE_WAIT: Duration = Duration::from_secs(5);
const PARAM_CALL_TIMEOUT: Duration = Duration::from_secs(10);

/// List all parameters declared by `node` and fetch their current values.
pub(super) async fn list_parameters(node: &str, graph_handle: &GraphHandle) -> Response {
    let ros_node = match graph_handle.lock().await.clone() {
        Some(n) => n,
        None => return Response::Error("ROS 2 node not available yet".into()),
    };
    let base = match normalize_node_name(node) {
        Some(b) => b,
        None => return Response::Error("node name must not be empty".into()),
    };

    let list_service = format!("{base}/list_parameters");
    let list_client = match ros_node
        .create_client::<rcl_interfaces::srv::ListParameters>(list_service.as_str())
    {
        Ok(c) => c,
        Err(e) => return Response::Error(format!("failed to create parameter client: {e}")),
    };
    if !wait_for_service(&list_client).await {
        return Response::Error(format!("parameter services for '{base}' are not available"));
    }

    let list_req = rcl_interfaces::srv::ListParameters_Request::default();
    let names = match call_service(&list_client, list_req).await {
        Ok(resp) => resp.result.names,
        Err(e) => return Response::Error(e),
    };
    drop(list_client);

    match fetch_parameter_values(&ros_node, &base, names).await {
        Ok(parameters) => Response::Parameters {
            node: base,
            parameters,
        },
        Err(e) => Response::Error(e),
    }
}

/// Fetch the current values of named parameters of `node`.
pub(super) async fn get_parameters(
    node: &str,
    names: &[String],
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

    match fetch_parameter_values(&ros_node, &base, names.to_vec()).await {
        Ok(parameters) => Response::Parameters {
            node: base,
            parameters,
        },
        Err(e) => Response::Error(e),
    }
}

/// Set a single parameter on `node` and report the per-parameter result.
pub(super) async fn set_parameter(
    node: &str,
    name: &str,
    value: &ParamValue,
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

    let set_service = format!("{base}/set_parameters");
    let set_client =
        match ros_node.create_client::<rcl_interfaces::srv::SetParameters>(set_service.as_str()) {
            Ok(c) => c,
            Err(e) => return Response::Error(format!("failed to create parameter client: {e}")),
        };
    if !wait_for_service(&set_client).await {
        return Response::Error(format!("parameter services for '{base}' are not available"));
    }

    let param = rcl_interfaces::msg::Parameter {
        name: name.to_string(),
        value: param_value_to_ros(value),
    };
    let set_req = rcl_interfaces::srv::SetParameters_Request {
        parameters: vec![param],
    };

    let resp = match call_service(&set_client, set_req).await {
        Ok(r) => r,
        Err(e) => return Response::Error(e),
    };
    match resp.results.into_iter().next() {
        Some(result) => {
            if result.successful {
                info!(node = %base, name = %name, "parameter set");
            } else {
                warn!(node = %base, name = %name, reason = %result.reason, "parameter set rejected");
            }
            Response::ParameterSet {
                node: base,
                name: name.to_string(),
                successful: result.successful,
                reason: result.reason,
            }
        }
        None => Response::Error("set_parameters returned no result".into()),
    }
}

/// Call `<node>/get_parameters` for `names` and pair each name with its value.
async fn fetch_parameter_values(
    ros_node: &rclrs::Node,
    base: &str,
    names: Vec<String>,
) -> Result<Vec<ParamInfo>, String> {
    if names.is_empty() {
        return Ok(Vec::new());
    }

    let get_service = format!("{base}/get_parameters");
    let get_client = ros_node
        .create_client::<rcl_interfaces::srv::GetParameters>(get_service.as_str())
        .map_err(|e| format!("failed to create parameter client: {e}"))?;
    if !wait_for_service(&get_client).await {
        return Err(format!("parameter services for '{base}' are not available"));
    }

    let get_req = rcl_interfaces::srv::GetParameters_Request {
        names: names.clone(),
    };
    let resp = call_service(&get_client, get_req).await?;

    Ok(names
        .into_iter()
        .zip(resp.values.iter())
        .map(|(name, value)| ParamInfo {
            name,
            value: param_value_from_ros(value),
        })
        .collect())
}

/// Normalise a node name to a fully-qualified, absolute name (leading `/`,
/// no trailing `/`). Returns `None` for an empty or root-only name.
fn normalize_node_name(node: &str) -> Option<String> {
    let trimmed = node.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with('/') {
        Some(trimmed.to_string())
    } else {
        Some(format!("/{trimmed}"))
    }
}

/// Wait until a service server is available for `client`, bounded by
/// [`PARAM_SERVICE_WAIT`].
async fn wait_for_service<T: rosidl_runtime_rs::Service>(client: &rclrs::Client<T>) -> bool {
    if client.service_is_ready().unwrap_or(false) {
        return true;
    }
    let ready = client.notify_on_service_ready();
    matches!(
        tokio::time::timeout(PARAM_SERVICE_WAIT, ready).await,
        Ok(Ok(()))
    )
}

/// Send a request on `client` and await its response, bounded by
/// [`PARAM_CALL_TIMEOUT`].
async fn call_service<T: rosidl_runtime_rs::Service>(
    client: &rclrs::Client<T>,
    request: T::Request,
) -> Result<T::Response, String> {
    let promise: rclrs::Promise<T::Response> = client
        .call(request)
        .map_err(|e| format!("parameter service call failed: {e}"))?;
    match tokio::time::timeout(PARAM_CALL_TIMEOUT, promise).await {
        Ok(Ok(resp)) => Ok(resp),
        Ok(Err(_)) => Err("parameter service response was dropped".into()),
        Err(_) => Err("parameter service call timed out".into()),
    }
}
