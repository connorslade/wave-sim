use std::{
    fs,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use egui::Egui;
use image::ImageFormat;
use spin_sleep_util::Interval;
use ui::Gui;
use wgpu::{
    CommandEncoderDescriptor, CompositeAlphaMode, Device, DeviceDescriptor, Features, Instance,
    Limits, Maintain, MapMode, PresentMode, Queue, RequestAdapterOptions, Surface,
    SurfaceConfiguration, TextureFormat, TextureUsages, TextureViewDescriptor,
};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Icon, Window, WindowBuilder},
};

mod config;
mod egui;
mod misc;
mod renderer;
mod simulation;
mod ui;
use misc::RingBuffer;
use renderer::Renderer;
use simulation::{Simulation, SimulationFlags};

const ICON: &[u8] = include_bytes!("assets/icon.png");
const TEXTURE_FORMAT: TextureFormat = TextureFormat::Bgra8Unorm;

struct App<'a> {
    graphics: GraphicsContext<'a>,
    simulation: Simulation,
    renderer: Renderer,

    egui: Egui,
    gui: Gui,

    fps: FpsTracker,
}

struct GraphicsContext<'a> {
    window: Arc<Window>,
    surface: Surface<'a>,
    device: Device,
    queue: Queue,
}

struct FpsTracker {
    fps_history: RingBuffer<f64, 256>,
    last_frame: Instant,

    interval: Interval,
    target_fps: u32,
}

#[pollster::main]
async fn main() -> Result<()> {
    let args = config::parse()?;

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
            .with_title("Wave Simulator")
            .with_window_icon(Some(
                Icon::from_rgba(icon.to_rgba8().to_vec(), icon.width(), icon.height()).unwrap(),
            ))
            .with_inner_size(PhysicalSize::new(args.size.0, args.size.1))
            .build(&event_loop)?,
    );

    let surface = instance.create_surface(window.clone()).unwrap();

    let egui = Egui::new(&device, &window);

    let mut app = App {
        simulation,
        renderer,
        egui,
        graphics: GraphicsContext {
            window,
            surface,
            device,
            queue,
        },
        gui: Gui {
            queue_screenshot: false,
            show_about: false,
        },
        fps: FpsTracker {
            fps_history: RingBuffer::new(),
            target_fps: 60,
            interval: spin_sleep_util::interval(Duration::from_secs_f64(60_f64.recip())),
            last_frame: Instant::now(),
        },
    };

    app.configure_surface();

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
                WindowEvent::Resized(_size) => app.configure_surface(),
                WindowEvent::KeyboardInput {
                    device_id: _,
                    event,
                    is_synthetic: _,
                } if event.state.is_pressed() => {
                    app.simulation.running ^=
                        event.physical_key == PhysicalKey::Code(KeyCode::Space);

                    if event.physical_key == PhysicalKey::Code(KeyCode::KeyE) {
                        app.simulation.flags.toggle(SimulationFlags::ENERGY_VIEW);
                    }

                    if event.physical_key == PhysicalKey::Code(KeyCode::KeyR) {
                        app.simulation.reset_states(&app.graphics.queue);
                        app.simulation.reset_average_energy(&app.graphics.queue);
                    }
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
            .update(gc, &mut encoder, gc.window.inner_size());

        let output = gc.surface.get_current_texture().unwrap();
        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        self.renderer
            .render(self, &mut encoder, &context_buffer, &view);
        self.egui.render(gc, &mut encoder, &view, |ctx| {
            self.gui.ui(ctx, gc, &mut self.simulation, &mut self.fps);
        });

        // let buffer = self.simulation.download_avg_energy(&gc, &mut encoder);

        gc.queue.submit([encoder.finish()]);

        // if self.simulation.tick >= 10000 {
        //     static mut FIRST: bool = true;

        //     if unsafe { FIRST } {
        //         unsafe { FIRST = false };

        //         let slice = buffer.slice(..);
        //         let (tx, rx) = crossbeam_channel::bounded(1);
        //         slice.map_async(MapMode::Read, move |_| tx.send(()).unwrap());

        //         gc.device.poll(Maintain::Wait);

        //         rx.recv().unwrap();
        //         let mapped = slice.get_mapped_range();
        //         let data = bytemuck::cast_slice::<_, f32>(&mapped);

        //         let size = self.simulation.get_size();
        //         let mut out = Vec::<f32>::with_capacity((size.1) as usize);
        //         for y in 0..size.1 {
        //             let idx = (y * size.0 + size.0 - 2 - 400) as usize;
        //             out.push(data[idx]);
        //         }
        //         drop(mapped);
        //         buffer.unmap();

        //         fs::write("out.f32", bytemuck::cast_slice(&out)).unwrap();

        //         self.simulation.running = false;
        //     }
        // }

        output.present();
        gc.window.request_redraw();

        if self.gui.queue_screenshot {
            self.gui.queue_screenshot = false;
            if let Err(e) = self.renderer.screenshot(self) {
                eprintln!("Failed to take screenshot: {:?}", e);
            }
        }

        self.fps.interval.tick();
    }

    fn configure_surface(&mut self) {
        let size = self.graphics.window.inner_size();
        self.graphics.surface.configure(
            &self.graphics.device,
            &SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format: TEXTURE_FORMAT,
                width: size.width,
                height: size.height,
                present_mode: PresentMode::Immediate,
                desired_maximum_frame_latency: 2,
                alpha_mode: CompositeAlphaMode::Opaque,
                view_formats: vec![],
            },
        );
    }
}
