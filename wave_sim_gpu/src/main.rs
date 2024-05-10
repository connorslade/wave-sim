use anyhow::{Context, Result};
use encase::ShaderType;
use nd_vec::{vector, Vec2};
use wgpu::{util::DeviceExt, ShaderSource};

const SIZE: (u32, u32) = (1920, 1080);

#[derive(ShaderType)]
struct ShaderContext {
    width: u32,
}

impl ShaderContext {
    fn to_wgsl_bytes(&self) -> Vec<u8> {
        let mut buffer = encase::UniformBuffer::new(Vec::new());
        buffer.write(self).unwrap();
        buffer.into_inner()
    }
}

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

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
    });

    let size = SIZE.0 * SIZE.1;
    let ctx = ShaderContext { width: SIZE.0 };

    let state_staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 4 * size as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

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
    let state_buffer_1 = device.create_buffer_init(&state_buffer_descriptor);
    let state_buffer_2 = device.create_buffer_init(&state_buffer_descriptor);
    let state_buffer_3 = device.create_buffer_init(&state_buffer_descriptor);

    let context_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Context Buffer"),
        contents: &ctx.to_wgsl_bytes(),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &shader,
        entry_point: "main",
        compilation_options: Default::default(),
    });

    let bind_group_layout = compute_pipeline.get_bind_group_layout(0);
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
                resource: state_buffer_1.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: state_buffer_2.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: state_buffer_3.as_entire_binding(),
            },
        ],
    });

    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        cpass.set_pipeline(&compute_pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.dispatch_workgroups(SIZE.0 / 8, SIZE.1 / 8, 1);
    }

    queue.submit(Some(encoder.finish()));

    let (tx, rx) = flume::bounded(1);
    let buffer_slice = state_staging.slice(..);
    buffer_slice.map_async(wgpu::MapMode::Read, move |x| tx.send(x).unwrap());

    device.poll(wgpu::MaintainBase::Wait).panic_on_timeout();

    rx.recv_async().await.unwrap().unwrap();
    let data = buffer_slice.get_mapped_range();
    let slice: Vec<f32> = bytemuck::cast_slice(&data).to_vec();

    drop(data);
    state_staging.unmap();

    Ok(())
}

pub fn main() -> Result<()> {
    pollster::block_on(run())
}
