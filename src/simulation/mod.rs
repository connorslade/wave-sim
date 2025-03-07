use std::{
    borrow::Cow,
    cmp::Ordering,
    f32::consts::TAU,
    fs::{self, File},
};

use anyhow::{Context, Result};
use bitflags::bitflags;
use encase::ShaderType;
use image::{DynamicImage, GenericImage, ImageReader};
use nalgebra::Vector2;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupDescriptor, BindGroupEntry, Buffer, BufferAddress, BufferDescriptor, BufferUsages,
    CommandEncoder, ComputePassDescriptor, ComputePipeline, ComputePipelineDescriptor, Device,
    Queue, ShaderModuleDescriptor, ShaderSource,
};
use winit::dpi::PhysicalSize;

use crate::{
    config::Config,
    misc::{
        audio::Audio,
        preprocess::{Data, Preprocessor},
    },
    GraphicsContext,
};

const TICK_SIGNATURE: &str = "fn tick(x: u32, y: u32, mul: ptr<function, f32>, distance: ptr<function, f32>, c: ptr<function, f32>)";

mod scripting;
pub mod snapshot;
use scripting::Scripting;
use snapshot::SnapshotQueue;

pub struct Simulation {
    compute_pipeline: ComputePipeline,
    size: Vector2<u32>,

    states: Buffer,
    map_buffer: Buffer,
    average_energy_buffer: Buffer,
    staging_buffer: Buffer,
    audio: Option<Audio>,
    script: Option<Scripting>,

    pub snapshot: SnapshotQueue,
    pub parameters: SimulationParameters,
}

#[derive(Clone)]
pub struct SimulationParameters {
    pub ticks_per_dispatch: u32,
    pub tick: u64,
    pub running: bool,
    pub flags: SimulationFlags,
    pub user: u32,

    pub v: f32,  // [length][time]^-1
    pub dt: f32, // [time]
    pub dx: f32, // [length]

    pub amplitude: f32,
    pub frequency: f32,
}

bitflags! {
    #[derive(Clone, Copy)]
    pub struct SimulationFlags: u32 {
        const REFLECTIVE_BOUNDARY = 1 << 0;
        const ENERGY_VIEW = 1 << 1;
        const BILINIER_SAMPLING = 1 << 2;
    }
}

#[derive(ShaderType)]
pub struct SimulationContext {
    pub size: Vector2<u32>,
    pub window: Vector2<u32>,
    pub user: u32,

    pub tick: u32,
    pub ticks_per_dispatch: u32,
    pub flags: u32,

    pub c: f32,
    pub amplitude: f32,
    pub frequency: f32,
}

