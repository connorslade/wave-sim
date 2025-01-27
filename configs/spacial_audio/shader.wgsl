// Damping
*mul = 1 - 0.001;

// Emitter
let emitter = vec2<f32>(
    f32(ctx.size.x)  / 2.0 + 100.0 * cos(0.0000157079632679489661923 * f32(ctx.tick)),
    f32(ctx.height) / 2.0 + 100.0 * sin(0.0000157079632679489661923 * f32(ctx.tick))
);
*distance = distance(vec2<f32>(f32(x), f32(y)), emitter);
