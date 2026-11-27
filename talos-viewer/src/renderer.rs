//! Offscreen wgpu renderer: draws a [`Robot`] into a texture Slint can display.

use std::collections::HashMap;

use slint::wgpu_30::wgpu;
use talos_urdf::{Mat4, Robot, TriMesh, Vec3};
use wgpu::util::DeviceExt;

const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// Dynamic uniform offsets must be multiples of 256 on every backend.
const DRAW_STRIDE: u64 = 256;
const GRID_HALF_CELLS: i32 = 10;
const GRID_CELL_M: f32 = 0.5;
/// Camera distance range in metres; the clip planes scale with the distance.
pub const MIN_DISTANCE: f32 = 0.05;
pub const MAX_DISTANCE: f32 = 1000.0;

/// Z-up orbit camera, matching ROS conventions.
pub struct OrbitCamera {
    pub target: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

impl OrbitCamera {
    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        let eye = self.target + self.distance * Vec3::new(cp * cy, cp * sy, sp);
        use talos_urdf::glam::camera::rh::{proj::directx::perspective, view::look_at_mat4};
        // Keep 45° on the narrower axis, so a tall, thin viewport does not clip the sides.
        let half = 22.5f32.to_radians().tan();
        let fov_y = 2.0
            * if aspect < 1.0 {
                (half / aspect).atan()
            } else {
                half.atan()
            };
        // WebGPU clip space has the same 0..1 depth range as DirectX.
        // A framed robot sits within distance / 3 of the target, so 3x the distance reaches it all.
        let (near, far) = (
            (self.distance * 0.002).max(0.01),
            (self.distance * 3.0).max(100.0),
        );
        perspective(fov_y, aspect, near, far) * look_at_mat4(eye, self.target, Vec3::Z)
    }

    /// Orbit by pixel deltas from a pointer drag.
    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.yaw -= dx * 0.01;
        self.pitch = (self.pitch + dy * 0.01).clamp(-1.55, 1.55);
    }

    pub fn zoom(&mut self, scroll: f32) {
        self.distance = (self.distance * (-scroll * 0.002).exp()).clamp(MIN_DISTANCE, MAX_DISTANCE);
    }
}

