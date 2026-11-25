# 3D Viewer

`talos-viewer` is an early, Slint-based 3D view of a URDF, in the spirit of
RViz's robot model display. It is standalone for now: it loads a URDF file and
lets you drive the joints with sliders. It does not connect to an agent yet.

```bash
cargo run -p talos-viewer                      # bundled demo arm
cargo run -p talos-viewer -- path/to/robot.urdf
```

A plain `cargo build` skips the viewer, so the robot build does not compile
the GUI stack; `-p talos-viewer` (as above) builds it.

Drag in the viewport to orbit and scroll to zoom. The view is Z-up, matching
ROS. It needs a GPU adapter through wgpu; on a machine without one, set
`SLINT_WGPU_CPU=1` to allow a software adapter such as lavapipe.

## Crates

- **`talos-urdf`**: renderer-agnostic core with no GPU, windowing or ROS 2
  dependency. It parses the URDF into a kinematic tree, computes link poses
  from joint positions keyed by joint name (as in `sensor_msgs/JointState`,
  including mimic joints), and tessellates box, cylinder and sphere visuals.
  Another frontend, such as Dioxus, can reuse it.
- **`talos-viewer`**: draws the scene with wgpu 30 into an offscreen texture,
  shown in Slint 1.19 through `unstable-wgpu-30` and
  `slint::Image::try_from(wgpu::Texture)`.

## Limitations

- Mesh visuals (`package://` STL, DAE, OBJ) are skipped with a warning.
- Capsules are drawn as cylinders.
- Translucent material colours are drawn opaque.
- Joints whose limits are equal, missing or non-finite stay fixed and get no
  slider.
- A `<limit>` must carry `effort` and `velocity`: the `urdf-rs` parser
  rejects the file otherwise, although urdfdom accepts it.
- Floating, planar and spherical joints stay at their origin.
- The render test needs a GPU adapter, so it is ignored by default; run it
  with `cargo test -p talos-viewer -- --ignored`.
- No live data: wiring `/robot_description` and `/joint_states` from an agent
  session is the next step.
