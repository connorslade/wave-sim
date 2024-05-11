use encase::ShaderType;
use image::DynamicImage;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupDescriptor, BindGroupEntry, Buffer, BufferUsages, CommandEncoder,
    ComputePassDescriptor, ComputePipelineDescriptor, Device, ShaderModuleDescriptor, ShaderSource,
};

pub struct Simulation {
    compute_pipeline: wgpu::ComputePipeline,
    states: [Buffer; 3],
    map_buffer: Buffer,
    size: (u32, u32),

    pub tick: usize,
    pub running: bool,

    pub c: f32,
    pub amplitude: f32,
    pub oscillation: f32,
}

#[derive(ShaderType)]
pub struct ShaderContext {
    width: u32,
    height: u32,
    tick: u32,

    c: f32,
    amplitude: f32,
    oscillation: f32,
}

impl Simulation {
    pub fn new(device: &Device, image: DynamicImage) -> Self {
        let size = (image.width(), image.height());

        let compute_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(include_str!("shaders/shader.wgsl").into()),
        });

        let image_data = image.into_rgba8().into_raw();
        let map_buffer_descriptor = BufferInitDescriptor {
            label: None,
            contents: image_data.as_slice(),
            usage: BufferUsages::STORAGE,
        };
        let map_buffer = device.create_buffer_init(&map_buffer_descriptor);

        let empty_buffer = vec![0f32; (size.0 * size.1) as usize];
        let state_buffer_descriptor = BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&empty_buffer),
            usage: BufferUsages::STORAGE,
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

        Self {
            compute_pipeline,
            states: [state_buffer_1, state_buffer_2, state_buffer_3],
            map_buffer,
            size,

            tick: 0,
            running: false,

            c: 0.02,
            amplitude: 0.005,
            oscillation: 30.0,
        }
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
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: context_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: self.map_buffer.as_entire_binding(),
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
            ],
        });

        let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        cpass.set_pipeline(&self.compute_pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.dispatch_workgroups(self.size.0 / 8, self.size.1 / 8, 1);
    }

    pub fn get_context_buffer(&self, device: &Device) -> Buffer {
        let context = ShaderContext {
            width: self.size.0,
            height: self.size.1,
            tick: self.tick as u32,

            c: self.c,
            amplitude: self.amplitude,
            oscillation: self.oscillation,
        };

        device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Context Buffer"),
            contents: &context.to_wgsl_bytes(),
            usage: BufferUsages::UNIFORM,
        })
    }
}

impl ShaderContext {
    fn to_wgsl_bytes(&self) -> Vec<u8> {
        let mut buffer = encase::UniformBuffer::new(Vec::new());
        buffer.write(self).unwrap();
        buffer.into_inner()
    }
}
