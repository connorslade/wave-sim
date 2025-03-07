let current = (ctx.tick + 2) % 3;
let i = index(x, y, current);

for (var j = -1.0; j < 2.0; j += 2.0) {
    let emitter = vec2<f32>(
        f32(ctx.size.x)  / 2.0 + 500.0 * j - f32(ctx.tick) * ctx.oscillation * 10.0 * j,
        f32(ctx.height) / 2.0
    );
    let distance = distance(emitter, vec2<f32>(f32(x), f32(y)));
    states[i] += 2.0 * ctx.amplitude * exp(-abs(distance)) * cos(f32(ctx.tick) * ctx.oscillation);
}
