use egui::{Context, ViewportId, Window};
use egui_wgpu::{Renderer, ScreenDescriptor};
use egui_winit::State;
use wgpu::{CommandEncoder, Device, Queue, RenderPassDescriptor, TextureFormat, TextureView};
use winit::event::WindowEvent;

pub struct Egui {
    state: State,
    renderer: Renderer,
}

impl Egui {
    pub fn new(device: &Device, window: &winit::window::Window) -> Self {
        let context = Context::default();

        let state = State::new(context, ViewportId::ROOT, window, None, None);
        let renderer = Renderer::new(device, TextureFormat::Rgba8Unorm, None, 1);

        Self { state, renderer }
    }

    pub fn render(
        &mut self,
        device: &Device,
        queue: &Queue,
        window: &winit::window::Window,
        encoder: &mut CommandEncoder,
        view: &TextureView,
    ) {
        let input = self.state.take_egui_input(window);
        let context = self.state.egui_ctx();

        let window_size = window.inner_size();
        let screen = ScreenDescriptor {
            size_in_pixels: [window_size.width, window_size.height],
            pixels_per_point: window.scale_factor() as f32,
        };

        let output = context.run(input, |ctx| {
            Window::new("Wave Simulator").show(ctx, |ui| {
                ui.heading("it work!");
                if ui.button("click me").clicked() {
                    println!("clicked!");
                }
            });
        });

        let clipped_primitives = context.tessellate(output.shapes, context.pixels_per_point());

        for (id, delta) in output.textures_delta.set {
            self.renderer.update_texture(device, &queue, id, &delta);
        }

        self.state
            .handle_platform_output(window, output.platform_output);

        self.renderer
            .update_buffers(device, queue, encoder, &clipped_primitives, &screen);

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

        // for texture in output.textures_delta.free {
        //     self.renderer.free_texture(&texture);
        // }
    }

    pub fn handle_event(&mut self, window: &winit::window::Window, event: &WindowEvent) {
        let _ = self.state.on_window_event(window, event);
    }
}
