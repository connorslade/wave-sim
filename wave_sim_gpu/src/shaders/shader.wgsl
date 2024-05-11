@group(0) @binding(0) var<uniform> ctx: Context;

@group(0) @binding(1) var<storage, read_write> next_states: array<f32>;
@group(0) @binding(2) var<storage, read> states: array<f32>;
@group(0) @binding(3) var<storage, read> last_states: array<f32>;

struct Context {
    width: u32,
    height: u32,
    tick: u32,
    c: f32
}

fn index(x: u32, y: u32) -> u32 {
    return y * ctx.width + x;
}

@compute
@workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    let i = index(x, y);

   let mul = f32(x > 0 && x < ctx.width - 1 && y > 0 && y < ctx.height - 1);
    next_states[i] = 2.0 * states[i]
        - last_states[i]
        + ctx.c * (
            states[index(x - 1, y)]
            + states[index(x + 1, y)]
            + states[index(x, y - 1)]
            + states[index(x, y + 1)]
            - 4.0 * states[i]
        ) * mul;

    // var muls = array<f32, 5>(30.0, 10.0, 8.0, 5.0, 4.0);
    // var length = 5;

    // for (var j = 0; j < length; j++) {
    //     let center = vec2<f32>(f32(ctx.width) / 2.0 - 800.0 + f32(ctx.tick) / muls[j], 256.0 + (1792.0 / f32(length)) * f32(j));
    //     let distance = length(center - vec2<f32>(f32(x), f32(y)));
    //     next_states[i] += 0.03 * exp(-distance) * cos(f32(ctx.tick) / 30.0);
    // }

    let center = vec2<f32>(f32(ctx.width) / 2.0 + 512.0 * cos(f32(ctx.tick) / 300.0), f32(ctx.height) / 2.0 + 512.0 * sin(f32(ctx.tick) / 300.0));
    let distance = length(center - vec2<f32>(f32(x), f32(y)));
    next_states[i] += 0.03 * exp(-distance) * cos(f32(ctx.tick) / 30.0);
}