let slit_height = u32(100);
let slit_gap = u32(50);

let i = index(x, y);

// Spinning emitter
let emitter = vec2<f32>(
    f32(ctx.width)  / 2.0 + 500.0 * cos(f32(ctx.tick) / 300.0),
    f32(ctx.height) / 2.0 + 500.0 * sin(f32(ctx.tick) / 300.0)
);
let distance = distance(emitter, vec2<f32>(f32(x), f32(y)));
next_states[i] += 2.0 * ctx.amplitude * exp(-abs(distance)) * cos(f32(ctx.tick) * ctx.oscillation);
