@group(0) @binding(0) var<uniform> ctx: Context;
@group(0) @binding(1) var<storage, read> states: array<f32>;

struct Context {
    width: u32,
    height: u32,
    window_width: u32,
    window_height: u32,
}

// VERTEX SHADER //

struct VertexOutput {
    @builtin(position)
    position: vec4<f32>,

    @location(0)
    tex_coord: vec2<f32>,
};

@vertex
fn vert(
    @location(0) position: vec4<f32>,
    @location(1) tex_coord: vec2<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.position = position;
    out.tex_coord = tex_coord;
    return out;
}

// FRAGMENT SHADER //

@fragment
fn frag(in: VertexOutput) -> @location(0) vec4<f32> {
    let x_offset = ctx.window_width / 2 - ctx.width / 2;
    let y_offset = ctx.window_height / 2 - ctx.height / 2;
    let x = i32(in.position.x) - i32(x_offset);
    let y = i32(in.position.y) - i32(y_offset);


    if x == -1 || y == -1 || x == i32(ctx.width) || y == i32(ctx.height) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    } else if x < 0 || x > i32(ctx.width) || y < 0 || y > i32(ctx.height) {
        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
    }

    let val = states[u32(y) * ctx.width + u32(x)];
    let color = (
          vec3<f32>(0.0, 0.0, 1.0) * f32(val > 0.0)
        + vec3<f32>(1.0, 0.0, 0.0) * f32(val < 0.0)
    );

    let aval = abs(val);
    return vec4<f32>(color * aval + (1 - aval), 1.0);
}