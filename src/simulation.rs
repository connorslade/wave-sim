use std::{
    borrow::Cow,
    f32::consts::PI,
    fs::{self, File},
    io::BufWriter,
};

use anyhow::{Context, Result};
use bitflags::bitflags;
use encase::ShaderType;
use hound::WavWriter;
use image::{io::Reader, DynamicImage, GenericImage};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupDescriptor, BindGroupEntry, Buffer, BufferAddress, BufferDescriptor, BufferUsages,
    CommandEncoder, ComputePassDescriptor, ComputePipeline, ComputePipelineDescriptor, Device,
    Maintain, MapMode, Queue, ShaderModuleDescriptor, ShaderSource,
};
use winit::dpi::PhysicalSize;

use crate::{args::Args, GraphicsContext};

pub struct Simulation {
    compute_pipeline: ComputePipeline,
    states: Buffer,
    map_buffer: Buffer,
    average_energy_buffer: Buffer,
    size: (u32, u32),

    audio_in_buffer: Buffer,
    audio_out_buffer: Buffer,
    audio_in_length: usize,
    staging_buffer: Buffer,
    audio_writer: Option<WavWriter<BufWriter<File>>>,

    pub ticks_per_dispatch: u32,
    pub tick: u64,
    pub running: bool,
    pub flags: SimulationFlags,

    pub v: f32,  // [length][time]^-1
    pub dt: f32, // [time]
    pub dx: f32, // [length]

    pub amplitude: f32,
    pub frequency: f32,
}

bitflags! {
    #[derive(Clone, Copy)]
    pub struct SimulationFlags: u32 {
        const REFLECTIVE_BOUNDARY = 0b0001;
        const ENERGY_VIEW = 0b0010;
    }
}

#[derive(ShaderType)]
pub struct ShaderContext {
    width: u32,
    height: u32,
    window_width: u32,
    window_height: u32,

    tick: u32,
    ticks_per_dispatch: u32,
    flags: u32,

    c: f32,
    amplitude: f32,
    frequency: f32,
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

        let mut raw_shader = Cow::Borrowed(include_str!("shaders/shader.wgsl"));
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

        let map_data = match map {
            Some(map) => map.into_rgba8().into_raw(),
            None => {
                let mut out = vec![0; (args.size.0 * args.size.1) as usize * 4];
                out.chunks_exact_mut(4).for_each(|x| x[2] = 128);
                out
            }
        };
        let map_buffer_descriptor = BufferInitDescriptor {
            label: None,
            contents: map_data.as_slice(),
            usage: BufferUsages::STORAGE,
        };
        let map_buffer = device.create_buffer_init(&map_buffer_descriptor);

