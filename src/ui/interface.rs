use std::time::Instant;

use bitflags::Flags;
use egui::{emath::Numeric, Color32, Context, RichText, Slider, Ui, Window};
use nalgebra::Vector2;

use crate::{
    misc::RingBuffer,
    renderer::Renderer,
    simulation::{Simulation, SimulationFlags, SnapshotType},
    GraphicsContext,
};

use super::sci_dragger::SciDragValue;

pub struct Gui {
    pub queue_screenshot: bool,
    pub show_about: bool,
    fps: FpsTracker,
}

struct FpsTracker {
    fps_history: RingBuffer<f64, 256>,
    last_frame: Instant,
}

const COURANT_TIP: &str =
    "When the Courant number is not in (0, 0.7], the simulation may become unstable.";
const DESCRIPTION: &str = "Wave Simulator is a GPU accelerated simulator for the discretized wave equation. Created by Connor Slade.";

impl Gui {
    pub fn ui(
        &mut self,
        ctx: &Context,
        gc: &GraphicsContext,
        simulation: &mut Simulation,
        render: &mut Renderer,
    ) {
        let now = Instant::now();
        let frame_time = now - self.fps.last_frame;
        self.fps.last_frame = now;

        let dragging_viewport = ctx.dragged_id().is_none() && !ctx.is_pointer_over_area();
        let scale_factor = gc.window.scale_factor() as f32;

        ctx.input(|input| {
            let pointer = input.pointer.latest_pos().unwrap_or_default();
            let pointer = Vector2::new(pointer.x, pointer.y) * scale_factor;

            let old_zoom = render.zoom;
            render.zoom = (old_zoom - input.smooth_scroll_delta.y / 1000.0).max(0.05);
            render.pan += (pointer - render.pan) * (1.0 - (old_zoom.powi(2) / render.zoom.powi(2)));

            if input.pointer.any_down() && dragging_viewport {
                let delta = input.pointer.delta() * scale_factor;
                render.pan += Vector2::new(delta.x, delta.y);
            }
        });

        Window::new("Wave Simulator")
            .default_width(0.0)
            .show(ctx, |ui| {
                let size = simulation.get_size();
                let current_fps = frame_time.as_secs_f64().recip();
                self.fps.fps_history.push(current_fps);
                let avg_fps = self.fps.fps_history.avg();

                let (shift, ctrl) = ui.input(|i| (i.modifiers.shift, i.modifiers.ctrl));

                ui.label(format!("Domain: {}×{}", size.x, size.y));
                ui.horizontal(|ui| {
                    ui.label(format!("FPS: {avg_fps:.1}"));
                    ui.label(format!(
                        "UPS: {:.1}",
                        avg_fps * simulation.ticks_per_dispatch as f64
                    ));
                });
                ui.label(format!("Tick: {}", simulation.tick));

                ui.separator();

                ui.add(
                    Slider::new(&mut simulation.ticks_per_dispatch, 1..=32)
                        .text("Ticks per Dispatch"),
                );

                ui.separator();

                ui.collapsing("Viewport", |ui| {
                    sci_dragger(ui, "Gain", &mut simulation.gain);
                    sci_dragger(ui, "Energy Gain", &mut simulation.energy_gain);

                    ui.separator();

                    bit_checkbox(
                        ui,
                        "Energy View",
                        &mut simulation.flags,
                        SimulationFlags::ENERGY_VIEW,
                    );
                    bit_checkbox(
                        ui,
                        "Smooth Sampling",
                        &mut simulation.flags,
                        SimulationFlags::BILINIER_SAMPLING,
                    );
                });

                ui.collapsing("Simulation", |ui| {
                    bit_checkbox(
                        ui,
                        "Reflective Boundaries",
                        &mut simulation.flags,
                        SimulationFlags::REFLECTIVE_BOUNDARY,
                    );

                    ui.separator();

                    sci_dragger(ui, "dx (m)", &mut simulation.dx);
                    sci_dragger(ui, "dt (s)", &mut simulation.dt);
                    sci_dragger(ui, "Wave Speed (m/s)", &mut simulation.v);

                    let c = simulation.dt * simulation.v / simulation.dx;
                    ui.horizontal(|ui| {
                        ui.label(format!("Courant: {c:.2}"));
                        if c > 0.7 {
                            ui.label(RichText::new("CFL not met. (c < 0.7)").color(Color32::RED))
                                .on_hover_text(COURANT_TIP);
                        } else if c == 0.0 {
                            ui.label(RichText::new("C is zero.").color(Color32::RED))
                                .on_hover_text(COURANT_TIP);
                        }
                    });
                });

                ui.collapsing("Oscillator", |ui| {
                    sci_dragger(ui, "Amplitude", &mut simulation.amplitude);
                    sci_dragger(ui, "Frequency (Hz)", &mut simulation.frequency);
                });

                ui.separator();

                ui.horizontal(|ui| {
                    simulation.running ^= ui
                        .button(if simulation.running { "⏸" } else { "▶" })
                        .on_hover_text(if simulation.running {
                            "Pause simulation (Space)"
                        } else {
                            "Resume simulation (Space)"
                        })
                        .clicked();

                    if ui
                        .button("⟳")
                        .on_hover_text(
                            "Reset simulation (R)\nHold shift to only reset average energy.",
                        )
                        .clicked()
                    {
                        simulation.reset_average_energy(&gc.queue);
                        if !shift {
                            simulation.reset_states(&gc.queue);
                        }
                    }

                    if ui
                        .button("📷")
                        .on_hover_text("Screenshot\nHold shift for state and ctrl for avg energy.")
                        .clicked()
                    {
                        if shift {
                            simulation.queue_snapshot = SnapshotType::State;
                        } else if ctrl {
                            simulation.queue_snapshot = SnapshotType::Energy;
                        } else {
                            self.queue_screenshot = true;
                        }
                    }

                    self.show_about ^= ui.button("ℹ").on_hover_text("About").clicked();
                });
            });

        if self.show_about {
            Window::new("About").show(ctx, |ui| {
                ui.label(DESCRIPTION);
                ui.spacing();
                ui.horizontal(|ui| {
                    ui.label("Github:");
                    ui.hyperlink_to(
                        "@connorslade/wave-sim",
                        "https://github.com/connorslade/wave-sim",
                    );
                })
            });
        }
    }
}

fn sci_dragger<Num: Numeric>(ui: &mut Ui, label: &str, value: &mut Num) {
    ui.horizontal(|ui| {
        SciDragValue::new(value).show(ui);
        ui.label(label);
    });
}

fn bit_checkbox<Value: Flags + Copy>(ui: &mut Ui, label: &str, value: &mut Value, flag: Value) {
    let mut bool_value = value.contains(flag);
    ui.checkbox(&mut bool_value, label);
    value.set(flag, bool_value);
}

impl Default for Gui {
    fn default() -> Self {
        Self {
            queue_screenshot: false,
            show_about: false,
            fps: FpsTracker {
                fps_history: RingBuffer::new(),
                last_frame: Instant::now(),
            },
        }
    }
}
