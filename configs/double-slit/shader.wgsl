let slit_height = u32(100);
let slit_gap = u32(50);

let i = index(x, y);

// Boundary conditions (reflective) and slits
let slit_x = ctx.width / 3;
let slit_y = ctx.height / 2;

let wall = (x == 0 || y == 0 || x == ctx.width - 1 || y == ctx.height - 1)
    || x == slit_x
    && !(y > slit_y - slit_height - slit_gap && y < slit_y - slit_gap)
    && !(y > slit_y + slit_gap && y < slit_y + slit_gap + slit_height);
next_states[i] *= f32(!wall);

// Liniar emitter on left wall
next_states[i] += ctx.amplitude * cos(f32(ctx.tick) / ctx.oscillation) * f32(x == 1);