        let empty_buffer = vec![0f32; (args.size.0 * args.size.1 * 3) as usize];
        let state_buffer_descriptor = BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&empty_buffer),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        };
        let state_buffer = device.create_buffer_init(&state_buffer_descriptor.clone());
        let average_energy_buffer = device.create_buffer_init(&state_buffer_descriptor);

        let audio_in = hound::WavReader::open("configs/reverb/input.wav")?
            .samples::<f32>()
            .collect::<Result<Vec<_>, hound::Error>>()?;
        let audio_in_length = audio_in.len();
        let audio_in_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&audio_in),
            usage: BufferUsages::STORAGE,
        });

        let audio_out_buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size: 512 * 4, // 512 samples * 4 bytes per sample
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let staging_buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size: 512 * 4,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let compute_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &compute_shader,
            entry_point: "main",
        });

        let mut flags = SimulationFlags::empty();
        if args.reflective_boundary {
            flags |= SimulationFlags::REFLECTIVE_BOUNDARY;
        }

        Ok(Self {
            compute_pipeline,
            states: state_buffer,
            map_buffer,
            average_energy_buffer,
            size: args.size,

            audio_in_buffer,
            audio_out_buffer,
            audio_in_length,
            staging_buffer,
            audio_writer: Some(
                WavWriter::create(
                    "configs/reverb/output.wav",
                    hound::WavSpec {
                        channels: 1,
                        sample_rate: 16_000,
                        bits_per_sample: 32,
                        sample_format: hound::SampleFormat::Float,
                    },
                )
                .unwrap(),
            ),

            ticks_per_dispatch: 1,
            tick: 0,
            running: false,
            flags,

            dt: args.dt,
            dx: args.dx,

            v: args.v,
            amplitude: args.amplitude,
            frequency: args.frequency,
        })
    }

    pub fn get_state(&self) -> &Buffer {
        &self.states
    }

    pub fn get_average_energy_buffer(&self) -> &Buffer {
        &self.average_energy_buffer
    }

    pub fn get_size(&self) -> (u32, u32) {
        self.size
    }

    pub fn update(
        &mut self,
        gc: &GraphicsContext,
        encoder: &mut CommandEncoder,
        window_size: PhysicalSize<u32>,
    ) {
        if !self.running {
            return;
        }

        for _ in 0..self.ticks_per_dispatch {
            self.tick += 1;

            let buf = self.get_context_buffer(&gc.device, window_size);

            let bind_group_layout = self.compute_pipeline.get_bind_group_layout(0);

            let bind_group = gc.device.create_bind_group(&BindGroupDescriptor {
                label: None,
                layout: &bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: buf.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: self.map_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: self.states.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: self.average_energy_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: self.audio_in_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 5,
                        resource: self.audio_out_buffer.as_entire_binding(),
                    },
                ],
            });

            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(self.size.0.div_ceil(8), self.size.1.div_ceil(8), 1);
            drop(compute_pass);

            if let Some(writer) = &mut self.audio_writer {
                if self.tick > 0 && self.tick % 512 == 0 {
                    encoder.copy_buffer_to_buffer(
                        &self.audio_out_buffer,
                        0,
                        &self.staging_buffer,
                        0,
                        512 * 4,
                    );

                    let slice = self.staging_buffer.slice(..);
                    let (tx, rx) = crossbeam_channel::bounded(1);
                    slice.map_async(MapMode::Read, move |_| tx.send(()).unwrap());

                    gc.device.poll(Maintain::Wait);

                    rx.recv().unwrap();
                    let mapped = slice.get_mapped_range();
                    let data = bytemuck::cast_slice::<_, f32>(&mapped);
                    for sample in data.iter().skip(1) {
                        writer
                            .write_sample((1.0 - (-sample.abs()).exp()).copysign(*sample))
                            .unwrap();
                    }
                    drop(mapped);
                    self.staging_buffer.unmap();
                }
            }

            if self.tick == self.audio_in_length as u64 {
                self.audio_writer.take().unwrap().finalize().unwrap();
                ::std::process::exit(0);
            }
        }
    }

    pub fn get_context_buffer(&self, device: &Device, window_size: PhysicalSize<u32>) -> Buffer {
        let context = ShaderContext {
            width: self.size.0,
            height: self.size.1,
            window_width: window_size.width,
            window_height: window_size.height,

            tick: self.tick as u32,
            ticks_per_dispatch: self.ticks_per_dispatch,
            flags: self.flags.bits(),

            c: 0.002 * self.dt * self.v / self.dx,
            amplitude: self.amplitude,
            frequency: 0.0002 * PI * self.dt / (self.frequency * 1000.0).recip(),
        };

        device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: &context.to_wgsl_bytes(),
            usage: BufferUsages::UNIFORM,
        })
    }

    pub fn reset_states(&mut self, queue: &Queue) {
        self.tick = 0;
        let empty_buffer = vec![0f32; (self.size.0 * self.size.1 * 3) as usize];
        queue.write_buffer(
            &self.states,
            BufferAddress::default(),
            bytemuck::cast_slice(&empty_buffer),
        )
    }

    pub fn reset_average_energy(&mut self, queue: &Queue) {
        let empty_buffer = vec![0f32; (self.size.0 * self.size.1) as usize];
        queue.write_buffer(
            &self.average_energy_buffer,
            BufferAddress::default(),
            bytemuck::cast_slice(&empty_buffer),
        )
    }
}

impl ShaderContext {
    fn to_wgsl_bytes(&self) -> Vec<u8> {
        let mut buffer = encase::UniformBuffer::new(Vec::new());
        buffer.write(self).unwrap();
        buffer.into_inner()
    }
}
