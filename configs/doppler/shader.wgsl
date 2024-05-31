// Damping
states[index(x, y, ctx.tick % 3)] *= 1 - 0.0001;

// Emitter
let emitter = vec2<f32>(
    f32(ctx.width)  / 2.0 + 700.0 * (f32(ctx.tick) / 32000f - 0.5),
    f32(ctx.height) / 2.0
);
*distance = distance(vec2<f32>(f32(x), f32(y)), emitter);