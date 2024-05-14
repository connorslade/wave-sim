use std::{fs, mem, path::Path};

use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use image::{ImageBuffer, Rgba};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferAddress, BufferBindingType, BufferSize,
    BufferUsages, ColorTargetState, ColorWrites, CommandEncoder, CommandEncoderDescriptor, Device,
    Extent3d, Face, FragmentState, ImageCopyBuffer, ImageCopyTexture, ImageDataLayout, IndexFormat,
    LoadOp, Maintain, MapMode, MultisampleState, Operations, Origin3d, PipelineLayoutDescriptor,
    PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderSource, ShaderStages, StoreOp,
    TextureAspect, TextureDescriptor, TextureDimension, TextureUsages, TextureView,
    TextureViewDescriptor, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState,
    VertexStepMode,
};
use winit::dpi::PhysicalSize;

use crate::{simulation::ShaderContext, App, TEXTURE_FORMAT};

pub struct Renderer {
    render_pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    vertex_buf: Buffer,
    index_buf: Buffer,
}

#[derive(Copy, Clone)]
struct Vertex {
    _position: [f32; 4],
    _tex_coords: [f32; 2],
}

impl Renderer {
    pub fn new(device: &Device, size: (u32, u32)) -> Self {
        let pixels = size.0 * size.1;

        let vertex_size = mem::size_of::<Vertex>();
        let vertex_data = [
            Vertex::new([-1.0, -1.0, 1.0, 1.0], [0.0, 0.0]),
            Vertex::new([1.0, -1.0, 1.0, 1.0], [1.0, 0.0]),
            Vertex::new([1.0, 1.0, 1.0, 1.0], [1.0, 1.0]),
            Vertex::new([-1.0, 1.0, 1.0, 1.0], [0.0, 1.0]),
        ];

        let index_data: &[u16] = &[0, 1, 2, 2, 3, 0];

        let vertex_buf = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&vertex_data),
            usage: BufferUsages::VERTEX,
        });
        let index_buf = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(index_data),
            usage: BufferUsages::INDEX,
        });

        let render_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(include_str!("shaders/render.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(mem::size_of::<ShaderContext>() as _),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(pixels as u64 * 4),
                    },
                    count: None,
                },
            ],
        });

        let vertex_buffers = [VertexBufferLayout {
            array_stride: vertex_size as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 0,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x2,
                    offset: 4 * 4,
                    shader_location: 1,
                },
            ],
        }];

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &render_shader,
                entry_point: "vert",
                buffers: &vertex_buffers,
            },
            fragment: Some(FragmentState {
                module: &render_shader,
                entry_point: "frag",
                targets: &[Some(ColorTargetState {
                    format: TEXTURE_FORMAT,
                    blend: None,
                    write_mask: ColorWrites::all(),
                })],
            }),
            primitive: PrimitiveState {
                cull_mode: Some(Face::Back),
                ..Default::default()
            },
            depth_stencil: None,
            multiview: None,
            multisample: MultisampleState::default(),
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
        let gc = &app.graphics;
        let bind_group = gc.device.create_bind_group(&BindGroupDescriptor {
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: context_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: app.simulation.get_state().as_entire_binding(),
                },
            ],
            label: None,
        });

        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.set_index_buffer(self.index_buf.slice(..), IndexFormat::Uint16);
        render_pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        render_pass.draw_indexed(0..6, 0, 0..1);
    }

    pub fn screenshot(&self, app: &App) -> Result<()> {
        let gc = &app.graphics;
        let size = app.simulation.get_size();
        let context_buffer = app
            .simulation
            .get_context_buffer(&gc.device, PhysicalSize::new(size.0, size.1));

        let texture = gc.device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TEXTURE_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let screenshot_buffer = gc.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (size.0 * size.1) as u64 * 4,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = gc
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });

        let view = texture.create_view(&TextureViewDescriptor::default());
        self.render(app, &mut encoder, &context_buffer, &view);

        encoder.copy_texture_to_buffer(
            ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            ImageCopyBuffer {
                buffer: &screenshot_buffer,
                layout: ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * size.0),
                    rows_per_image: None,
                },
            },
            Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
        );

        gc.queue.submit([encoder.finish()]);

        let screenshot_slice = screenshot_buffer.slice(..);
        let (tx, rx) = crossbeam_channel::bounded(1);
        screenshot_slice.map_async(MapMode::Read, move |_| tx.send(()).unwrap());

        gc.device.poll(Maintain::Wait);
        rx.recv().unwrap();

        let data = screenshot_slice.get_mapped_range();
        let result = bytemuck::cast_slice::<_, u8>(&data).to_vec();

        drop(data);
        screenshot_buffer.unmap();

        let image = ImageBuffer::<Rgba<u8>, _>::from_vec(size.0, size.1, result).unwrap();
        save_screenshot(image)
    }
}

fn save_screenshot(mut image: ImageBuffer<Rgba<u8>, Vec<u8>>) -> Result<()> {
    // Convert Bgra to Rgba
    for y in 0..image.height() {
        for x in 0..image.width() {
            let bgra = image.get_pixel(x, y).0;
            image.put_pixel(x, y, Rgba([bgra[2], bgra[1], bgra[0], bgra[3]]));
        }
    }

    let parent = Path::new("screenshots");

    if !parent.exists() {
        fs::create_dir(parent)?;
    }

    for i in 0.. {
        let path = parent.join(format!("screenshot-{}.png", i));
        if !path.exists() {
            image.save(path)?;
            break;
        }
    }

    Ok(())
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
