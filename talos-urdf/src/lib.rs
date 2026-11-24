//! Renderer-agnostic URDF scene for 3D viewers.
//!
//! Parses a URDF into a kinematic tree, computes world poses of every link
//! from joint positions keyed by joint name, and tessellates primitive visual
//! geometry. No GPU, windowing or ROS 2 dependency, so any frontend can use it.

use std::collections::HashMap;
use std::f32::consts::TAU;

pub use glam::{self, Mat4, Vec3};
use glam::{EulerRot, Quat};

#[cfg(test)]
mod tests;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to parse URDF: {0}")]
    Parse(#[from] urdf_rs::UrdfError),
    #[error("joint {joint} references unknown link {link}")]
    UnknownLink { joint: String, link: String },
    #[error("URDF is not a single tree: {0}")]
    NotATree(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JointKind {
    Fixed,
    Revolute,
    Continuous,
    Prismatic,
    /// Floating, planar and spherical joints: drawn at their origin, not moved.
    Unsupported,
}

#[derive(Debug, Clone)]
pub struct Joint {
    pub name: String,
    pub kind: JointKind,
    pub parent: usize,
    pub child: usize,
    origin: Mat4,
    axis: Vec3,
    pub lower: f64,
    pub upper: f64,
    /// (mimicked joint, multiplier, offset)
    mimic: Option<(String, f64, f64)>,
}

impl Joint {
    /// True for joints a user or `/joint_states` can drive.
    pub fn is_movable(&self) -> bool {
        matches!(
            self.kind,
            JointKind::Revolute | JointKind::Continuous | JointKind::Prismatic
        ) && self.mimic.is_none()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Box {
        size: Vec3,
    },
    /// Along the local Z axis, centred on the origin.
    Cylinder {
        radius: f32,
        length: f32,
    },
    Sphere {
        radius: f32,
    },
    /// Unresolved mesh file (`package://`, `file://`, ...).
    Mesh {
        filename: String,
        scale: Vec3,
    },
}

#[derive(Debug, Clone)]
pub struct Visual {
    pub link: usize,
    /// Pose of the geometry in its link frame.
    pub origin: Mat4,
    pub shape: Shape,
    /// Linear RGBA.
    pub color: [f32; 4],
}

#[derive(Debug, Clone)]
pub struct Robot {
    pub name: String,
    pub links: Vec<String>,
    /// In topological order: a joint's parent link is placed before it is used.
    pub joints: Vec<Joint>,
    pub visuals: Vec<Visual>,
    root: usize,
}

const DEFAULT_COLOR: [f32; 4] = [0.7, 0.7, 0.7, 1.0];

fn pose(p: &urdf_rs::Pose) -> Mat4 {
    let [r, pi, y] = p.rpy.map(|v| v as f32);
    // URDF rpy is fixed-axis XYZ, i.e. R = Rz(yaw) * Ry(pitch) * Rx(roll).
    Mat4::from_rotation_translation(
        Quat::from_euler(EulerRot::ZYX, y, pi, r),
        Vec3::from(p.xyz.map(|v| v as f32)),
    )
}

fn vec3(v: &urdf_rs::Vec3) -> Vec3 {
    Vec3::from(v.map(|c| c as f32))
}

impl Robot {
    pub fn from_urdf_str(xml: &str) -> Result<Self, Error> {
        let urdf = urdf_rs::read_from_string(xml)?;
        Self::from_urdf(&urdf)
    }

    pub fn from_urdf(urdf: &urdf_rs::Robot) -> Result<Self, Error> {
        let links: Vec<String> = urdf.links.iter().map(|l| l.name.clone()).collect();
        let index: HashMap<&str, usize> = links
            .iter()
            .enumerate()
            .map(|(i, n)| (n.as_str(), i))
            .collect();
        let find = |joint: &str, link: &str| {
            index.get(link).copied().ok_or_else(|| Error::UnknownLink {
                joint: joint.to_owned(),
                link: link.to_owned(),
            })
        };

        let mut joints = Vec::with_capacity(urdf.joints.len());
        let mut has_parent = vec![false; links.len()];
        for j in &urdf.joints {
            let parent = find(&j.name, &j.parent.link)?;
            let child = find(&j.name, &j.child.link)?;
            if std::mem::replace(&mut has_parent[child], true) {
                return Err(Error::NotATree(format!(
                    "link {} has more than one parent joint",
                    links[child]
                )));
            }
            let kind = match j.joint_type {
                urdf_rs::JointType::Fixed => JointKind::Fixed,
                urdf_rs::JointType::Revolute => JointKind::Revolute,
                urdf_rs::JointType::Continuous => JointKind::Continuous,
                urdf_rs::JointType::Prismatic => JointKind::Prismatic,
                _ => JointKind::Unsupported,
            };
            joints.push(Joint {
                name: j.name.clone(),
                kind,
                parent,
                child,
                origin: pose(&j.origin),
                axis: vec3(&j.axis.xyz).normalize_or(Vec3::X),
                lower: j.limit.lower,
                upper: j.limit.upper,
                mimic: j.mimic.as_ref().map(|m| {
                    (
                        m.joint.clone(),
                        m.multiplier.unwrap_or(1.0),
                        m.offset.unwrap_or(0.0),
                    )
                }),
            });
        }

        let roots: Vec<usize> = (0..links.len()).filter(|&i| !has_parent[i]).collect();
        let [root] = roots[..] else {
            let names: Vec<&str> = roots.iter().map(|&i| links[i].as_str()).collect();
            return Err(Error::NotATree(format!("root links: {names:?}")));
        };

        // Order joints parent-first so poses can be filled in a single pass.
        let mut ordered = Vec::with_capacity(joints.len());
        let mut frontier = vec![root];
        while let Some(link) = frontier.pop() {
            for j in joints.iter().filter(|j| j.parent == link) {
                frontier.push(j.child);
                ordered.push(j.clone());
            }
        }
        if ordered.len() != joints.len() {
            return Err(Error::NotATree("cycle in joint graph".into()));
        }

        let materials: HashMap<&str, [f32; 4]> = urdf
            .materials
            .iter()
            .filter_map(|m| Some((m.name.as_str(), m.color.as_ref()?.rgba.map(|c| c as f32))))
            .collect();
        let visuals = urdf
            .links
            .iter()
            .enumerate()
            .flat_map(|(link, l)| l.visual.iter().map(move |v| (link, v)))
            .map(|(link, v)| Visual {
                link,
                origin: pose(&v.origin),
                shape: match &v.geometry {
                    urdf_rs::Geometry::Box { size } => Shape::Box { size: vec3(size) },
                    // shortcut: capsule drawn as a cylinder, add end caps if a robot needs them.
                    urdf_rs::Geometry::Cylinder { radius, length }
                    | urdf_rs::Geometry::Capsule { radius, length } => Shape::Cylinder {
                        radius: *radius as f32,
                        length: *length as f32,
                    },
                    urdf_rs::Geometry::Sphere { radius } => Shape::Sphere {
                        radius: *radius as f32,
                    },
                    urdf_rs::Geometry::Mesh { filename, scale } => Shape::Mesh {
                        filename: filename.clone(),
                        scale: scale.as_ref().map_or(Vec3::ONE, vec3),
                    },
                },
                color: v
                    .material
                    .as_ref()
                    .and_then(|m| {
                        m.color
                            .as_ref()
                            .map(|c| c.rgba.map(|x| x as f32))
                            .or_else(|| materials.get(m.name.as_str()).copied())
                    })
                    .unwrap_or(DEFAULT_COLOR),
            })
            .collect();

        Ok(Self {
            name: urdf.name.clone(),
            links,
            joints: ordered,
            visuals,
            root,
        })
    }

    pub fn root(&self) -> &str {
        &self.links[self.root]
    }

    /// World pose of every link (indexed like [`Robot::links`]), root at identity.
    ///
    /// `positions` is keyed by joint name, as in `sensor_msgs/JointState`.
    /// Missing joints sit at zero; mimic joints follow their source joint.
    pub fn link_poses(&self, positions: &HashMap<String, f64>) -> Vec<Mat4> {
        let mut world = vec![Mat4::IDENTITY; self.links.len()];
        for j in &self.joints {
            let q = match &j.mimic {
                Some((src, mul, off)) => positions.get(src).copied().unwrap_or(0.0) * mul + off,
                None => positions.get(&j.name).copied().unwrap_or(0.0),
            } as f32;
            let motion = match j.kind {
                JointKind::Revolute | JointKind::Continuous => Mat4::from_axis_angle(j.axis, q),
                JointKind::Prismatic => Mat4::from_translation(j.axis * q),
                JointKind::Fixed | JointKind::Unsupported => Mat4::IDENTITY,
            };
            world[j.child] = world[j.parent] * j.origin * motion;
        }
        world
    }
}

/// Indexed triangle mesh with per-vertex normals.
#[derive(Debug, Clone, Default)]
pub struct TriMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

impl TriMesh {
    fn push(&mut self, p: Vec3, n: Vec3) -> u32 {
        self.positions.push(p.into());
        self.normals.push(n.into());
        self.positions.len() as u32 - 1
    }
}

const SEGMENTS: u32 = 32;

impl Shape {
    /// Triangles for primitive shapes; `None` for meshes, which need loading.
    pub fn tessellate(&self) -> Option<TriMesh> {
        let mut m = TriMesh::default();
        match *self {
            Shape::Box { size } => {
                let h = size / 2.0;
                for n in [
                    Vec3::X,
                    Vec3::NEG_X,
                    Vec3::Y,
                    Vec3::NEG_Y,
                    Vec3::Z,
                    Vec3::NEG_Z,
                ] {
                    let (u, v) = n.any_orthonormal_pair();
                    let base = m.positions.len() as u32;
                    for (a, b) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                        m.push((n + u * a + v * b) * h, n);
                    }
                    // any_orthonormal_pair gives a right-handed (u, v, n) basis, so this is CCW.
                    m.indices
                        .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
                }
            }
            Shape::Cylinder { radius, length } => {
                let h = length / 2.0;
                for cap in [h, -h] {
                    let n = Vec3::Z * cap.signum();
                    let centre = m.push(Vec3::new(0.0, 0.0, cap), n);
                    for i in 0..SEGMENTS {
                        let a = TAU * i as f32 / SEGMENTS as f32;
                        m.push(Vec3::new(radius * a.cos(), radius * a.sin(), cap), n);
                    }
                    for i in 0..SEGMENTS {
                        let (a, b) = (centre + 1 + i, centre + 1 + (i + 1) % SEGMENTS);
                        if cap > 0.0 {
                            m.indices.extend([centre, a, b]);
                        } else {
                            m.indices.extend([centre, b, a]);
                        }
                    }
                }
                let base = m.positions.len() as u32;
                for i in 0..SEGMENTS {
                    let a = TAU * i as f32 / SEGMENTS as f32;
                    let n = Vec3::new(a.cos(), a.sin(), 0.0);
                    m.push(n * radius + Vec3::Z * h, n);
                    m.push(n * radius - Vec3::Z * h, n);
                }
                for i in 0..SEGMENTS {
                    let (t0, b0) = (base + 2 * i, base + 2 * i + 1);
                    let j = (i + 1) % SEGMENTS;
                    let (t1, b1) = (base + 2 * j, base + 2 * j + 1);
                    m.indices.extend([b0, b1, t1, b0, t1, t0]);
                }
            }
            Shape::Sphere { radius } => {
                let rings = SEGMENTS / 2;
                for r in 0..=rings {
                    let theta = std::f32::consts::PI * r as f32 / rings as f32;
                    for s in 0..=SEGMENTS {
                        let phi = TAU * s as f32 / SEGMENTS as f32;
                        let n = Vec3::new(
                            theta.sin() * phi.cos(),
                            theta.sin() * phi.sin(),
                            theta.cos(),
                        );
                        m.push(n * radius, n);
                    }
                }
                let row = SEGMENTS + 1;
                for r in 0..rings {
                    for s in 0..SEGMENTS {
                        let (a, b) = (r * row + s, (r + 1) * row + s);
                        m.indices.extend([a, b, b + 1, a, b + 1, a + 1]);
                    }
                }
            }
            Shape::Mesh { .. } => return None,
        }
        Some(m)
    }
}
