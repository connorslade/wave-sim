use std::{
    sync::Arc,
    time::{Duration, Instant},
};

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
mod ui;
use renderer::Renderer;
use simulation::Simulation;

const ICON: &[u8] = include_bytes!("assets/icon.png");

struct App<'a> {
    graphics: GraphicsContext<'a>,
    simulation: Simulation,
    renderer: Renderer,
    egui: Egui,

    target_fps: u32,
    interval: Interval,
    last_frame: Instant,
}

struct GraphicsContext<'a> {
    window: Arc<Window>,
    surface: Surface<'a>,
    device: Device,
    queue: Queue,
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
        graphics: GraphicsContext {
            window,
            surface,
            device,
            queue,
        },

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
                app.egui.handle_event(&app.graphics, &event);
            }

            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::RedrawRequested => app.render(),
                WindowEvent::Resized(size) => {
                    app.graphics.surface.configure(
                        &app.graphics.device,
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
        let gc = &self.graphics;
        let context_buffer = self
            .simulation
            .get_context_buffer(&gc.device, gc.window.inner_size());
        let mut encoder = gc
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });

        self.simulation
            .update(&gc.device, &mut encoder, &context_buffer);

        let output = gc.surface.get_current_texture().unwrap();
        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        self.renderer
            .render(self, &mut encoder, &context_buffer, &view);

        let mut do_screenshot = false;
        self.egui.render(gc, &mut encoder, &view, |ctx| {
            ui::ui(
                ctx,
                &gc.queue,
                &mut self.simulation,
                &mut self.interval,
                &mut self.last_frame,
                &mut self.target_fps,
                &mut do_screenshot,
            );
        });

        gc.queue.submit([encoder.finish()]);

        output.present();
        gc.window.request_redraw();

        if do_screenshot {
            if let Err(e) = self.renderer.screenshot(self) {
                eprintln!("Failed to take screenshot: {:?}", e);
            }
        }

        self.interval.tick();
    }
}
