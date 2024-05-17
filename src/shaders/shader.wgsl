fn tick(x: u32, y: u32) {} // Populated at runtime

@group(0) @binding(0) var<uniform> ctx: Context;
@group(0) @binding(1) var<storage> map: array<u32>;

@group(0) @binding(2) var<storage, read_write> next_states: array<f32>;
@group(0) @binding(3) var<storage, read> states: array<f32>;
@group(0) @binding(4) var<storage, read> last_states: array<f32>;

@group(0) @binding(5) var<storage, read_write> average_energy: array<f32>;

struct Context {
    width: u32,
    height: u32,
    window_width: u32,
    window_height: u32,

    tick: u32,
    flags: u32,
    // 1 << 0: reflective boundary
    // 1 << 1: energy_view

    c: f32,
    amplitude: f32,
    oscillation: f32,
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

    if (ctx.flags & 0x01) == 0 {
        if x == 0 {
            next_states[i] = states[i] + (states[index(x + 1, y)] - states[i]) * ctx.c;
            return;
        } else if x == ctx.width - 1 {
            next_states[i] = states[i] - (states[i] - states[index(x - 1, y)]) * ctx.c;
            return;
        } else if y == 0 {
            next_states[i] = states[i] + (states[index(x, y + 1)] - states[i]) * ctx.c;
            return;
        } else if y == ctx.height - 1 {
            next_states[i] = states[i] - (states[i] - states[index(x, y - 1)]) * ctx.c;
            return;
        }
    } else if x == 0 || y == 0 || x == ctx.width - 1 || y == ctx.height - 1 {
        next_states[i] = 0.0;
        return;
    }

    next_states[i] = 2.0 * states[i]
        - last_states[i]
        + pow(ctx.c, 2.0) * (
            states[index(x - 1, y)]
            + states[index(x + 1, y)]
            + states[index(x, y - 1)]
            + states[index(x, y + 1)]
            - 4.0 * states[i]
        );

    tick(x, y);

    let nd = f32(ctx.tick) + 1.0;
    average_energy[i] = average_energy[i] * (f32(ctx.tick) / nd) + pow(next_states[i], 2.0) / nd; 
}
