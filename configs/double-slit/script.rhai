const TICKS = 100;
const ITERATIONS = 10;

let iter = 0;

fn post_tick() {
    if sim.tick >= TICKS {
        sim.snapshot_state();
        sim.reset();

        iter += 1;
        if iter >= ITERATIONS { sim.pause() };
    }
}
