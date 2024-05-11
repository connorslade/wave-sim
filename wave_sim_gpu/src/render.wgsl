@group(0) @binding(0) var<uniform> ctx: Context;
@group(0) @binding(1) var<storage, read> states: array<f32>;

struct Context {
    width: u32,
    height: u32,
    tick: u32,
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
    let x: u32 = u32(in.position.x);
    let y: u32 = u32(in.position.y);

    var val = states[y * ctx.width + x];
    var color = vec3<f32>(0.0, 0.0, 0.0);
    
    if val > 0.0 {
        color = vec3<f32>(0.0, 0.0, 1.0);
    } else {
        color = vec3<f32>(1.0, 0.0, 0.0);
    }

    val = abs(val);
    return vec4<f32>(color * val + (1 - val), 1.0);
}