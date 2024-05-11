use std::sync::Arc;

use anyhow::{Context, Result};
use egui::Egui;
use encase::ShaderType;
use wgpu::{util::DeviceExt, TextureFormat, TextureUsages};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

mod egui;
mod renderer;
mod simulation;
use renderer::Renderer;
use simulation::Simulation;

const SIZE: (u32, u32) = (2048, 2048);

struct App<'a> {
    window: Arc<Window>,
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,

    simulation: Simulation,
    renderer: Renderer,
    egui: Egui,
}

#[derive(ShaderType)]
struct ShaderContext {
    width: u32,
    height: u32,
    tick: u32,
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
        .await?;

    let simulation = Simulation::new(&device, SIZE);
    let renderer = Renderer::new(&device, SIZE.0 * SIZE.1);

    let event_loop = EventLoop::new()?;

    let window = Arc::new(
        WindowBuilder::new()
            .with_inner_size(PhysicalSize::new(SIZE.0, SIZE.1))
            .with_resizable(false)
            .build(&event_loop)?,
    );

    let surface = instance.create_surface(window.clone()).unwrap();

    let size = window.inner_size();
    surface.configure(
        &device,
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

    let egui = Egui::new(&device, &*window);

    let mut app = App {
        window,
        surface,
        device,
        queue,

        simulation,
        renderer,
        egui,
    };

    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    event_loop.run(|event, event_loop| {
        if let Event::WindowEvent {
            window_id: _,
            event,
        } = event
        {
            if !matches!(event, WindowEvent::RedrawRequested) {
                app.egui.handle_event(&app.window, &event);
            }

            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::RedrawRequested => app.render(),
                WindowEvent::KeyboardInput {
                    device_id: _,
                    event,
                    is_synthetic: _,
                } => {
                    app.simulation.running ^= event.physical_key
                        == PhysicalKey::Code(KeyCode::Space)
                        && event.state.is_pressed();
                    app.update_title();
                }
                _ => {}
            }
        }
    })?;

    Ok(())
}

impl<'a> App<'a> {
    fn update_title(&mut self) {
        self.window.as_ref().set_title(&format!(
            "Wave Simulator | {}",
            if self.simulation.running {
                "running"
            } else {
                "paused"
            }
        ));
    }

    fn render(&mut self) {
        let context_buffer = self.get_context_buffer();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        self.simulation
            .update(&self.device, &mut encoder, &context_buffer);

        let output = self.surface.get_current_texture().unwrap();
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        self.renderer
            .render(self, &mut encoder, &context_buffer, &view);

        self.egui
            .render(&self.device, &self.queue, &self.window, &mut encoder, &view);

        self.queue.submit([encoder.finish()]);

        output.present();
        self.window.request_redraw();
    }

    fn get_context_buffer(&self) -> wgpu::Buffer {
        let ctx = ShaderContext {
            width: SIZE.0,
            height: SIZE.1,
            tick: self.simulation.tick as u32,
        };

        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: &ctx.to_wgsl_bytes(),
                usage: wgpu::BufferUsages::UNIFORM,
            })
    }
}

pub fn main() -> Result<()> {
    pollster::block_on(run())
}
