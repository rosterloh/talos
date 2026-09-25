use std::collections::HashMap;

use talos_common::protocol::types::{DynValue, JointInfo};

use super::AppState;

#[derive(Debug, Clone)]
pub struct JointData {
    pub info: JointInfo,
    pub position: Option<f64>,
    pub velocity: Option<f64>,
    pub effort: Option<f64>,
}

type JointSnapshot = (Option<f64>, Option<f64>, Option<f64>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JointFocus {
    JointList,
    PoseList,
}

impl AppState {
    pub(crate) fn update_joints_from_data(&mut self, data: &DynValue) {
        if let DynValue::Struct { fields, .. } = data {
            let names: Vec<String> = fields
                .iter()
                .find(|(k, _)| k == "name")
                .and_then(|(_, v)| {
                    if let DynValue::Array(arr) = v {
                        Some(
                            arr.iter()
                                .map(|v| {
                                    if let DynValue::String(s) = v {
                                        s.clone()
                                    } else {
                                        String::new()
                                    }
                                })
                                .collect(),
                        )
                    } else {
                        None
                    }
                })
                .unwrap_or_default();

            let positions = extract_f64_array(fields, "position");
            let velocities = extract_f64_array(fields, "velocity");
            let efforts = extract_f64_array(fields, "effort");

            for (i, name) in names.iter().enumerate() {
                if let Some(joint) = self.joints.iter_mut().find(|j| &j.info.name == name) {
                    joint.position = positions.get(i).copied().flatten();
                    joint.velocity = velocities.get(i).copied().flatten();
                    joint.effort = efforts.get(i).copied().flatten();
                }
            }
        }
    }

    pub(crate) fn update_joints_from_urdf(&mut self, urdf_xml: &str) {
        if let Ok(joint_infos) = talos_common::urdf::extract_joints(urdf_xml) {
            let selected = self
                .joints
                .get(self.joint_selected)
                .map(|j| j.info.name.clone());
            // Preserve existing position data
            let existing: HashMap<String, JointSnapshot> = self
                .joints
                .iter()
                .map(|j| (j.info.name.clone(), (j.position, j.velocity, j.effort)))
                .collect();
            self.joints = joint_infos
                .into_iter()
                .map(|info| {
                    let (position, velocity, effort) = existing
                        .get(&info.name)
                        .copied()
                        .unwrap_or((None, None, None));
                    JointData {
                        info,
                        position,
                        velocity,
                        effort,
                    }
                })
                .collect();
            self.joint_selected = selected
                .and_then(|name| self.joints.iter().position(|j| j.info.name == name))
                .unwrap_or(self.joint_selected.min(self.joints.len().saturating_sub(1)));
            if let Some(data) = self
                .topics
                .get("/joint_states")
                .and_then(|t| t.latest.clone())
            {
                self.update_joints_from_data(&data);
            }
        }
    }
}

fn extract_f64_array(fields: &[(String, DynValue)], name: &str) -> Vec<Option<f64>> {
    fields
        .iter()
        .find(|(k, _)| k == name)
        .and_then(|(_, v)| {
            if let DynValue::Array(arr) = v {
                Some(
                    arr.iter()
                        .map(|v| {
                            if let DynValue::F64(f) = v
                                && f.is_finite()
                            {
                                Some(*f)
                            } else {
                                None
                            }
                        })
                        .collect(),
                )
            } else {
                None
            }
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use talos_common::protocol::types::JointType;

    #[test]
    fn telemetry_before_urdf_is_applied_and_selection_survives_urdf_refresh() {
        let mut state = AppState::default();
        state.handle_topic_data(
            "/joint_states".into(),
            "JointState".into(),
            DynValue::Struct {
                type_name: "JointState".into(),
                fields: vec![
                    (
                        "name".into(),
                        DynValue::Array(vec![DynValue::String("b".into())]),
                    ),
                    ("position".into(), DynValue::Array(vec![DynValue::F64(0.7)])),
                ],
            },
        );
        let urdf = |names: &[&str]| {
            format!("<robot name='test'><link name='base'/><link name='tip'/>{}</robot>",
            names.iter().map(|name| format!("<joint name='{name}' type='continuous'><parent link='base'/><child link='tip'/></joint>")).collect::<String>())
        };
        state.update_joints_from_urdf(&urdf(&["a", "b"]));
        assert_eq!(state.joints[1].position, Some(0.7));
        state.joint_selected = 1;
        state.update_joints_from_urdf(&urdf(&["b", "a"]));
        assert_eq!(state.joints[state.joint_selected].info.name, "b");
        assert_eq!(state.joints[state.joint_selected].position, Some(0.7));
    }

    #[test]
    fn malformed_or_nonfinite_telemetry_does_not_shift_other_joints() {
        let mut state = AppState::default();
        for name in ["a", "b", "c"] {
            state.joints.push(JointData {
                info: JointInfo {
                    name: name.into(),
                    joint_type: JointType::Continuous,
                    parent_link: "base".into(),
                    child_link: name.into(),
                    limits: None,
                },
                position: Some(42.0),
                velocity: None,
                effort: None,
            });
        }
        state.update_joints_from_data(&DynValue::Struct {
            type_name: "JointState".into(),
            fields: vec![
                (
                    "name".into(),
                    DynValue::Array(
                        ["a", "b", "c"]
                            .into_iter()
                            .map(|s| DynValue::String(s.into()))
                            .collect(),
                    ),
                ),
                (
                    "position".into(),
                    DynValue::Array(vec![
                        DynValue::String("bad".into()),
                        DynValue::F64(2.0),
                        DynValue::F64(f64::NAN),
                    ]),
                ),
            ],
        });
        assert_eq!(
            state.joints.iter().map(|j| j.position).collect::<Vec<_>>(),
            vec![None, Some(2.0), None]
        );
    }
}
