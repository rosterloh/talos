use super::protocol::types::JointType;
use super::urdf::extract_joints;

const SAMPLE_URDF: &str = r#"<?xml version="1.0"?>
<robot name="test_robot">
  <link name="base_link"/>
  <link name="shoulder_link"/>
  <link name="elbow_link"/>
  <link name="fixed_link"/>

  <joint name="shoulder_pan" type="revolute">
    <parent link="base_link"/>
    <child link="shoulder_link"/>
    <axis xyz="0 0 1"/>
    <limit lower="-3.14" upper="3.14" effort="100" velocity="1.0"/>
  </joint>

  <joint name="elbow" type="revolute">
    <parent link="shoulder_link"/>
    <child link="elbow_link"/>
    <axis xyz="0 1 0"/>
    <limit lower="-1.57" upper="1.57" effort="50" velocity="0.5"/>
  </joint>

  <joint name="fixed_mount" type="fixed">
    <parent link="base_link"/>
    <child link="fixed_link"/>
  </joint>
</robot>"#;

#[test]
fn extracts_non_fixed_joints() {
    let joints = extract_joints(SAMPLE_URDF).unwrap();
    assert_eq!(joints.len(), 2);
}

#[test]
fn joint_properties() {
    let joints = extract_joints(SAMPLE_URDF).unwrap();
    let shoulder = &joints[0];
    assert_eq!(shoulder.name, "shoulder_pan");
    assert!(matches!(shoulder.joint_type, JointType::Revolute));
    assert_eq!(shoulder.parent_link, "base_link");
    assert_eq!(shoulder.child_link, "shoulder_link");

    let limits = shoulder.limits.as_ref().unwrap();
    assert!((limits.lower - (-3.14)).abs() < f64::EPSILON);
    assert!((limits.upper - 3.14).abs() < f64::EPSILON);
    assert!((limits.effort - 100.0).abs() < f64::EPSILON);
    assert!((limits.velocity - 1.0).abs() < f64::EPSILON);
}

#[test]
fn invalid_urdf() {
    let result = extract_joints("not xml at all");
    assert!(result.is_err());
}
