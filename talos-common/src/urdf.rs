use crate::error::Error;
use crate::protocol::types::{JointInfo, JointLimits, JointType};

pub fn extract_joints(urdf_xml: &str) -> Result<Vec<JointInfo>, Error> {
    let robot = urdf_rs::read_from_string(urdf_xml)
        .map_err(|e| Error::Urdf(format!("failed to parse URDF: {e}")))?;

    let joints = robot
        .joints
        .into_iter()
        .filter(|j| !matches!(j.joint_type, urdf_rs::JointType::Fixed))
        .map(|j| {
            let joint_type = match j.joint_type {
                urdf_rs::JointType::Revolute => JointType::Revolute,
                urdf_rs::JointType::Prismatic => JointType::Prismatic,
                urdf_rs::JointType::Continuous => JointType::Continuous,
                urdf_rs::JointType::Floating => JointType::Floating,
                urdf_rs::JointType::Planar => JointType::Planar,
                urdf_rs::JointType::Spherical => JointType::Revolute, // approximate
                urdf_rs::JointType::Fixed => unreachable!(),
            };

            let l = j.limit;
            let limits = if l.lower == 0.0 && l.upper == 0.0 && l.effort == 0.0 {
                None
            } else {
                Some(JointLimits {
                    lower: l.lower,
                    upper: l.upper,
                    effort: l.effort,
                    velocity: l.velocity,
                })
            };

            JointInfo {
                name: j.name,
                joint_type,
                parent_link: j.parent.link,
                child_link: j.child.link,
                limits,
            }
        })
        .collect();

    Ok(joints)
}