impl Simulation {
    pub fn new(device: &Device, config: &Config) -> Result<Self> {
        let map = config
            .map
            .as_ref()
            .map(|map| {
                let mut image = DynamicImage::new_rgba8(config.size.0, config.size.1);
                let map = ImageReader::open(config.base_path().join(map))?.decode()?;
                let x = (config.size.0 - map.width()) / 2;
                let y = (config.size.1 - map.height()) / 2;
                image
                    .copy_from(&map, x, y)
                    .context("Map must have a size equal or smaller than the simulation size.")?;
                Ok::<_, anyhow::Error>(image)
            })
            .transpose()?;

        let audio = config
            .audio
            .as_ref()
            .map(|x| {
                Audio::new(
                    device,
                    File::open(config.base_path().join(&x.input))?,
                    File::create(config.base_path().join(&x.output))?,
                )
            })
            .transpose()?;

        let script = config
            .script
            .as_ref()
            .map(|x| Scripting::from_file(config.base_path().join(x)));

        let mut raw_shader = Cow::Borrowed(include_str!("../shaders/shader.wgsl"));
        if let Some(ref shader) = config.shader {
            let shader = fs::read_to_string(config.base_path().join(shader)).unwrap();
            let line_end = raw_shader.find('\n').unwrap();
            raw_shader = Cow::Owned(format!(
                "{TICK_SIGNATURE} {{\n{shader}\n}}{}",
                &raw_shader[line_end..]
            ));
        }

        let mut preprocessor = Preprocessor::new();
        if let Some(audio) = &config.audio {
            preprocessor = preprocessor.define("AUDIO", Data::vec2(audio.pickup.0, audio.pickup.1));
        } else {
            preprocessor = preprocessor.define("OSCILLATOR", Data::Null);
        }

        let raw_shader = preprocessor.process(&raw_shader);
        let compute_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(raw_shader.into()),
        });

        let map_data = match map {
            Some(map) => map.into_rgba8().into_raw(),
            None => {
                let mut out = vec![0; (config.size.0 * config.size.1) as usize * 4];
                out.chunks_exact_mut(4).for_each(|x| x[2] = 128);
                out
            }
        };
        let map_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: map_data.as_slice(),
            usage: BufferUsages::STORAGE,
        });

        let state_buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size: (config.size.0 * config.size.1 * 3 * 4) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let average_energy_buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size: (config.size.0 * config.size.1 * 4) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let staging_buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size: (config.size.0 * config.size.1 * 4) as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let compute_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &compute_shader,
            entry_point: "main",
        });

        let mut flags = SimulationFlags::BILINIER_SAMPLING;
        if config.parameters.reflective_boundary {
            flags |= SimulationFlags::REFLECTIVE_BOUNDARY;
        }

        let mut this = Self {
            compute_pipeline,
            size: Vector2::new(config.size.0, config.size.1),

            states: state_buffer,
            map_buffer,
            average_energy_buffer,
            staging_buffer,
            audio,
            script,

            snapshot: Default::default(),
            parameters: SimulationParameters {
                ticks_per_dispatch: 1,
                tick: 0,
                running: false,
                flags,
                user: 0,

                dt: config.parameters.dt,
                dx: config.parameters.dx,

                v: config.parameters.v,
                amplitude: config.oscillator.amplitude,
                frequency: config.oscillator.frequency,
            },
        };

        this.script_update(None, "init");
        Ok(this)
    }

    pub fn get_state(&self) -> &Buffer {
        &self.states
    }

    pub fn get_average_energy_buffer(&self) -> &Buffer {
        &self.average_energy_buffer
    }

    pub fn get_size(&self) -> Vector2<u32> {
        self.size
    }

    pub fn update(
        &mut self,
        gc: &GraphicsContext,
        encoder: &mut CommandEncoder,
        window_size: PhysicalSize<u32>,
    ) {
        if !self.parameters.running {
            return;
        }

        for _ in 0..self.parameters.ticks_per_dispatch {
            let buf = self.get_context_buffer(&gc.device, window_size);

            let params = &mut self.parameters;
            let bind_group_layout = self.compute_pipeline.get_bind_group_layout(0);
            let mut entries = vec![
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
            ];

            if let Some(audio) = &self.audio {
                entries.extend([
                    BindGroupEntry {
                        binding: 4,
                        resource: audio.audio_in_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 5,
                        resource: audio.audio_out_buffer.as_entire_binding(),
                    },
                ]);
            }

            let bind_group = gc.device.create_bind_group(&BindGroupDescriptor {
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
            compute_pass.dispatch_workgroups(self.size.x.div_ceil(8), self.size.y.div_ceil(8), 1);
            drop(compute_pass);

            if let Some(audio) = &mut self.audio {
                match audio.audio_in_len.cmp(&(params.tick as usize)) {
                    Ordering::Equal => params.running = false,
                    Ordering::Greater => audio.tick(params.tick, gc, encoder),
                    _ => {}
                }
            }

            params.tick += 1;
            self.script_update(Some(&gc.queue), "update");
        }
    }

    fn script_update(&mut self, queue: Option<&Queue>, func: &str) {
        if let Some(script) = &mut self.script {
            let response = script.update(&mut self.parameters, func);

            if let (Some(queue), true) = (queue, response.reset) {
                self.reset_states(queue);
                self.reset_average_energy(queue);
            }

            self.snapshot.extend(response.snapshot);
        }
    }

    pub fn get_context_buffer(&self, device: &Device, window_size: PhysicalSize<u32>) -> Buffer {
        let params = &self.parameters;
        let context = SimulationContext {
            size: self.size,
            window: Vector2::new(window_size.width, window_size.height),
            user: params.user,

            tick: params.tick as u32,
            ticks_per_dispatch: params.ticks_per_dispatch,
            flags: params.flags.bits(),

            c: params.v * (params.dt / params.dx),
            amplitude: params.amplitude,
            frequency: TAU * params.dt * params.frequency,
        };

        device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: &context.to_wgsl_bytes(),
            usage: BufferUsages::UNIFORM,
        })
    }

    pub fn stage_energy(&self, encoder: &mut CommandEncoder) -> &Buffer {
        encoder.copy_buffer_to_buffer(
            &self.average_energy_buffer,
            0,
            &self.staging_buffer,
            0,
            self.staging_buffer.size(),
        );
        &self.staging_buffer
    }

    pub fn reset_states(&mut self, queue: &Queue) {
        self.parameters.tick = 0;
        let empty_buffer = vec![0f32; (self.size.x * self.size.y * 3) as usize];
        queue.write_buffer(
            &self.states,
            BufferAddress::default(),
            bytemuck::cast_slice(&empty_buffer),
        )
    }

    pub fn reset_average_energy(&mut self, queue: &Queue) {
        let empty_buffer = vec![0f32; (self.size.x * self.size.y) as usize];
        queue.write_buffer(
            &self.average_energy_buffer,
            BufferAddress::default(),
            bytemuck::cast_slice(&empty_buffer),
        )
    }
}

impl SimulationContext {
    pub fn to_wgsl_bytes(&self) -> Vec<u8> {
        let mut buffer = encase::UniformBuffer::new(Vec::new());
        buffer.write(self).unwrap();
        buffer.into_inner()
    }
}
