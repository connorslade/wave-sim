@group(0) @binding(0) var<uniform> ctx: Context;

@group(0) @binding(1) var<storage, read_write> next_states: array<f32>;
@group(0) @binding(2) var<storage, read_write> states: array<f32>;
@group(0) @binding(3) var<storage, read_write> last_states: array<f32>;

struct Context {
    width: u32,
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

    let sum = 2.0 * states[i]
        - last_states[i]
        + 0.01 * (
            states[index(x - 1, y)]
            + states[index(x + 1, y)]
            + states[index(x, y - 1)]
            + states[index(x, y + 1)]
            - 4.0 * states[i]
        );

    next_states[i] = sum;
}