use std::{fs, mem, path::Path, sync::Arc};

use wgpu::{
    CommandEncoderDescriptor, CompositeAlphaMode, Device, PresentMode, Queue, Surface,
    SurfaceConfiguration, TextureFormat, TextureUsages, TextureViewDescriptor,
};
use winit::window::Window;

use crate::{
    misc::util::{download_buffer, save_dated_file},
    renderer::Renderer,
    simulation::Simulation,
    ui::{egui::Egui, interface::Gui},
};

pub const TEXTURE_FORMAT: TextureFormat = TextureFormat::Bgra8Unorm;

pub struct App<'a> {
    pub graphics: GraphicsContext<'a>,
    pub simulation: Simulation,
    pub renderer: Renderer,

    pub egui: Egui,
    pub gui: Gui,
}

pub struct GraphicsContext<'a> {
    pub window: Arc<Window>,
    pub surface: Surface<'a>,
    pub device: Device,
    pub queue: Queue,
}

impl App<'_> {
    pub fn render(&mut self) {
        let gc = &self.graphics;
        let mut encoder = gc
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });

        self.simulation
            .update(gc, &mut encoder, gc.window.inner_size());

        let output = gc.surface.get_current_texture().unwrap();
        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        self.renderer.render(self, &mut encoder, &view);
        self.egui.render(gc, &mut encoder, &view, |ctx| {
            self.gui
                .ui(ctx, gc, &mut self.simulation, &mut self.renderer);
        });

        while let Some((snapshot, name)) = self.simulation.snapshot.pop() {
            let buffer = snapshot.stage(&self.simulation, &mut encoder);

            let mut data = Vec::with_capacity(8 + buffer.size() as usize);
            let size = self.simulation.get_size();
            data.extend_from_slice(&size.x.to_le_bytes());
            data.extend_from_slice(&size.y.to_le_bytes());
            data.extend_from_slice(&download_buffer(buffer, gc));

            let path = if let Some(name) = name {
                Path::new("states").join(name).to_path_buf()
            } else {
                save_dated_file("states", snapshot.name(), "bin").unwrap()
            };

            fs::write(path, data).unwrap();
        }

        gc.queue.submit([encoder.finish()]);

        output.present();
        gc.window.request_redraw();

        if mem::take(&mut self.gui.queue_screenshot) {
            if let Err(e) = self.renderer.screenshot(self) {
                eprintln!("Failed to take screenshot: {:?}", e);
            }
        }
    }

    pub fn configure_surface(&mut self) {
        let size = self.graphics.window.inner_size();
        self.graphics.surface.configure(
            &self.graphics.device,
            &SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format: TEXTURE_FORMAT,
                width: size.width,
                height: size.height,
                present_mode: PresentMode::AutoVsync,
                desired_maximum_frame_latency: 2,
                alpha_mode: CompositeAlphaMode::Opaque,
                view_formats: vec![],
            },
        );
    }
}
