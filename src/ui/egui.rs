use egui::{Context, ViewportId};
use egui_wgpu::{Renderer, ScreenDescriptor};
use egui_winit::State;
use wgpu::{CommandEncoder, Device, RenderPassDescriptor, TextureView};
use winit::event::WindowEvent;

use crate::{app::TEXTURE_FORMAT, GraphicsContext};

pub struct Egui {
    state: State,
    renderer: Renderer,
}

impl Egui {
    pub fn new(device: &Device, window: &winit::window::Window) -> Self {
        let context = Context::default();

        let state = State::new(context, ViewportId::ROOT, window, None, None);
        let renderer = Renderer::new(device, TEXTURE_FORMAT, None, 1);

        Self { state, renderer }
    }

    pub fn render(
        &mut self,
        gc: &GraphicsContext,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        run_ui: impl FnOnce(&egui::Context),
    ) {
        let input = self.state.take_egui_input(&gc.window);
        let context = self.state.egui_ctx();

        let window_size = gc.window.inner_size();
        let screen = ScreenDescriptor {
            size_in_pixels: [window_size.width, window_size.height],
            pixels_per_point: gc.window.scale_factor() as f32,
        };

        let output = context.run(input, run_ui);

        let clipped_primitives = context.tessellate(output.shapes, context.pixels_per_point());

        for (id, delta) in output.textures_delta.set {
            self.renderer
                .update_texture(&gc.device, &gc.queue, id, &delta);
        }

        self.state
            .handle_platform_output(&gc.window, output.platform_output);

        self.renderer
            .update_buffers(&gc.device, &gc.queue, encoder, &clipped_primitives, &screen);

        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        self.renderer
            .render(&mut render_pass, &clipped_primitives, &screen);

        drop(render_pass);
        for texture in output.textures_delta.free {
            self.renderer.free_texture(&texture);
        }
    }

    pub fn handle_event(&mut self, gc: &GraphicsContext, event: &WindowEvent) {
        let _ = self.state.on_window_event(&gc.window, event);
    }
}
