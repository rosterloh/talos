use std::collections::HashMap;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use slint::wgpu_30::wgpu;
use talos_urdf::Robot;

use crate::initial_camera;
use crate::renderer::Renderer;

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
fn renders_robot_over_background() {
    let Some((device, queue)) = gpu() else {
        // shortcut: CI runners have no GPU adapter, so this only runs where one (or lavapipe) exists.
        eprintln!("no wgpu adapter, skipping");
        return;
    };
    let robot: Robot = crate::demo_robot();
    let mut renderer = Renderer::new(&device, &queue, &robot);
    let camera = initial_camera(&robot);
    let texture = renderer.render(
        &robot,
        &robot.link_poses(&HashMap::new()),
        &camera,
        200,
        200,
    );

    let corner = pixel(&device, &queue, &texture, 0, 0);
    let centre = pixel(&device, &queue, &texture, 100, 100);
    assert_ne!(corner, centre, "robot not drawn at viewport centre");
}
