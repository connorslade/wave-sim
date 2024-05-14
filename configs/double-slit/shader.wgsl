let slit_height = u32(100);
let slit_gap = u32(50);

let i = index(x, y);

// The slits
let slit_x = ctx.width / 3;
let slit_y = ctx.height / 2;

let slit = x == slit_x
    && !(y > slit_y - slit_height - slit_gap && y < slit_y - slit_gap)
    && !(y > slit_y + slit_gap && y < slit_y + slit_gap + slit_height);
next_states[i] *= f32(!slit);

// Liniar emitter on left wall
next_states[i] += ctx.amplitude * cos(f32(ctx.tick) * ctx.oscillation) * f32(x == 1);