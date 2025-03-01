// Wavelength config (nm)
const WAVE_START = 310;
const WAVE_END = 1100;
const WAVE_STEP = 10;

const TICKS = 6000;
const ITERATIONS = (WAVE_END - WAVE_START) / WAVE_STEP;

let iter = 0;
let took_snapshot = false;

fn init() { sim.user = configure(sim, 0); }

fn update() {
    if sim.tick >= TICKS && !took_snapshot {
        let wavelength = WAVE_START + WAVE_STEP * iter;
        sim.snapshot_energy("wl-" + wavelength + "nm.bin");
        took_snapshot = true;
    }

    if sim.tick >= TICKS + 100 && took_snapshot {
        took_snapshot = false;
        sim.reset();

        iter += 1;
        if iter >= ITERATIONS {
            print("Done!");
            sim.pause();
        } else {
            sim.user = configure(sim, iter);
        }
    }
}

fn configure(sim, iter) {
    let wavelength = global::WAVE_START + (global::WAVE_STEP * iter);
    let wave_speed = sellmeier_equation(wavelength);
    print("λ=" + wavelength + "nm, v=" + wave_speed + "m/s");
    wave_speed * (sim.dt / sim.dx)
}

fn sellmeier_equation(wl) {
    const B_1 = 1.03961212;
    const B_2 = 1.03961212;
    const B_3 = 1.01046945;

    const C_1 = 6.00069867e-3;
    const C_2 = 2.00179144e-2;
    const C_3 = 1.03560653e2;

    const C = 299_792_458.0;

    let sq_wl = pow(wl / 1000.0, 2.0);
    return C / sqrt(
        1 + (B_1 * sq_wl) / (sq_wl - C_1)
            + (B_2 * sq_wl) / (sq_wl - C_2)
            + (B_3 * sq_wl) / (sq_wl - C_3)
    );
}
