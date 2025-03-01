use std::sync::Arc;

use anyhow::{Context, Result};
use app::{App, GraphicsContext};
use image::ImageFormat;
use ui::egui::Egui;
use wgpu::{Instance, RequestAdapterOptions};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Icon, WindowBuilder},
};

mod app;
mod config;
mod misc;
mod renderer;
mod scripting;
mod simulation;
mod ui;
use renderer::Renderer;
use simulation::{Simulation, SimulationFlags};

const ICON: &[u8] = include_bytes!("assets/icon.png");

#[pollster::main]
async fn main() -> Result<()> {
    let args = config::parse()?;

    let instance = Instance::default();
    let adapter = instance
        .request_adapter(&RequestAdapterOptions::default())
        .await
        .context("No adapter found")?;
    let (device, queue) = adapter.request_device(&Default::default(), None).await?;

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

    let mut app = App {
        simulation,
        renderer,
        egui: Egui::new(&device, &window),
        gui: Default::default(),
        graphics: GraphicsContext {
            window,
            surface,
            device,
            queue,
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
