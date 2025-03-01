const TICKS = 6000;
const ITERATIONS = 10;

let iter = 0;

fn update() {
    let wave_speed = 200_000_000.0;
    sim.user = wave_speed * (sim.dt / sim.dx);

    if sim.tick >= TICKS {
        sim.snapshot_state();
        sim.reset();

        iter += 1;
        if iter >= ITERATIONS { sim.pause() };
    }
}
