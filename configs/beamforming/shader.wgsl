let next = ctx.tick % 3;
let i = index(x, y, n);

let n = 15;
let spacing = 3.0;

let start_x = (f32(ctx.width) - spacing * f32(n)) / 2.0;
let center_y = f32(ctx.height) / 2.0;
for (var j = 0; j < n; j++) {
    let emitter = vec2<f32>(f32(start_x + spacing * f32(j)), center_y);
    let distance = distance(emitter, vec2<f32>(f32(x), f32(y)));
    states[i] += 2.0 * 0.1 * exp(-abs(distance)) * cos((f32(ctx.tick) + f32(j) * ctx.amplitude) * ctx.oscillation);
}