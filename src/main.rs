use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use egui::Egui;
use image::io::Reader;
use spin_sleep_util::Interval;
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

struct App<'a> {
    window: Arc<Window>,
    surface: Surface<'a>,
    device: Device,
    queue: Queue,

    simulation: Simulation,
    renderer: Renderer,
    egui: Egui,

    target_fps: u32,
    interval: Interval,
    last_frame: Instant,
}

#[pollster::main]
async fn main() -> Result<()> {
    let image = Reader::open("map.png")?.decode()?;
    let size = (image.width(), image.height());

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

    let simulation = Simulation::new(&device, image);
    let renderer = Renderer::new(&device, size.0 * size.1);

    let event_loop = EventLoop::new()?;

    let window = Arc::new(
        WindowBuilder::new()
            .with_title("Wave Simulator")
            .with_inner_size(PhysicalSize::new(size.0, size.1))
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
            present_mode: PresentMode::Immediate,
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

        target_fps: 60,
        interval: spin_sleep_util::interval(Duration::from_secs_f64(1.0 / 60.0)),
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
                WindowEvent::Resized(size) => {
                    app.surface.configure(
                        &app.device,
                        &SurfaceConfiguration {
                            usage: TextureUsages::RENDER_ATTACHMENT,
                            format: TextureFormat::Rgba8Unorm,
                            width: size.width,
                            height: size.height,
                            present_mode: PresentMode::Immediate,
                            desired_maximum_frame_latency: 2,
                            alpha_mode: CompositeAlphaMode::Opaque,
                            view_formats: vec![],
                        },
                    );
                }
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

        let context_buffer = self
            .simulation
            .get_context_buffer(&self.device, self.window.inner_size());
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
                ::egui::Window::new("Wave Simulator")
                    .default_width(0.0)
                    .show(ctx, |ui| {
                        let size = self.simulation.get_size();
                        let fps = frame_time.as_secs_f64().recip();

                        ui.label(format!("Size: {}x{}", size.0, size.1));
                        ui.label(format!("FPS: {fps:.2}"));
                        ui.label(format!("Tick: {}", self.simulation.tick));

                        ui.separator();

                        ui.add(
                            ::egui::Slider::new(&mut self.target_fps, 30..=1000).text("Target FPS"),
                        );
                        ui.add(::egui::Slider::new(&mut self.simulation.c, 0.0..=0.1).text("C"));
                        ui.add(
                            ::egui::Slider::new(&mut self.simulation.amplitude, 0.0..=0.05)
                                .text("Amplitude"),
                        );
                        ui.add(
                            ::egui::Slider::new(&mut self.simulation.oscillation, 1.0..=1000.0)
                                .text("Oscillation"),
                        );

                        ui.separator();

                        ui.horizontal(|ui| {
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

                            if ui.button("⟳").clicked() {
                                self.simulation.reset_states(&self.queue);
                            }
                        });

                        self.interval
                            .set_period(Duration::from_secs_f64(1.0 / self.target_fps as f64));
                    });
            },
        );

        self.queue.submit([encoder.finish()]);

        output.present();
        self.window.request_redraw();
        self.interval.tick();
    }
}
