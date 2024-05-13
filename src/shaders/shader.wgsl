fn tick(x: u32, y: u32) {} // Populated at runtime

@group(0) @binding(0) var<uniform> ctx: Context;
@group(0) @binding(1) var<storage> map: array<u32>;

@group(0) @binding(2) var<storage, read_write> next_states: array<f32>;
@group(0) @binding(3) var<storage, read_write> states: array<f32>;
@group(0) @binding(4) var<storage, read> last_states: array<f32>;

struct Context {
    width: u32,
    height: u32,
    window_width: u32,
    window_height: u32,
    tick: u32,

    c: f32,
    amplitude: f32,
    oscillation: f32,
}

fn index(x: u32, y: u32) -> u32 {
    return y * ctx.width + x;
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
    let x = global_id.x;
    let y = global_id.y;
    let i = index(x, y);

    // maybe clean this up
    if x == 0 {
        let dx = (states[index(x + 1, y)] - states[i]) * ctx.c;
        next_states[i] = states[i] + dx;
    } else if x == ctx.width - 1 {
        let dx = (states[i] - states[index(x - 1, y)]) * ctx.c;
        next_states[i] = states[i] - dx;
    } else if y == 0 {
        let dy = (states[index(x, y + 1)] - states[i]) * ctx.c;
        next_states[i] = states[i] + dy;
    } else if y == ctx.height - 1 {
        let dy = (states[i] - states[index(x, y - 1)]) * ctx.c;
        next_states[i] = states[i] - dy;
    } else {
        next_states[i] = 2.0 * states[i]
        - last_states[i]
        + pow(ctx.c, 2.0) * (
            states[index(x - 1, y)]
            + states[index(x + 1, y)]
            + states[index(x, y - 1)]
            + states[index(x, y + 1)]
            - 4.0 * states[i]
            );
    }

    tick(x, y);
}