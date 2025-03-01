const TICKS = 100;

fn post_tick() {
    if sim.tick >= TICKS { sim.running = false; }
}
