fn tick(x: u32, y: u32, mul: ptr<function, f32>, distance: ptr<function, f32>, c: ptr<function, f32>) {} // Populated at runtime

@group(0) @binding(0) var<uniform> ctx: Context;
@group(0) @binding(1) var<storage> map: array<u32>;

@group(0) @binding(2) var<storage, read_write> states: array<f32>;
@group(0) @binding(3) var<storage, read_write> average_energy: array<f32>;

// #if AUDIO
@group(0) @binding(4) var<storage, read> audio_in: array<f32>;
@group(0) @binding(5) var<storage, read_write> audio_out: array<f32>;
// #endif

struct Context {
    width: u32,
    height: u32,
    window_width: u32,
    window_height: u32,

    tick: u32,
    ticks_per_dispatch: u32,
    flags: u32,
    // 1 << 0: reflective boundary
    // 1 << 1: energy_view

    c: f32,
    amplitude: f32,
    oscillation: f32,
}

fn index(x: u32, y: u32, n: u32) -> u32 {
    return (ctx.width * ctx.height * n) + (y * ctx.width) + x;
}

fn get_map(x: u32, y: u32) -> vec4<u32> {
    let value = map[y * ctx.width + x];
    return vec4<u32>(
        value & 0xFF,
        (value >> 8) & 0xFF,
        (value >> 16) & 0xFF,
        (value >> 24) & 0xFF,
    );
}

@compute
@workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let tick = ctx.tick;
    let x = global_id.x;
    let y = global_id.y;
    let map_value = get_map(x, y);

    var mul = f32(map_value.r == 0);
    var distance = 255.0 - f32(map_value.g);
    var c = pow(ctx.c * (f32(map_value.b) / 255.0 * 2.0), 2.0);
    tick(x, y, &mul, &distance, &c);

    let next = tick % 3;
    let current = (tick + 2) % 3;
    let last = (tick + 1) % 3;

    let ni = index(x, y, next);
    let ci = index(x, y, current);

    if (ctx.flags & 0x01) == 0 {
        if x == 0 {
            states[ni] = states[ci] + (states[index(x + 1, y, current)] - states[ci]) * ctx.c;
            return;
        } else if x == ctx.width - 1 {
            states[ni] = states[ci] - (states[ci] - states[index(x - 1, y, current)]) * ctx.c;
            return;
        } else if y == 0 {
            states[ni] = states[ci] + (states[index(x, y + 1, current)] - states[ci]) * ctx.c;
            return;
        } else if y == ctx.height - 1 {
            states[ni] = states[ci] - (states[ci] - states[index(x, y - 1, current)]) * ctx.c;
            return;
        }
    } else if x == 0 || y == 0 || x == ctx.width - 1 || y == ctx.height - 1 {
        states[ni] = 0.0;
        return;
    }

    states[ni] = 2.0 * states[index(x, y, current)]
        - states[index(x, y, last)]
        + c * (
            states[index(x - 1, y, current)]
            + states[index(x + 1, y, current)]
            + states[index(x, y - 1, current)]
            + states[index(x, y + 1, current)]
            - 4.0 * states[index(x, y, current)]
        );
    states[ni] *= mul;

    // #if OSCILLATOR
    states[ni] += ctx.amplitude * exp(-abs(distance)) * cos(f32(ctx.tick) * ctx.oscillation);
    // #endif

    // #if AUDIO
    if y == AUDIO.y && x == AUDIO.x {
        audio_out[ctx.tick % 512] = states[ni];
    }

    states[ni] += ctx.amplitude * exp(-abs(distance)) * audio_in[ctx.tick];
    // #endif

    let nd = f32(tick) + 1.0;
    average_energy[index(x, y, 0u)] = average_energy[index(x, y, 0u)] * (f32(tick) / nd) + pow(states[ni], 2.0) / nd; 
}
