use std::mem;

use bytemuck::{Pod, Zeroable};
use wgpu::{
    util::DeviceExt, Buffer, ColorWrites, CommandEncoder, Device, TextureFormat, TextureView,
};

use crate::{App, ShaderContext};

pub struct Renderer {
    render_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
}

#[derive(Copy, Clone)]
struct Vertex {
    _position: [f32; 4],
    _tex_coords: [f32; 2],
}

impl Renderer {
    pub fn new(device: &Device, size: u32) -> Self {
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
            contents: bytemuck::cast_slice(index_data),
            usage: wgpu::BufferUsages::INDEX,
        });

        let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/render.wgsl").into()),
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
                        min_binding_size: wgpu::BufferSize::new(
                            mem::size_of::<ShaderContext>() as _
                        ),
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

        Self {
            render_pipeline,
            bind_group_layout,
            vertex_buf,
            index_buf,
        }
    }

    pub fn render(
        &self,
        app: &App,
        encoder: &mut CommandEncoder,
        context_buffer: &Buffer,
        view: &TextureView,
    ) {
        let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: context_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: app.simulation.get_state().as_entire_binding(),
                },
            ],
            label: None,
        });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,

            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
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
