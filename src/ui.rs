use std::time::{Duration, Instant};

use bitflags::Flags;
use egui::{emath::Numeric, Color32, Context, DragValue, RichText, Slider, Ui, Window};

use crate::{
    simulation::{Simulation, SimulationFlags},
    FpsTracker, GraphicsContext,
};

pub struct Gui {
    pub queue_screenshot: bool,
    pub show_about: bool,
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
        fps: &mut FpsTracker,
    ) {
        let now = Instant::now();
        let frame_time = now - fps.last_frame;
        fps.last_frame = now;

        Window::new("Wave Simulator")
            .default_width(0.0)
            .show(ctx, |ui| {
                let size = simulation.get_size();
                let current_fps = frame_time.as_secs_f64().recip();
                fps.fps_history.push(current_fps);
                let avg_fps = fps.fps_history.avg();

                ui.label(format!("Size: {}x{}", size.0, size.1));
                ui.label(format!("FPS: {avg_fps:.1}"));
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

                if last_target_fps != fps.target_fps {
                    fps.fps_history.reset();
                    fps.interval
                        .set_period(Duration::from_secs_f64(1.0 / fps.target_fps as f64));
                }

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
                        if ui.input(|i| i.modifiers.shift) {
                            simulation.reset_average_energy(&gc.queue);
                        } else {
                            simulation.reset_states(&gc.queue);
                        }
                    }

                    self.queue_screenshot |= ui.button("📷").on_hover_text("Screenshot").clicked();

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
