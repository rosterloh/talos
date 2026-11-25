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
    let i = robot.links().iter().position(|l| l == link).unwrap();
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
    let twin = robot.links().iter().position(|l| l == "twin").unwrap();
    let expected = Mat4::from_rotation_z(0.5 - FRAC_PI_2);
    assert!(poses[twin].abs_diff_eq(expected, 1e-5));

    let movable: Vec<&str> = robot
        .joints()
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
fn chained_mimic_follows_driven_joint() {
    let limit = r#"<axis xyz="0 0 1"/><limit lower="-1" upper="1" effort="1" velocity="1"/>"#;
    let urdf = format!(
        r#"<robot name="g"><link name="base"/><link name="a"/><link name="b"/><link name="c"/>
      <joint name="drive" type="revolute"><parent link="base"/><child link="a"/>{limit}</joint>
      <joint name="finger_b" type="revolute"><parent link="base"/><child link="c"/>{limit}
        <mimic joint="finger_a" multiplier="2" offset="0.1"/></joint>
      <joint name="finger_a" type="revolute"><parent link="base"/><child link="b"/>{limit}
        <mimic joint="drive" multiplier="-1" offset="0.2"/></joint></robot>"#
    );
    let robot = Robot::from_urdf_str(&urdf).unwrap();
    let poses = robot.link_poses(&HashMap::from([("drive".to_owned(), 0.5)]));
    // finger_a = -0.5 + 0.2 = -0.3; finger_b = 2 * -0.3 + 0.1 = -0.5
    assert!(poses[2].abs_diff_eq(Mat4::from_rotation_z(-0.3), 1e-5));
    assert!(poses[3].abs_diff_eq(Mat4::from_rotation_z(-0.5), 1e-5));

    let cycle = urdf.replace(r#"joint="drive""#, r#"joint="finger_b""#);
    assert!(matches!(
        Robot::from_urdf_str(&cycle),
        Err(Error::MimicCycle(_))
    ));
}

#[test]
fn joints_keep_urdf_order() {
    let urdf = r#"<robot name="r"><link name="root"/><link name="A"/><link name="B"/>
      <link name="A1"/><link name="B1"/>
      <joint name="j3" type="fixed"><parent link="A"/><child link="A1"/><origin xyz="0 0 1"/></joint>
      <joint name="j1" type="fixed"><parent link="root"/><child link="A"/><origin xyz="1 0 0"/></joint>
      <joint name="j2" type="fixed"><parent link="root"/><child link="B"/><origin xyz="0 1 0"/></joint>
      <joint name="j4" type="fixed"><parent link="B"/><child link="B1"/><origin xyz="0 0 2"/></joint>
    </robot>"#;
    let robot = Robot::from_urdf_str(urdf).unwrap();
    let names: Vec<&str> = robot.joints().iter().map(|j| j.name.as_str()).collect();
    assert_eq!(names, ["j3", "j1", "j2", "j4"]);
    // FK still runs parent-first although j3 is declared before j1.
    let poses = robot.link_poses(&HashMap::new());
    assert_near(translation(&robot, &poses, "A1"), Vec3::new(1.0, 0.0, 1.0));
    assert_near(translation(&robot, &poses, "B1"), Vec3::new(0.0, 1.0, 2.0));
}

#[test]
fn materials_follow_urdfdom_precedence() {
    let sphere = r#"<geometry><sphere radius="1"/></geometry>"#;
    let urdf = format!(
        r#"<robot name="m"><material name="green"><color rgba="0 1 0 1"/></material>
      <material name="tex"><texture filename="t.png"/></material>
      <link name="a">
        <visual>{sphere}<material name="blue"/></visual>
        <visual>{sphere}<material name="red"><color rgba="1 0 0 1"/></material></visual>
        <visual>{sphere}<material name="green"><color rgba="0 0 1 1"/></material></visual>
        <visual>{sphere}<material name="tex"><color rgba="1 0 0 1"/></material></visual>
      </link>
      <link name="b"><visual>{sphere}<material name="red"/></visual>
        <visual>{sphere}<material name="blue"><color rgba="0 0 1 1"/></material></visual></link>
      <joint name="j" type="fixed"><parent link="a"/><child link="b"/></joint></robot>"#
    );
    let robot = Robot::from_urdf_str(&urdf).unwrap();
    let colors: Vec<[f32; 4]> = robot.visuals.iter().map(|v| v.color).collect();
    let (red, green, blue) = (
        [1.0, 0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0, 1.0],
        [0.0, 0.0, 1.0, 1.0],
    );
    // blue is referenced before it is defined; the texture-only global keeps "tex" uncoloured.
    assert_eq!(
        colors,
        [DEFAULT_COLOR, red, green, DEFAULT_COLOR, red, blue]
    );
}

#[test]
fn rejects_what_urdfdom_rejects() {
    let base = r#"<robot name="r"><link name="a"/><link name="b"/><link name="c"/>
      <joint name="j" type="prismatic"><parent link="a"/><child link="b"/>
        <limit lower="0" upper="1" effort="1" velocity="1"/></joint>
      <joint name="k" type="prismatic"><parent link="a"/><child link="c"/>
        <limit lower="0" upper="1" effort="1" velocity="1"/></joint></robot>"#;
    assert!(Robot::from_urdf_str(base).is_ok());
    let dup_joint = base.replace(r#"name="k""#, r#"name="j""#);
    assert!(matches!(
        Robot::from_urdf_str(&dup_joint),
        Err(Error::Duplicate { kind: "joint", .. })
    ));
    let dup_material = base.replace(
        r#"<link name="a"/>"#,
        r#"<material name="m"/><material name="m"/><link name="a"/>"#,
    );
    assert!(matches!(
        Robot::from_urdf_str(&dup_material),
        Err(Error::Duplicate {
            kind: "material",
            ..
        })
    ));
    let bad_mimic = base.replace(
        "</joint></robot>",
        r#"<mimic joint="missing"/></joint></robot>"#,
    );
    assert!(matches!(
        Robot::from_urdf_str(&bad_mimic),
        Err(Error::UnknownMimic { .. })
    ));
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

#[test]
fn limit_effort_and_velocity_default_like_urdfdom() {
    let joint = |limit: &str| {
        format!(
            r#"<robot name="r"><link name="a"/><link name="b"/>
          <joint name="j" type="prismatic"><parent link="a"/><child link="b"/>{limit}</joint></robot>"#
        )
    };
    for limit in [
        r#"<limit lower="0.1" upper="1"/>"#,
        r#"<limit lower="0.1" upper="1" effort="5"/>"#,
        r#"<limit velocity = "2" lower="0.1" upper="1"></limit>"#,
        "<limit\n lower=\"0.1\"\n upper=\"1\" effort=\"1\" velocity=\"1\" />",
    ] {
        let robot = Robot::from_urdf_str(&joint(limit)).unwrap_or_else(|e| panic!("{limit}: {e}"));
        let j = &robot.joints()[0];
        assert_eq!((j.lower, j.upper), (0.1, 1.0), "{limit}");
    }
    // Present attributes are kept, not duplicated.
    let kept = default_limit_attrs(r#"<limit effort="5" velocity="2"/>"#);
    assert_eq!(kept, r#"<limit effort="5" velocity="2"/>"#);
}

#[test]
fn limit_defaults_leave_comments_alone() {
    let urdf = r#"<robot name="r"><!-- no <limit --><link name="a"/></robot>"#;
    assert!(Robot::from_urdf_str(urdf).is_ok());
    assert_eq!(default_limit_attrs(urdf), urdf);
}

#[test]
fn inline_texture_only_material_keeps_its_name() {
    let sphere = r#"<geometry><sphere radius="1"/></geometry>"#;
    let urdf = format!(
        r#"<robot name="m"><link name="a">
        <visual>{sphere}<material name="surface"><texture filename="t.png"/></material></visual>
        <visual>{sphere}<material name="surface"><color rgba="1 0 0 1"/></material></visual>
        <visual>{sphere}<material name="surface"/></visual></link></robot>"#
    );
    let robot = Robot::from_urdf_str(&urdf).unwrap();
    assert!(robot.visuals.iter().all(|v| v.color == DEFAULT_COLOR));
}
