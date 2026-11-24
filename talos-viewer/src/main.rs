//! Slint 3D URDF viewer: orbit a robot model and drive its joints with sliders.
//!
//! Usage: `talos-viewer [path/to/robot.urdf]` (defaults to the bundled demo arm).

mod renderer;

use std::cell::RefCell;
use std::collections::HashMap;
use std::f64::consts::PI;
use std::rc::Rc;

use renderer::{OrbitCamera, Renderer};
use slint::wgpu_30::WGPUConfiguration;
use slint::{Model, VecModel};
use talos_urdf::{JointKind, Robot, Vec3};

slint::slint! {
    #[edition = "1.19"]
    import { Slider, ScrollView } from "std-widgets.slint";

    export struct JointSlider { name: string, value: float, minimum: float, maximum: float }

    export component App inherits Window {
        title: "Talos viewer";
        preferred-width: 1100px;
        preferred-height: 720px;

        in property <image> texture;
        out property <int> texture-width: view.width / 1phx;
        out property <int> texture-height: view.height / 1phx;
        in-out property <[JointSlider]> joints;
        callback joint-changed(int, float);
        callback orbit(float, float);
        callback zoom(float);

        HorizontalLayout {
            view := Rectangle {
                horizontal-stretch: 1;
                Image { source: root.texture; width: 100%; height: 100%; }
                TouchArea {
                    property <length> last-x;
                    property <length> last-y;
                    pointer-event(e) => {
                        if e.kind == PointerEventKind.down {
                            self.last-x = self.mouse-x;
                            self.last-y = self.mouse-y;
                        }
                    }
                    moved => {
                        root.orbit((self.mouse-x - self.last-x) / 1px, (self.mouse-y - self.last-y) / 1px);
                        self.last-x = self.mouse-x;
                        self.last-y = self.mouse-y;
                    }
                    scroll-event(e) => {
                        root.zoom(e.delta-y / 1px);
                        accept
                    }
                }
            }
            ScrollView {
                width: 300px;
                VerticalLayout {
                    padding: 12px;
                    spacing: 6px;
                    alignment: start;
                    for joint[i] in root.joints: VerticalLayout {
                        Text { text: joint.name + ": " + Math.round(joint.value * 1000) / 1000; }
                        Slider {
                            minimum: joint.minimum;
                            maximum: joint.maximum;
                            value: joint.value;
                            changed(v) => { root.joint-changed(i, v); }
                        }
                    }
                }
            }
        }
    }
}

struct State {
    robot: Robot,
    positions: HashMap<String, f64>,
    camera: OrbitCamera,
    renderer: Option<Renderer>,
}

fn sliders(robot: &Robot) -> Vec<JointSlider> {
    robot
        .joints
        .iter()
        .filter(|j| j.is_movable())
        .map(|j| {
            let (lo, hi) = if j.kind == JointKind::Continuous || j.lower >= j.upper {
                (-PI, PI)
            } else {
                (j.lower, j.upper)
            };
            JointSlider {
                name: j.name.as_str().into(),
                value: 0f64.clamp(lo, hi) as f32,
                minimum: lo as f32,
                maximum: hi as f32,
            }
        })
        .collect()
}

/// Frame the robot at its zero pose.
fn initial_camera(robot: &Robot) -> OrbitCamera {
    let origins: Vec<Vec3> = robot
        .link_poses(&HashMap::new())
        .iter()
        .map(|m| m.w_axis.truncate())
        .collect();
    let centre = origins.iter().sum::<Vec3>() / origins.len().max(1) as f32;
    let radius = origins
        .iter()
        .map(|p| p.distance(centre))
        .fold(0.3, f32::max);
    OrbitCamera {
        target: centre,
        yaw: 0.6,
        pitch: 0.35,
        distance: radius * 3.0,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let xml = match std::env::args().nth(1) {
        Some(path) => std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?,
        None => include_str!("../demo.urdf").to_owned(),
    };
    let robot = Robot::from_urdf_str(&xml)?;
    for v in &robot.visuals {
        if let talos_urdf::Shape::Mesh { filename, .. } = &v.shape {
            eprintln!("skipping mesh {filename}: mesh loading is not supported yet");
        }
    }

    slint::BackendSelector::new()
        .require_wgpu_30(WGPUConfiguration::default())
        .select()?;

    let app = App::new()?;
    let joints = Rc::new(VecModel::from(sliders(&robot)));
    app.set_joints(joints.clone().into());

    let state = Rc::new(RefCell::new(State {
        camera: initial_camera(&robot),
        positions: HashMap::new(),
        renderer: None,
        robot,
    }));

    let weak = app.as_weak();
    app.on_joint_changed({
        let state = state.clone();
        move |i, value| {
            let i = i as usize;
            let Some(mut row) = joints.row_data(i) else {
                return;
            };
            state
                .borrow_mut()
                .positions
                .insert(row.name.to_string(), value.into());
            row.value = value;
            joints.set_row_data(i, row);
        }
    });
    app.on_orbit({
        let (state, weak) = (state.clone(), weak.clone());
        move |dx, dy| {
            state.borrow_mut().camera.orbit(dx, dy);
            weak.upgrade().inspect(|app| app.window().request_redraw());
        }
    });
    app.on_zoom({
        let (state, weak) = (state.clone(), weak.clone());
        move |delta| {
            state.borrow_mut().camera.zoom(delta);
            weak.upgrade().inspect(|app| app.window().request_redraw());
        }
    });

    app.window().set_rendering_notifier(move |rendering, api| {
        let mut state = state.borrow_mut();
        let state = &mut *state;
        match (rendering, api) {
            (
                slint::RenderingState::RenderingSetup,
                slint::GraphicsAPI::WGPU30 { device, queue, .. },
            ) => {
                state.renderer = Some(Renderer::new(device, queue, &state.robot));
            }
            (slint::RenderingState::BeforeRendering, _) => {
                let (Some(renderer), Some(app)) = (state.renderer.as_mut(), weak.upgrade()) else {
                    return;
                };
                let poses = state.robot.link_poses(&state.positions);
                let texture = renderer.render(
                    &state.robot,
                    &poses,
                    &state.camera,
                    app.get_texture_width() as u32,
                    app.get_texture_height() as u32,
                );
                match slint::Image::try_from(texture) {
                    Ok(image) => app.set_texture(image),
                    Err(e) => eprintln!("cannot display viewport texture: {e}"),
                }
            }
            (slint::RenderingState::RenderingTeardown, _) => state.renderer = None,
            _ => {}
        }
    })?;

    app.run()?;
    Ok(())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
fn demo_robot() -> Robot {
    Robot::from_urdf_str(include_str!("../demo.urdf")).unwrap()
}
