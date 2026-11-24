use std::collections::HashMap;
use std::f32::consts::FRAC_PI_2;

use super::*;

const ARM: &str = r#"
<robot name="arm">
  <material name="blue"><color rgba="0 0 1 1"/></material>
  <link name="base"><visual><geometry><box size="1 1 1"/></geometry><material name="blue"/></visual></link>
  <link name="upper"><visual><geometry><cylinder radius="0.1" length="1"/></geometry></visual></link>
  <link name="slider"/>
  <link name="twin"/>
  <joint name="shoulder" type="revolute">
    <parent link="base"/><child link="upper"/>
    <origin xyz="0 0 1" rpy="0 0 0"/><axis xyz="0 0 1"/>
    <limit lower="-1" upper="1" effort="1" velocity="1"/>
  </joint>
  <joint name="extend" type="prismatic">
    <parent link="upper"/><child link="slider"/>
    <origin xyz="1 0 0" rpy="0 0 0"/><axis xyz="1 0 0"/>
    <limit lower="0" upper="0.5" effort="1" velocity="1"/>
  </joint>
  <joint name="mirror" type="revolute">
    <parent link="base"/><child link="twin"/>
    <axis xyz="0 0 1"/><mimic joint="shoulder" multiplier="-1" offset="0.5"/>
    <limit lower="-1" upper="1" effort="1" velocity="1"/>
  </joint>
</robot>"#;

fn translation(robot: &Robot, poses: &[Mat4], link: &str) -> Vec3 {
    let i = robot.links.iter().position(|l| l == link).unwrap();
    poses[i].w_axis.truncate()
}

fn assert_near(a: Vec3, b: Vec3) {
    assert!(a.abs_diff_eq(b, 1e-5), "{a} != {b}");
}

#[test]
fn forward_kinematics_by_joint_name() {
    let robot = Robot::from_urdf_str(ARM).unwrap();
    assert_eq!(robot.root(), "base");

    let zero = robot.link_poses(&HashMap::new());
    assert_near(
        translation(&robot, &zero, "slider"),
        Vec3::new(1.0, 0.0, 1.0),
    );

    let q = HashMap::from([
        ("shoulder".to_owned(), FRAC_PI_2 as f64),
        ("extend".to_owned(), 0.5),
    ]);
    let poses = robot.link_poses(&q);
    // Shoulder turns +90° about Z, so the 1.5 m reach now points along +Y.
    assert_near(
        translation(&robot, &poses, "slider"),
        Vec3::new(0.0, 1.5, 1.0),
    );

    // mirror = -1 * shoulder + 0.5
    let twin = robot.links.iter().position(|l| l == "twin").unwrap();
    let expected = Mat4::from_rotation_z(0.5 - FRAC_PI_2);
    assert!(poses[twin].abs_diff_eq(expected, 1e-5));

    let movable: Vec<&str> = robot
        .joints
        .iter()
        .filter(|j| j.is_movable())
        .map(|j| j.name.as_str())
        .collect();
    assert_eq!(movable.len(), 2);
    assert!(movable.contains(&"shoulder") && movable.contains(&"extend"));
}

#[test]
fn origin_rpy_is_fixed_axis_xyz() {
    let urdf = r#"<robot name="r"><link name="a"/><link name="b"/>
      <joint name="j" type="fixed"><parent link="a"/><child link="b"/>
        <origin xyz="0 0 0" rpy="1.5707963 1.5707963 0"/></joint></robot>"#;
    let robot = Robot::from_urdf_str(urdf).unwrap();
    let b = robot.link_poses(&HashMap::new())[1];
    // Rz(0) * Ry(90°) * Rx(90°): local Y maps to world X, local Z to world -Y.
    assert_near(b.transform_vector3(Vec3::Y), Vec3::X);
    assert_near(b.transform_vector3(Vec3::Z), Vec3::NEG_Y);
}

#[test]
fn materials_resolve_by_name_and_inline() {
    let robot = Robot::from_urdf_str(ARM).unwrap();
    assert_eq!(robot.visuals[0].color, [0.0, 0.0, 1.0, 1.0]);
    assert_eq!(robot.visuals[1].color, DEFAULT_COLOR);
}

#[test]
fn rejects_two_roots() {
    let urdf = r#"<robot name="r"><link name="a"/><link name="b"/></robot>"#;
    assert!(matches!(
        Robot::from_urdf_str(urdf),
        Err(Error::NotATree(_))
    ));
}

#[test]
fn primitives_wind_outward() {
    for shape in [
        Shape::Box {
            size: Vec3::new(1.0, 2.0, 3.0),
        },
        Shape::Cylinder {
            radius: 0.5,
            length: 2.0,
        },
        Shape::Sphere { radius: 1.0 },
    ] {
        let m = shape.tessellate().unwrap();
        assert_eq!(m.positions.len(), m.normals.len());
        for t in m.indices.chunks(3) {
            let [a, b, c] = [0, 1, 2].map(|k| Vec3::from(m.positions[t[k] as usize]));
            let n = Vec3::from(m.normals[t[0] as usize]);
            let face = (b - a).cross(c - a);
            // Degenerate sphere-pole triangles have zero area.
            assert!(
                face.dot(n) >= -1e-6,
                "{shape:?} triangle {t:?} faces inward"
            );
        }
    }
    assert!(
        Shape::Mesh {
            filename: "package://x.stl".into(),
            scale: Vec3::ONE
        }
        .tessellate()
        .is_none()
    );
}
