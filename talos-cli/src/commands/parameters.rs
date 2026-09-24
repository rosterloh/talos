use talos_common::protocol::messages::{Request, Response};
use talos_common::protocol::types::{ParamInfo, ParamValue};
use talos_common::session::ProtocolClient;

pub async fn list<C: ProtocolClient>(
    client: &mut C,
    node: String,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let response = client
        .request(Request::ListParameters { node: node.clone() })
        .await?;
    handle_list_params_response(response, json)
}

pub async fn get<C: ProtocolClient>(
    client: &mut C,
    node: String,
    names: Vec<String>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let response = client
        .request(Request::GetParameters {
            node: node.clone(),
            names: names.clone(),
        })
        .await?;
    handle_get_param_response(response, json)
}

pub async fn set<C: ProtocolClient>(
    client: &mut C,
    node: String,
    name: String,
    value: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let parsed = ParamValue::parse(&value);
    let response = client
        .request(Request::SetParameter {
            node: node.clone(),
            name: name.clone(),
            value: parsed,
        })
        .await?;
    match response {
        Response::ParameterSet {
            node,
            name,
            successful: true,
            ..
        } => {
            println!("set {node} {name}");
            Ok(())
        }
        Response::ParameterSet {
            name,
            successful: false,
            reason,
            ..
        } => Err(format!("failed to set '{name}': {reason}").into()),
        Response::Error(e) => Err(e.into()),
        _ => Err("unexpected response".into()),
    }
}

fn handle_list_params_response(
    response: Response,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match response {
        Response::Parameters { parameters, .. } if json => print_json(&parameters),
        Response::Parameters { node, parameters } => {
            println!("{:<40} {:<14} VALUE", "PARAMETER", "TYPE");
            println!("{}", "-".repeat(80));
            for p in &parameters {
                println!("{:<40} {:<14} {}", p.name, p.value.type_name(), p.value);
            }
            println!("\n{} parameter(s) on {node}", parameters.len());
            Ok(())
        }
        Response::Error(e) => Err(e.into()),
        _ => Err("unexpected response".into()),
    }
}

fn handle_get_param_response(
    response: Response,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match response {
        Response::Parameters { parameters, .. } if json => print_json(&parameters),
        Response::Parameters { parameters, .. } => {
            for p in &parameters {
                println!("{}: {} ({})", p.name, p.value, p.value.type_name());
            }
            Ok(())
        }
        Response::Error(e) => Err(e.into()),
        _ => Err("unexpected response".into()),
    }
}

/// `[{"name", "type", "value"}]`, `type` as in the table's TYPE column.
fn print_json(parameters: &[ParamInfo]) -> Result<(), Box<dyn std::error::Error>> {
    let array: Vec<_> = parameters
        .iter()
        .map(|p| {
            serde_json::json!({
                "name": p.name,
                "type": p.value.type_name(),
                "value": crate::json::param_value(&p.value),
            })
        })
        .collect();
    println!("{}", serde_json::to_string(&array)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_params_error_response_returns_error() {
        let err = handle_list_params_response(Response::Error("boom".into()), false)
            .expect_err("list params error response should fail");
        assert_eq!(err.to_string(), "boom");
    }

    #[test]
    fn get_param_error_response_returns_error() {
        let err = handle_get_param_response(Response::Error("missing".into()), false)
            .expect_err("get param error response should fail");
        assert_eq!(err.to_string(), "missing");
    }
}
