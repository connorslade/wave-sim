// Damping
*mul = 1 - 0.001;

// Emitter
let emitter = vec2<f32>(
    f32(ctx.width)  / 2.0 + 100.0 * cos(0.00000314159265358979323846 * f32(ctx.tick)),
    f32(ctx.height) / 2.0 + 100.0 * sin(0.00000314159265358979323846 * f32(ctx.tick))
);
*distance = distance(vec2<f32>(f32(x), f32(y)), emitter);