use std::collections::HashMap;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use slint::wgpu_30::wgpu;
use talos_urdf::Robot;

use crate::renderer::Renderer;
use crate::{initial_camera, initial_positions, sliders};

/// wgpu's native futures complete on first poll, so no executor is needed.
fn block_on<F: Future>(f: F) -> F::Output {
    let mut f = pin!(f);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
            return v;
        }
    }
}

fn gpu() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let adapter = block_on(instance.request_adapter(&Default::default())).ok()?;
    block_on(adapter.request_device(&Default::default())).ok()
}

fn pixel(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    x: u32,
    y: u32,
) -> [u8; 4] {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d { x, y, z: 0 },
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: None,
                rows_per_image: None,
            },
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, |r| r.unwrap());
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let px = buffer.slice(..4).get_mapped_range().unwrap();
    [px[0], px[1], px[2], px[3]]
}

#[test]
#[ignore = "needs a wgpu adapter: cargo test -p talos-viewer -- --ignored"]
fn renders_robot_over_background() {
    let (device, queue) = gpu().expect("no wgpu adapter");
    let robot: Robot = crate::demo_robot();
    let mut renderer = Renderer::new(&device, &queue, &robot);
    let positions = initial_positions(&robot);
    let camera = initial_camera(&robot, &positions);
    let texture = renderer.render(&positions, &camera, 200, 200);

    let corner = pixel(&device, &queue, &texture, 0, 0);
    let centre = pixel(&device, &queue, &texture, 100, 100);
    assert_ne!(corner, centre, "robot not drawn at viewport centre");
}

const LIMIT: &str = r#"effort="1" velocity="1""#;

#[test]
fn sliders_start_at_the_drawn_pose() {
    let urdf = format!(
        r#"<robot name="r"><link name="a"/><link name="b"/><link name="c"/><link name="d"/>
      <joint name="offset" type="revolute"><parent link="a"/><child link="b"/>
        <limit lower="0.2" upper="1" {LIMIT}/></joint>
      <joint name="locked" type="revolute"><parent link="a"/><child link="c"/>
        <limit lower="0.3" upper="0.3" {LIMIT}/></joint>
      <joint name="spin" type="continuous"><parent link="a"/><child link="d"/></joint></robot>"#
    );
    let robot = Robot::from_urdf_str(&urdf).unwrap();
    let positions = initial_positions(&robot);
    assert_eq!(
        positions,
        HashMap::from([
            ("offset".to_owned(), 0.2),
            ("locked".to_owned(), 0.3),
            ("spin".to_owned(), 0.0),
        ])
    );
    let sliders = sliders(&robot, &positions);
    let rows: Vec<(&str, f32, f32, f32)> = sliders
        .iter()
        .map(|s| (s.name.as_str(), s.value, s.minimum, s.maximum))
        .collect();
    let pi = std::f32::consts::PI;
    assert_eq!(rows, [("offset", 0.2, 0.2, 1.0), ("spin", 0.0, -pi, pi)]);
}

#[test]
fn camera_starts_outside_large_visuals() {
    let robot = Robot::from_urdf_str(
        r#"<robot name="r"><link name="base"><visual>
          <geometry><box size="3 3 1"/></geometry></visual></link></robot>"#,
    )
    .unwrap();
    let camera = initial_camera(&robot, &HashMap::new());
    // Half the box diagonal is about 2.18 m.
    assert!(camera.distance > 2.2, "distance {}", camera.distance);
}
