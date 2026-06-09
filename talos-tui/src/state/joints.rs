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
                                .filter_map(|v| {
                                    if let DynValue::String(s) = v {
                                        Some(s.clone())
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
                .unwrap_or_default();

            let positions = extract_f64_array(fields, "position");
            let velocities = extract_f64_array(fields, "velocity");
            let efforts = extract_f64_array(fields, "effort");

            for (i, name) in names.iter().enumerate() {
                if let Some(joint) = self.joints.iter_mut().find(|j| &j.info.name == name) {
                    joint.position = positions.get(i).copied();
                    joint.velocity = velocities.get(i).copied();
                    joint.effort = efforts.get(i).copied();
                }
            }
        }
    }

    pub(crate) fn update_joints_from_urdf(&mut self, urdf_xml: &str) {
        if let Ok(joint_infos) = talos_common::urdf::extract_joints(urdf_xml) {
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
        }
    }
}

fn extract_f64_array(fields: &[(String, DynValue)], name: &str) -> Vec<f64> {
    fields
        .iter()
        .find(|(k, _)| k == name)
        .and_then(|(_, v)| {
            if let DynValue::Array(arr) = v {
                Some(
                    arr.iter()
                        .filter_map(|v| {
                            if let DynValue::F64(f) = v {
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
