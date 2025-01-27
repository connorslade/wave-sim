let slit_height = u32(100);
let slit_gap = u32(50);

let i = index(x, y, ctx.tick % 3);

// Spinning emitter
let emitter = vec2<f32>(
    f32(ctx.size.x)  / 2.0 + 500.0 * cos(f32(ctx.tick) / 300.0),
    f32(ctx.height) / 2.0 + 500.0 * sin(f32(ctx.tick) / 300.0)
);
*distance = distance(emitter, vec2<f32>(f32(x), f32(y)));
