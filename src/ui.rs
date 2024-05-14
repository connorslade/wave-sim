use std::time::{Duration, Instant};

use egui::{emath::Numeric, Color32, Context, DragValue, RichText, Slider, Ui, Window};
use spin_sleep_util::Interval;

use crate::simulation::Simulation;

pub fn ui(
    ctx: &Context,
    queue: &wgpu::Queue,
    simulation: &mut Simulation,
    interval: &mut Interval,
    last_frame: &mut Instant,
    target_fps: &mut u32,
    do_screenshot: &mut bool,
) {
    let now = Instant::now();
    let frame_time = now - *last_frame;
    *last_frame = now;

    Window::new("Wave Simulator")
        .default_width(0.0)
        .show(ctx, |ui| {
            let size = simulation.get_size();
            let fps = frame_time.as_secs_f64().recip();

            ui.label(format!("Size: {}x{}", size.0, size.1));
            ui.label(format!("FPS: {fps:.2}"));
            ui.label(format!("Tick: {}", simulation.tick));

            let c = 0.002 * simulation.dt * simulation.v / simulation.dx;
            ui.horizontal(|ui| {
                ui.label(format!("Courant: {c:.2}"));
                if c > 0.7 {
                    ui.label(RichText::new("CFL not met. (c < 0.7)").color(Color32::RED));
                } else if c == 0.0 {
                    ui.label(RichText::new("C is zero.").color(Color32::RED));
                }
            });

            ui.separator();

            ui.add(Slider::new(target_fps, 30..=1000).text("Target FPS"));

            ui.separator();

            dragger(ui, "dx (m)", &mut simulation.dx, |x| {
                x.clamp_range(0.0..=f32::MAX).fixed_decimals(4)
            });
            dragger(ui, "dt (ms)", &mut simulation.dt, |x| {
                x.clamp_range(0.0..=f32::MAX).fixed_decimals(4)
            });
            dragger(ui, "Wave Speed", &mut simulation.v, |x| {
                x.clamp_range(0.0..=f32::MAX)
            });

            ui.separator();

            dragger(ui, "Amplitude", &mut simulation.amplitude, |x| {
                x.clamp_range(0.0..=f32::MAX).speed(0.1)
            });
            dragger(ui, "Frequency (kHz)", &mut simulation.frequency, |x| {
                x.clamp_range(0.1..=f32::MAX).speed(0.1)
            });

            ui.separator();

            ui.horizontal(|ui| {
                if ui
                    .button(if simulation.running { "⏸" } else { "▶" })
                    .clicked()
                {
                    simulation.running ^= true;
                }

                if ui.button("⟳").clicked() {
                    simulation.reset_states(queue);
                }

                if ui.button("📷").clicked() {
                    *do_screenshot = true;
                }
            });

            interval.set_period(Duration::from_secs_f64(1.0 / *target_fps as f64));
        });
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
