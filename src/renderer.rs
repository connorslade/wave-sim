use std::mem;

use anyhow::Result;
use encase::{ShaderType, UniformBuffer};
use image::{GenericImageView, ImageBuffer, Rgba};
use nalgebra::Vector2;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferSize, BufferUsages,
    ColorTargetState, ColorWrites, CommandEncoder, CommandEncoderDescriptor, Device, Extent3d,
    Face, FragmentState, ImageCopyBuffer, ImageCopyTexture, ImageDataLayout, IndexFormat, LoadOp,
    Maintain, MapMode, MultisampleState, Operations, Origin3d, PipelineLayoutDescriptor,
    PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderSource, ShaderStages, StoreOp,
    TextureAspect, TextureDescriptor, TextureDimension, TextureUsages, TextureView,
    TextureViewDescriptor, VertexState, COPY_BYTES_PER_ROW_ALIGNMENT,
};

use crate::{misc::util, App, TEXTURE_FORMAT};

pub struct Renderer {
    render_pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    index: Buffer,
    context: Buffer,

    pub pan: Vector2<f32>,
}

#[derive(ShaderType, Default)]
pub struct RenderContext {
    size: Vector2<u32>,
    window: Vector2<u32>,

    tick: u32,
    flags: u32,
    gain: f32,
    energy_gain: f32,
}

impl Renderer {
    pub fn new(device: &Device, size: (u32, u32)) -> Self {
        let pixels = size.0 * size.1;

        let index_data: &[u16] = &[0, 1, 2, 2, 3, 0];

        let index = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(index_data),
            usage: BufferUsages::INDEX,
        });

        let mut context = UniformBuffer::new(Vec::new());
        context.write(&RenderContext::default()).unwrap();
        let context = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: &context.into_inner(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let render_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(include_str!("shaders/render.wgsl").into()),
        });

        let state_layout_type = BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: BufferSize::new(pixels as u64 * 4),
        };
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(
                            dbg!(mem::size_of::<RenderContext>()) as _
                        ),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: state_layout_type,
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: state_layout_type,
                    count: None,
                },
            ],
        });

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
                buffers: &[],
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
            index,
            context,

            pan: Vector2::zeros(),
        }
    }

    pub fn render(&self, app: &App, encoder: &mut CommandEncoder, view: &TextureView) {
        let gc = &app.graphics;
        let bind_group = gc.device.create_bind_group(&BindGroupDescriptor {
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: self.context.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: app.simulation.get_state().as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: app
                        .simulation
                        .get_average_energy_buffer()
                        .as_entire_binding(),
                },
            ],
            label: None,
        });

        let window = app.graphics.window.inner_size();
        let mut context = UniformBuffer::new(Vec::new());
        context
            .write(&RenderContext {
                size: app.simulation.get_size(),
                window: Vector2::new(window.width, window.height),
                tick: app.simulation.tick as u32,
                flags: app.simulation.flags.bits(),

                // TODO: move out of simulation
                gain: app.simulation.gain,
                energy_gain: app.simulation.energy_gain,
            })
            .unwrap();
        gc.queue
            .write_buffer(&self.context, 0, &context.into_inner());

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
        render_pass.set_index_buffer(self.index.slice(..), IndexFormat::Uint16);
        render_pass.draw_indexed(0..6, 0, 0..1);
    }

    pub fn screenshot(&self, app: &App) -> Result<()> {
        let gc = &app.graphics;
        let size = app.simulation.get_size();

        let texture = gc.device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TEXTURE_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        const ALIGNMENT_BYTES: u64 = COPY_BYTES_PER_ROW_ALIGNMENT as u64 - 1;
        let row_bytes = (size.x as u64 * 4 + ALIGNMENT_BYTES) & !ALIGNMENT_BYTES;
        let screenshot_buffer = gc.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: row_bytes * size.y as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = gc
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });

        let view = texture.create_view(&TextureViewDescriptor::default());
        self.render(app, &mut encoder, &view);

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
                    bytes_per_row: Some(row_bytes as u32),
                    rows_per_image: None,
                },
            },
            Extent3d {
                width: size.x,
                height: size.y,
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

        let image =
            ImageBuffer::<Rgba<u8>, _>::from_vec(row_bytes as u32 / 4, size.y, result).unwrap();
        save_screenshot(image, size)
    }
}

fn save_screenshot(mut image: ImageBuffer<Rgba<u8>, Vec<u8>>, size: Vector2<u32>) -> Result<()> {
    image = image.view(0, 0, size.x, size.y).to_image();

    // Convert Bgra to Rgba
    for y in 0..image.height() {
        for x in 0..image.width() {
            let bgra = image.get_pixel(x, y).0;
            image.put_pixel(x, y, Rgba([bgra[2], bgra[1], bgra[0], bgra[3]]));
        }
    }

    let path = util::save_dated_file("screenshots", "screenshot", "png")?;
    image.save(path)?;

    Ok(())
}
