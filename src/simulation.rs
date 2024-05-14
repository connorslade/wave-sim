use std::{borrow::Cow, f32::consts::PI, fs};

use anyhow::{Context, Result};
use encase::ShaderType;
use image::{io::Reader, DynamicImage, GenericImage};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupDescriptor, BindGroupEntry, Buffer, BufferAddress, BufferUsages, CommandEncoder,
    ComputePassDescriptor, ComputePipelineDescriptor, Device, Queue, ShaderModuleDescriptor,
    ShaderSource,
};
use winit::dpi::PhysicalSize;

use crate::args::Args;

pub struct Simulation {
    compute_pipeline: wgpu::ComputePipeline,
    states: [Buffer; 3],
    map_buffer: Option<Buffer>,
    size: (u32, u32),

    pub tick: usize,
    pub running: bool,

    pub v: f32,  // [length][time]^-1
    pub dt: f32, // [time]
    pub dx: f32, // [length]

    pub amplitude: f32,
    pub oscillation: f32,
}

#[derive(ShaderType)]
pub struct ShaderContext {
    width: u32,
    height: u32,
    window_width: u32,
    window_height: u32,
    tick: u32,

    c: f32,
    amplitude: f32,
    oscillation: f32,
}

impl Simulation {
    pub fn new(device: &Device, args: &Args) -> Result<Self> {
        let map = args
            .map
            .as_ref()
            .map(|map| {
                let mut image = DynamicImage::new_rgba8(args.size.0, args.size.1);
                let map = Reader::open(args.base_path().join(map))?.decode()?;
                let x = (args.size.0 - map.width()) / 2;
                let y = (args.size.1 - map.height()) / 2;
                image
                    .copy_from(&map, x, y)
                    .context("Map must have a size equal or smaller than the simulation size.")?;
                Ok::<_, anyhow::Error>(image)
            })
            .transpose()?;

        let mut raw_shader = if args.map.is_some() {
            Cow::Borrowed(include_str!("shaders/shader_map.wgsl"))
        } else {
            Cow::Borrowed(include_str!("shaders/shader.wgsl"))
        };
        if let Some(ref shader) = args.shader {
            let shader = fs::read_to_string(args.base_path().join(shader)).unwrap();
            let line_end = raw_shader.find('\n').unwrap();
            raw_shader = Cow::Owned(format!(
                "fn tick(x: u32, y: u32) {{\n{shader}\n}}{}",
                &raw_shader[line_end..]
            ));
        }

        let compute_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(raw_shader),
        });

        let map_buffer = map.map(|map| {
            let image_data = map.into_rgba8().into_raw();
            let map_buffer_descriptor = BufferInitDescriptor {
                label: None,
                contents: image_data.as_slice(),
                usage: BufferUsages::STORAGE,
            };
            device.create_buffer_init(&map_buffer_descriptor)
        });

        let empty_buffer = vec![0f32; (args.size.0 * args.size.1) as usize];
        let state_buffer_descriptor = BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&empty_buffer),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        };
        let state_buffer_1 = device.create_buffer_init(&state_buffer_descriptor.clone());
        let state_buffer_2 = device.create_buffer_init(&state_buffer_descriptor.clone());
        let state_buffer_3 = device.create_buffer_init(&state_buffer_descriptor);

        let compute_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &compute_shader,
            entry_point: "main",
        });

        Ok(Self {
            compute_pipeline,
            states: [state_buffer_1, state_buffer_2, state_buffer_3],
            map_buffer,
            size: args.size,

            tick: 0,
            running: false,

            v: 340.29,
            dt: 0.011,
            dx: 0.05,
            amplitude: args.amplitude,
            oscillation: args.oscillation,
        })
    }

    pub fn get_state(&self) -> &wgpu::Buffer {
        &self.states[self.tick % 3]
    }

    pub fn get_size(&self) -> (u32, u32) {
        self.size
    }

    pub fn update(
        &mut self,
        device: &Device,
        encoder: &mut CommandEncoder,
        context_buffer: &Buffer,
    ) {
        if !self.running {
            return;
        }

        self.tick = self.tick.wrapping_add(1);

        let bind_group_layout = self.compute_pipeline.get_bind_group_layout(0);
        let mut entries = vec![
            BindGroupEntry {
                binding: 0,
                resource: context_buffer.as_entire_binding(),
            },
            // 2 => next, 3 => last, 4 => last2
            BindGroupEntry {
                binding: 2,
                resource: self.states[self.tick % 3].as_entire_binding(),
            },
            BindGroupEntry {
                binding: 3,
                resource: self.states[(self.tick + 2) % 3].as_entire_binding(),
            },
            BindGroupEntry {
                binding: 4,
                resource: self.states[(self.tick + 1) % 3].as_entire_binding(),
            },
        ];
        if let Some(ref map) = self.map_buffer {
            entries.push(BindGroupEntry {
                binding: 1,
                resource: map.as_entire_binding(),
            });
        }
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &entries,
        });

        let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        compute_pass.dispatch_workgroups(self.size.0 / 8, self.size.1 / 8, 1);
    }

    pub fn get_context_buffer(&self, device: &Device, window_size: PhysicalSize<u32>) -> Buffer {
        let context = ShaderContext {
            width: self.size.0,
            height: self.size.1,
            window_width: window_size.width,
            window_height: window_size.height,
            tick: self.tick as u32,

            c: 0.002 * self.dt * self.v / self.dx,
            amplitude: self.amplitude,
            oscillation: 2.0 * PI / (1000.0 * self.oscillation * self.dt),
        };

        device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Context Buffer"),
            contents: &context.to_wgsl_bytes(),
            usage: BufferUsages::UNIFORM,
        })
    }

    pub fn reset_states(&mut self, queue: &Queue) {
        self.tick = 0;
        let empty_buffer = vec![0f32; (self.size.0 * self.size.1) as usize];
        for buffer in &self.states {
            queue.write_buffer(
                buffer,
                BufferAddress::default(),
                bytemuck::cast_slice(&empty_buffer),
            )
        }
    }
}

impl ShaderContext {
    fn to_wgsl_bytes(&self) -> Vec<u8> {
        let mut buffer = encase::UniformBuffer::new(Vec::new());
        buffer.write(self).unwrap();
        buffer.into_inner()
    }
}
