use std::{mem, sync::Arc};

use anyhow::{Context, Result};
use bytemuck::{Pod, Zeroable};
use encase::ShaderType;
use nd_vec::{vector, Vec2};
use wgpu::{util::DeviceExt, ColorWrites, ShaderSource, TextureFormat, TextureUsages};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::EventLoop,
    window::{Window, WindowAttributes},
};

const SIZE: (u32, u32) = (1920, 1080);

struct App<'a> {
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'a>>,

    states: [wgpu::Buffer; 3],
    n: usize,

    instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
    compute_pipeline: wgpu::ComputePipeline,
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

#[derive(ShaderType)]
struct ShaderContext {
    width: u32,
    height: u32,
    tick: u32,
}

#[derive(Copy, Clone)]
struct Vertex {
    _position: [f32; 4],
    _tex_coords: [f32; 2],
}

impl ShaderContext {
    fn to_wgsl_bytes(&self) -> Vec<u8> {
        let mut buffer = encase::UniformBuffer::new(Vec::new());
        buffer.write(self).unwrap();
        buffer.into_inner()
    }
}

impl Vertex {
    const fn new(position: [f32; 4], tex_coords: [f32; 2]) -> Self {
        Self {
            _position: position,
            _tex_coords: tex_coords,
        }
    }
}

unsafe impl Zeroable for Vertex {}
unsafe impl Pod for Vertex {}

async fn run() -> Result<()> {
    let instance = wgpu::Instance::default();

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .context("No adapter found")?;

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
            },
            None,
        )
        .await
        .unwrap();

    let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
    });
    let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(include_str!("render.wgsl").into()),
    });

    let size = SIZE.0 * SIZE.1;
    let mut empty_buffer = vec![0f32; size as usize];
    let center = Vec2::new([SIZE.0 as f32 / 2.0, SIZE.1 as f32 / 2.0]);
    for y in 0..SIZE.1 {
        for x in 0..SIZE.0 {
            let pos = vector!(x as f32, y as f32);
            let idx = y * SIZE.0 + x;

            let dist = (center - pos).magnitude();
            empty_buffer[idx as usize] = 2.0 * (-dist).exp();
        }
    }

    let state_buffer_descriptor = wgpu::util::BufferInitDescriptor {
        label: Some("Storage Buffer"),
        contents: bytemuck::cast_slice(&empty_buffer),
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
    };
    let state_buffer_1 = device.create_buffer_init(&state_buffer_descriptor.clone());
    let state_buffer_2 = device.create_buffer_init(&state_buffer_descriptor.clone());
    let state_buffer_3 = device.create_buffer_init(&state_buffer_descriptor);

    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &compute_shader,
        entry_point: "main",
        compilation_options: Default::default(),
    });

    let vertex_size = mem::size_of::<Vertex>();
    let vertex_data = [
        Vertex::new([-1.0, -1.0, 1.0, 1.0], [0.0, 0.0]),
        Vertex::new([1.0, -1.0, 1.0, 1.0], [1.0, 0.0]),
        Vertex::new([1.0, 1.0, 1.0, 1.0], [1.0, 1.0]),
        Vertex::new([-1.0, 1.0, 1.0, 1.0], [0.0, 1.0]),
    ];

    let index_data: &[u16] = &[0, 1, 2, 2, 3, 0];

    let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&vertex_data),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&index_data),
        usage: wgpu::BufferUsages::INDEX,
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(mem::size_of::<ShaderContext>() as _),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(size as u64 * 4),
                },
                count: None,
            },
        ],
    });

    let vertex_buffers = [wgpu::VertexBufferLayout {
        array_stride: vertex_size as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: 0,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 4 * 4,
                shader_location: 1,
            },
        ],
    }];

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &render_shader,
            entry_point: "vert",
            buffers: &vertex_buffers,
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &render_shader,
            entry_point: "frag",
            targets: &[Some(wgpu::ColorTargetState {
                format: TextureFormat::Rgba8Unorm,
                blend: None,
                write_mask: ColorWrites::all(),
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: None,
        multiview: None,
        multisample: wgpu::MultisampleState::default(),
    });

    let event_loop = EventLoop::new()?;
    let mut app = App {
        window: None,
        surface: None,
        n: 0,

        instance,
        device,
        queue,
        states: [state_buffer_1, state_buffer_2, state_buffer_3],
        compute_pipeline,
        vertex_buf,
        index_buf,
        render_pipeline,
        bind_group_layout,
    };

    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    event_loop.run_app(&mut app)?;

    Ok(())
}

impl<'a> ApplicationHandler for App<'a> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes().with_inner_size(PhysicalSize::new(SIZE.0, SIZE.1)),
                )
                .unwrap(),
        );
        let surface = self.instance.create_surface(window.clone()).unwrap();

        let size = window.inner_size();
        surface.configure(
            &self.device,
            &wgpu::SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format: TextureFormat::Rgba8Unorm,
                width: size.width,
                height: size.height,
                present_mode: wgpu::PresentMode::Fifo,
                desired_maximum_frame_latency: 2,
                alpha_mode: wgpu::CompositeAlphaMode::Opaque,
                view_formats: vec![],
            },
        );

        self.window = Some(window);
        self.surface = Some(surface);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if event == WindowEvent::CloseRequested {
            event_loop.exit();
        } else if event == WindowEvent::RedrawRequested {
            self.n = self.n.wrapping_add(1);

            let ctx = ShaderContext {
                width: SIZE.0,
                height: SIZE.1,
                tick: self.n as u32,
            };

            let context_buffer =
                self.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        contents: &ctx.to_wgsl_bytes(),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });

            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            {
                let bind_group_layout = self.compute_pipeline.get_bind_group_layout(0);
                let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: context_buffer.as_entire_binding(),
                        },
                        // 1 => next, 2 => last, 3 => last2
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: self.states[self.n % 3].as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: self.states[(self.n + 2) % 3].as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: self.states[(self.n + 1) % 3].as_entire_binding(),
                        },
                    ],
                });

                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: None,
                    timestamp_writes: None,
                });
                cpass.set_pipeline(&self.compute_pipeline);
                cpass.set_bind_group(0, &bind_group, &[]);
                cpass.dispatch_workgroups(SIZE.0 / 8, SIZE.1 / 8, 1);
            }

            let output = self
                .surface
                .as_ref()
                .unwrap()
                .get_current_texture()
                .unwrap();
            let view = output
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            {
                let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &self.bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: context_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: self.states[self.n % 3].as_entire_binding(),
                        },
                    ],
                    label: None,
                });

                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,

                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_bind_group(0, &bind_group, &[]);
                render_pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
                render_pass.draw_indexed(0..6, 0, 0..1);
            }

            self.queue.submit(Some(encoder.finish()));
            output.present();

            self.window.as_ref().unwrap().request_redraw();
        }
    }
}

pub fn main() -> Result<()> {
    pollster::block_on(run())
}
