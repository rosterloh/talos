use talos_common::protocol::types::{NodeInfo, ParamInfo};

use super::AppState;

pub(crate) fn node_fqn(node: &NodeInfo) -> String {
    if node.namespace.is_empty() || node.namespace == "/" {
        format!("/{}", node.name)
    } else {
        format!("{}/{}", node.namespace.trim_end_matches('/'), node.name)
    }
}

pub(crate) fn node_label(node: &NodeInfo) -> String {
    node_fqn(node)
}

impl AppState {
    pub(crate) fn handle_parameters(&mut self, node: String, parameters: Vec<ParamInfo>) {
        self.param_node = Some(node);
        self.parameters = parameters;
        if self.param_selected >= self.parameters.len() {
            self.param_selected = 0;
        }
        self.param_status = Some(format!("{} parameter(s)", self.parameters.len()));
    }

    pub(crate) fn handle_parameter_set(&mut self, name: String, successful: bool, reason: String) {
        self.param_status = Some(if successful {
            format!("set '{name}'")
        } else {
            format!("set '{name}' rejected: {reason}")
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(name: &str, namespace: &str) -> NodeInfo {
        NodeInfo {
            name: name.into(),
            namespace: namespace.into(),
            publishers: Vec::new(),
            subscribers: Vec::new(),
            services: Vec::new(),
        }
    }

    #[test]
    fn node_label_includes_namespace() {
        assert_eq!(node_label(&node("controller", "/left")), "/left/controller");
        assert_eq!(node_label(&node("controller", "/")), "/controller");
    }
}