struct GpuMesh {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    triangles: wgpu::RenderPipeline,
    lines: wgpu::RenderPipeline,
    camera: wgpu::Buffer,
    camera_group: wgpu::BindGroup,
    draws: wgpu::Buffer,
    draw_group: wgpu::BindGroup,
    /// The robot the buffers below were sized for, so `render` cannot mismatch them.
    robot: Robot,
    /// One per robot visual; `None` for meshes.
    // shortcut: mesh visuals are not drawn, load STL/DAE/OBJ when live /robot_description needs real robots.
    meshes: Vec<Option<GpuMesh>>,
    grid: (wgpu::Buffer, u32),
    target: Option<(wgpu::Texture, wgpu::TextureView)>,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Draw {
    model: [f32; 16],
    color: [f32; 4],
}

fn vertex_bytes(mesh: &TriMesh) -> Vec<[f32; 6]> {
    mesh.positions
        .iter()
        .zip(&mesh.normals)
        .map(|(p, n)| [p[0], p[1], p[2], n[0], n[1], n[2]])
        .collect()
}

fn uniform_layout(device: &wgpu::Device, dynamic: bool) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: dynamic,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

impl Renderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, robot: &Robot) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        let camera_layout = uniform_layout(device, false);
        let draw_layout = uniform_layout(device, true);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&camera_layout), Some(&draw_layout)],
            immediate_size: 0,
        });
        let pipeline = |topology| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: 24,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                    })],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(COLOR_FORMAT.into())],
                }),
                primitive: wgpu::PrimitiveState {
                    topology,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            })
        };

        let camera = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let draws = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            // +1 for the grid.
            size: DRAW_STRIDE * (robot.visuals.len() as u64 + 1),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let group = |layout, buffer: &wgpu::Buffer, size| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer,
                        offset: 0,
                        size: wgpu::BufferSize::new(size),
                    }),
                }],
            })
        };

        let upload = |mesh: &TriMesh| GpuMesh {
            vertices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&vertex_bytes(mesh)),
                usage: wgpu::BufferUsages::VERTEX,
            }),
            indices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            }),
            count: mesh.indices.len() as u32,
        };

        // One segment per cell: on lavapipe, long lines crossing the near plane rasterized as filled shapes.
        let cells = (-GRID_HALF_CELLS..GRID_HALF_CELLS).map(|i| i as f32 * GRID_CELL_M);
        let grid: Vec<[f32; 6]> = (-GRID_HALF_CELLS..=GRID_HALF_CELLS)
            .flat_map(|i| cells.clone().map(move |a| (i as f32 * GRID_CELL_M, a)))
            .flat_map(|(c, a)| {
                let b = a + GRID_CELL_M;
                [
                    [c, a, 0.0, 0.0, 0.0, 1.0],
                    [c, b, 0.0, 0.0, 0.0, 1.0],
                    [a, c, 0.0, 0.0, 0.0, 1.0],
                    [b, c, 0.0, 0.0, 0.0, 1.0],
                ]
            })
            .collect();

        Self {
            triangles: pipeline(wgpu::PrimitiveTopology::TriangleList),
            lines: pipeline(wgpu::PrimitiveTopology::LineList),
            camera_group: group(&camera_layout, &camera, 64),
            draw_group: group(&draw_layout, &draws, size_of::<Draw>() as u64),
            camera,
            draws,
            meshes: robot
                .visuals
                .iter()
                .map(|v| v.shape.tessellate().map(|m| upload(&m)))
                .collect(),
            grid: (
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&grid),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
                grid.len() as u32,
            ),
            target: None,
            robot: robot.clone(),
            device: device.clone(),
            queue: queue.clone(),
        }
    }

    /// Render the robot at joint `positions`, keyed by joint name.
    pub fn render(
        &mut self,
        positions: &HashMap<String, f64>,
        camera: &OrbitCamera,
        width: u32,
        height: u32,
    ) -> wgpu::Texture {
        let (width, height) = (width.max(1), height.max(1));
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        if self.target.as_ref().is_none_or(|(t, _)| t.size() != size) {
            let color = self.device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: COLOR_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let depth = self.device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.target = Some((color, depth.create_view(&Default::default())));
        }
        let (color, depth) = self.target.as_ref().expect("target created above");

        let view_proj = camera.view_proj(width as f32 / height as f32);
        self.queue.write_buffer(
            &self.camera,
            0,
            bytemuck::bytes_of(&view_proj.to_cols_array()),
        );

        let mut draws = vec![0u8; self.draws.size() as usize];
        let mut put = |slot: usize, model: Mat4, color: [f32; 4]| {
            let at = slot * DRAW_STRIDE as usize;
            let draw = Draw {
                model: model.to_cols_array(),
                color,
            };
            draws[at..at + size_of::<Draw>()].copy_from_slice(bytemuck::bytes_of(&draw));
        };
        put(0, Mat4::IDENTITY, [0.45, 0.45, 0.5, 1.0]);
        let link_poses = self.robot.link_poses(positions);
        for (i, v) in self.robot.visuals.iter().enumerate() {
            put(i + 1, link_poses[v.link] * v.origin, v.color);
        }
        self.queue.write_buffer(&self.draws, 0, &draws);

        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &color.create_view(&Default::default()),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.12,
                            g: 0.12,
                            b: 0.14,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.camera_group, &[]);

            pass.set_pipeline(&self.lines);
            pass.set_bind_group(1, &self.draw_group, &[0]);
            pass.set_vertex_buffer(0, self.grid.0.slice(..));
            pass.draw(0..self.grid.1, 0..1);

            pass.set_pipeline(&self.triangles);
            for (i, mesh) in self.meshes.iter().enumerate() {
                let Some(mesh) = mesh else { continue };
                pass.set_bind_group(
                    1,
                    &self.draw_group,
                    &[(i as u64 + 1) as u32 * DRAW_STRIDE as u32],
                );
                pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.count, 0, 0..1);
            }
        }
        self.queue.submit(Some(encoder.finish()));
        color.clone()
    }
}
