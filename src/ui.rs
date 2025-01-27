use std::time::{Duration, Instant};

use bitflags::Flags;
use egui::{emath::Numeric, Color32, Context, DragValue, RichText, Slider, Ui, Window};
use nalgebra::Vector2;
use wgpu::{Buffer, CommandEncoder};

use crate::{
    renderer::Renderer,
    simulation::{Simulation, SimulationFlags},
    FpsTracker, GraphicsContext,
};

pub struct Gui {
    pub queue_screenshot: bool,
    pub queue_snapshot: SnapshotType,
    pub show_about: bool,
}

pub enum SnapshotType {
    None,
    State,
    Energy,
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
        fps: &mut FpsTracker,
    ) {
        let now = Instant::now();
        let frame_time = now - fps.last_frame;
        fps.last_frame = now;

        let dragging_viewport = ctx.dragged_id().is_none();
        let scale_factor = gc.window.scale_factor() as f32;

        ctx.input(|input| {
            let pointer = input.pointer.latest_pos().unwrap_or_default();
            let pointer = Vector2::new(pointer.x, pointer.y) * scale_factor;

            let old_zoom = render.zoom;
            render.zoom = (old_zoom + input.smooth_scroll_delta.y / 1000.0).max(0.05);
            render.pan += (pointer - render.pan) * (1.0 - (old_zoom / render.zoom));

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
                fps.fps_history.push(current_fps);
                let avg_fps = fps.fps_history.avg();

                let (shift, ctrl) = ui.input(|i| (i.modifiers.shift, i.modifiers.ctrl));

                ui.label(format!("Size: {}x{}", size.x, size.y));
                ui.horizontal(|ui| {
                    ui.label(format!("FPS: {avg_fps:.1}"));
                    ui.label(format!(
                        "UPS: {:.1}",
                        avg_fps * simulation.ticks_per_dispatch as f64
                    ));
                });
                ui.label(format!("Tick: {}", simulation.tick));

                let c = 0.002 * simulation.dt * simulation.v / simulation.dx;
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

                ui.separator();

                let last_target_fps = fps.target_fps;
                ui.add(Slider::new(&mut fps.target_fps, 30..=1000).text("Target FPS"));

                if last_target_fps != fps.target_fps {
                    fps.fps_history.reset();
                    fps.interval
                        .set_period(Duration::from_secs_f64(1.0 / fps.target_fps as f64));
                }

                ui.add(
                    Slider::new(&mut simulation.ticks_per_dispatch, 1..=32)
                        .text("Ticks per Dispatch"),
                );

                ui.columns(2, |ui| {
                    dragger(&mut ui[0], "Gain", &mut simulation.gain, |x| {
                        x.clamp_range(0.0..=f32::MAX).speed(0.1)
                    });

                    dragger(
                        &mut ui[1],
                        "Energy Gain",
                        &mut simulation.energy_gain,
                        |x| x.clamp_range(0.0..=f32::MAX).speed(0.1),
                    );
                });

                bit_checkbox(
                    ui,
                    "Reflective Boundaries",
                    &mut simulation.flags,
                    SimulationFlags::REFLECTIVE_BOUNDARY,
                );
                bit_checkbox(
                    ui,
                    "Energy View",
                    &mut simulation.flags,
                    SimulationFlags::ENERGY_VIEW,
                );

                ui.separator();

                dragger(ui, "dx (m)", &mut simulation.dx, |x| {
                    x.clamp_range(0.0..=f32::MAX).fixed_decimals(4).speed(0.001)
                });
                dragger(ui, "dt (ms)", &mut simulation.dt, |x| {
                    x.clamp_range(0.0..=f32::MAX).fixed_decimals(4).speed(0.001)
                });
                dragger(ui, "Wave Speed", &mut simulation.v, |x| {
                    x.clamp_range(0.0..=f32::MAX)
                });

                ui.separator();

                dragger(ui, "Amplitude", &mut simulation.amplitude, |x| {
                    x.clamp_range(0.0..=f32::MAX).speed(0.001)
                });
                dragger(ui, "Frequency (kHz)", &mut simulation.frequency, |x| {
                    x.clamp_range(0.1..=f32::MAX).speed(0.1)
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
                            self.queue_snapshot = SnapshotType::State;
                        } else if ctrl {
                            self.queue_snapshot = SnapshotType::Energy;
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

fn dragger<Num: Numeric>(
    ui: &mut Ui,
    label: &str,
    value: &mut Num,
    func: fn(DragValue) -> DragValue,
) {
    ui.horizontal(|ui| {
        ui.add(func(DragValue::new(value)));
        ui.label(label);
    });
}

fn bit_checkbox<Value: Flags + Copy>(ui: &mut Ui, label: &str, value: &mut Value, flag: Value) {
    let mut bool_value = value.contains(flag);
    ui.checkbox(&mut bool_value, label);
    value.set(flag, bool_value);
}

impl SnapshotType {
    pub fn name(&self) -> &'static str {
        match self {
            SnapshotType::None => "none",
            SnapshotType::State => "state",
            SnapshotType::Energy => "energy",
        }
    }

    pub fn stage<'a>(
        &self,
        simulation: &'a Simulation,
        encoder: &mut CommandEncoder,
    ) -> Option<&'a Buffer> {
        Some(match self {
            SnapshotType::State => simulation.stage_state(encoder),
            SnapshotType::Energy => simulation.stage_energy(encoder),
            SnapshotType::None => return None,
        })
    }
}
