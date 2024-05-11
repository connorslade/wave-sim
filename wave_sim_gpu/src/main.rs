use std::{sync::Arc, time::Instant};

use anyhow::{Context, Result};
use egui::Egui;
use wgpu::{
    CommandEncoderDescriptor, CompositeAlphaMode, Device, DeviceDescriptor, Features, Instance,
    Limits, PresentMode, Queue, RequestAdapterOptions, Surface, SurfaceConfiguration,
    TextureFormat, TextureUsages, TextureViewDescriptor,
};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

mod egui;
mod renderer;
mod simulation;
use renderer::Renderer;
use simulation::Simulation;

const SIZE: (u32, u32) = (1920, 1080);

struct App<'a> {
    window: Arc<Window>,
    surface: Surface<'a>,
    device: Device,
    queue: Queue,

    simulation: Simulation,
    renderer: Renderer,
    egui: Egui,

    last_frame: Instant,
}

async fn run() -> Result<()> {
    let instance = Instance::default();

    let adapter = instance
        .request_adapter(&RequestAdapterOptions::default())
        .await
        .context("No adapter found")?;

    let (device, queue) = adapter
        .request_device(
            &DeviceDescriptor {
                label: None,
                required_features: Features::empty(),
                required_limits: Limits::default(),
            },
            None,
        )
        .await?;

    let simulation = Simulation::new(&device, SIZE);
    let renderer = Renderer::new(&device, SIZE.0 * SIZE.1);

    let event_loop = EventLoop::new()?;

    let window = Arc::new(
        WindowBuilder::new()
            .with_title("Wave Simulator")
            .with_inner_size(PhysicalSize::new(SIZE.0, SIZE.1))
            .with_resizable(false)
            .build(&event_loop)?,
    );

    let surface = instance.create_surface(window.clone()).unwrap();

    let size = window.inner_size();
    surface.configure(
        &device,
        &SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: TextureFormat::Rgba8Unorm,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: CompositeAlphaMode::Opaque,
            view_formats: vec![],
        },
    );

    let egui = Egui::new(&device, &window);

    let mut app = App {
        window,
        surface,
        device,
        queue,

        simulation,
        renderer,
        egui,

        last_frame: Instant::now(),
    };

    event_loop.set_control_flow(ControlFlow::Poll);

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
                }
                _ => {}
            }
        }
    })?;

    Ok(())
}

impl<'a> App<'a> {
    fn render(&mut self) {
        let now = Instant::now();
        let frame_time = now - self.last_frame;
        self.last_frame = now;

        let context_buffer = self.simulation.get_context_buffer(&self.device);
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });

        self.simulation
            .update(&self.device, &mut encoder, &context_buffer);

        let output = self.surface.get_current_texture().unwrap();
        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        self.renderer
            .render(self, &mut encoder, &context_buffer, &view);

        self.egui.render(
            &self.device,
            &self.queue,
            &self.window,
            &mut encoder,
            &view,
            |ctx| {
                ::egui::Window::new("Wave Simulator").show(ctx, |ui| {
                    ui.label(format!("Size: {}x{}", SIZE.0, SIZE.1));
                    ui.label(format!("FPS: {:.2}", frame_time.as_secs_f64().recip()));
                    ui.label(format!("Tick: {}", self.simulation.tick));

                    ui.add(::egui::Slider::new(&mut self.simulation.c, 0.0..=0.1).text("C"));

                    if ui
                        .button(if self.simulation.running {
                            "⏸"
                        } else {
                            "▶"
                        })
                        .clicked()
                    {
                        self.simulation.running ^= true;
                    }
                });
            },
        );

        self.queue.submit([encoder.finish()]);

        output.present();
        self.window.request_redraw();
    }
}

pub fn main() -> Result<()> {
    pollster::block_on(run())
}
