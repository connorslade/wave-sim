use encase::ShaderType;
use wgpu::{util::DeviceExt, Buffer, CommandEncoder, Device, ShaderSource};

pub struct Simulation {
    compute_pipeline: wgpu::ComputePipeline,
    states: [wgpu::Buffer; 3],
    size: (u32, u32),

    pub c: f32,
    pub tick: usize,
    pub running: bool,
}

#[derive(ShaderType)]
pub struct ShaderContext {
    width: u32,
    height: u32,
    tick: u32,
    c: f32,
}

impl Simulation {
    pub fn new(device: &Device, size: (u32, u32)) -> Self {
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(include_str!("shaders/shader.wgsl").into()),
        });

        let empty_buffer = vec![0f32; (size.0 * size.1) as usize];
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
        });

        Self {
            compute_pipeline,
            states: [state_buffer_1, state_buffer_2, state_buffer_3],
            size,

            c: 0.02,
            tick: 0,
            running: false,
        }
    }

    pub fn get_state(&self) -> &wgpu::Buffer {
        &self.states[self.tick % 3]
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
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
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
                    resource: self.states[self.tick % 3].as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.states[(self.tick + 2) % 3].as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.states[(self.tick + 1) % 3].as_entire_binding(),
                },
            ],
        });

        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
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
        };

        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Context Buffer"),
            contents: &context.to_wgsl_bytes(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
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
