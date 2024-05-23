let slit_height = 100u;
let slit_gap = 50u;

let next = ctx.tick % 3;
let i = index(x, y, next);

// The slits
let slit_x = ctx.width / 3;
let slit_y = ctx.height / 2;

let slit = x == slit_x
    && !(y > slit_y - slit_height - slit_gap && y < slit_y - slit_gap)
    && !(y > slit_y + slit_gap && y < slit_y + slit_gap + slit_height);
states[i] *= f32(!slit);

// Liniar emitter on left wall
states[i] += ctx.amplitude * cos(f32(ctx.tick) * ctx.oscillation) * f32(x == 1);