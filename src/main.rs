use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use ::egui::{Color32, RichText, Slider};
use anyhow::{Context, Result};
use egui::Egui;
use image::ImageFormat;
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
    window::{Icon, Window, WindowBuilder},
};

mod args;
mod egui;
mod renderer;
mod simulation;
use renderer::Renderer;
use simulation::Simulation;

const ICON: &[u8] = include_bytes!("assets/icon.png");

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
    let args = args::parse()?;

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

    let simulation = Simulation::new(&device, &args)?;
    let renderer = Renderer::new(&device, args.size);

    let event_loop = EventLoop::new()?;

    let icon = image::load_from_memory_with_format(ICON, ImageFormat::Png).unwrap();
    let window = Arc::new(
        WindowBuilder::new()
            .with_title("Wave Simulator | Connor Slade")
            .with_window_icon(Some(
                Icon::from_rgba(icon.to_rgba8().to_vec(), icon.width(), icon.height()).unwrap(),
            ))
            .with_inner_size(PhysicalSize::new(args.size.0, args.size.1))
            .build(&event_loop)?,
    );

    let surface = instance.create_surface(window.clone()).unwrap();

    let size = window.inner_size();
    surface.configure(
        &device,
        &SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: TextureFormat::Bgra8Unorm,
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
                            format: TextureFormat::Bgra8Unorm,
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

        let mut do_screenshot = false;
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

                        let c = 0.002 * self.simulation.dt * self.simulation.v / self.simulation.dx;
                        ui.horizontal(|ui| {
                            ui.label(format!("Courant: {c:.2}"));
                            if c > 0.7 {
                                ui.label(
                                    RichText::new(format!("CFL not met. (c < 0.7)"))
                                        .color(Color32::RED),
                                );
                            } else if c == 0.0 {
                                ui.label(RichText::new(format!("C is zero.")).color(Color32::RED));
                            }
                        });

                        ui.separator();

                        ui.add(Slider::new(&mut self.target_fps, 30..=1000).text("Target FPS"));

                        ui.separator();

                        ui.add(Slider::new(&mut self.simulation.dx, 0.0..=10.0).text("dx (m)"));
                        ui.add(Slider::new(&mut self.simulation.dt, 0.0..=0.1).text("dt (ms)"));
                        ui.add(
                            Slider::new(&mut self.simulation.v, 0.0..=300_000_000.0)
                                .text("Wave Speed"),
                        );

                        self.simulation.dx = self.simulation.dx.max(0.00001);
                        self.simulation.dt = self.simulation.dt.max(0.00001);
                        self.simulation.v = self.simulation.v.max(0.00001);

                        ui.separator();

                        ui.add(
                            Slider::new(&mut self.simulation.amplitude, 0.0..=0.05)
                                .text("Amplitude"),
                        );
                        ui.add(
                            Slider::new(&mut self.simulation.oscillation, 0.0..=50.0)
                                .text("Oscillation (kHz)"),
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

                            if ui.button("📷").clicked() {
                                do_screenshot = true;
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

        if do_screenshot {
            if let Err(e) = self.renderer.screenshot(self) {
                eprintln!("Failed to take screenshot: {:?}", e);
            }
        }

        self.interval.tick();
    }
}
