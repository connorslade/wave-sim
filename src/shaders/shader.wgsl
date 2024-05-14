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
    flags: u32,

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
}
