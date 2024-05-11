use std::sync::Arc;

use anyhow::{Context, Result};
use encase::ShaderType;
use soon::Soon;
use wgpu::{util::DeviceExt, TextureFormat, TextureUsages};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

mod renderer;
mod simulation;
use renderer::Renderer;
use simulation::Simulation;

const SIZE: (u32, u32) = (2048, 2048);

struct App<'a> {
    window: Soon<Arc<Window>>,
    surface: Soon<wgpu::Surface<'a>>,
    instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,

    simulation: Simulation,
    renderer: Renderer,
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
    let mut app = App {
        window: Soon::empty(),
        surface: Soon::empty(),
        instance,
        device,
        queue,

        simulation,
        renderer,
    };

    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    event_loop.run_app(&mut app)?;

    Ok(())
}

impl<'a> ApplicationHandler for App<'a> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes().with_inner_size(PhysicalSize::new(SIZE.0, SIZE.1)),
                )
                .unwrap(),
        );

        let surface = self.instance.create_surface(window.clone()).unwrap();

        let size = window.inner_size();
        surface.configure(
            &self.device,
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

        self.window.replace(window);
        self.surface.replace(surface);
        self.update_title();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => self.render(),
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                self.simulation.running ^= event.physical_key == PhysicalKey::Code(KeyCode::Space)
                    && event.state.is_pressed();
                self.update_title();
            }
            _ => {}
        }
    }
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
        let ctx = ShaderContext {
            width: SIZE.0,
            height: SIZE.1,
            tick: self.simulation.tick as u32,
        };

        let context_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: &ctx.to_wgsl_bytes(),
                usage: wgpu::BufferUsages::UNIFORM,
            });

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

        self.queue.submit(Some(encoder.finish()));
        output.present();

        self.window.request_redraw();
    }
}

pub fn main() -> Result<()> {
    pollster::block_on(run())
}
