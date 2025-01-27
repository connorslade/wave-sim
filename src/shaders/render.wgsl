@group(0) @binding(0) var<uniform> ctx: Context;
@group(0) @binding(1) var<storage, read> states: array<f32>;
@group(0) @binding(2) var<storage, read> average_energy: array<f32>;

struct Context {
    size: vec2<u32>,
    window: vec2<u32>,

    tick: u32,
    // 1 << 0: reflective boundary
    // 1 << 1: energy_view
    flags: u32,
    gain: f32,
    energy_gain: f32,

    pan: vec2<f32>,
    zoom: f32,
}

// VERTEX SHADER //

struct VertexOutput {
    @builtin(position)
    position: vec4<f32>,
}

@vertex
fn vert(
    @builtin(vertex_index) index: u32,
) -> VertexOutput {
    var positions = array(
        vec4(-1.0, -1.0, 1.0, 1.0),
        vec4(1.0, -1.0, 1.0, 1.0),
        vec4(1.0, 1.0, 1.0, 1.0),
        vec4(-1.0, 1.0, 1.0, 1.0)
    );

    return VertexOutput(positions[index]);
}

// FRAGMENT SHADER //

const COLOR_SCHEME: array<vec3<f32>, 6> = array<vec3<f32>, 6>(
    vec3<f32>(0.0, 0.0, 0.0),
    vec3<f32>(0.4549019607843137, 0.1607843137254902, 0.4588235294117647) ,
    vec3<f32>(0.8666666666666667, 0.33725490196078434, 0.1803921568627451),
    vec3<f32>(0.9921568627450981, 0.592156862745098, 0.09803921568627451) ,
    vec3<f32>(1.0, 0.8431372549019608, 0.4196078431372549),
    vec3<f32>(1.0, 1.0, 1.0),
);

fn index(x: u32, y: u32, n: u32) -> u32 {
    return (ctx.size.x * ctx.size.y * n) + (y * ctx.size.x) + x;
}

fn nearest_neighbor_sample_avg_energy(pos: vec2<i32>) -> f32 {
    return average_energy[u32(pos.y) * ctx.size.x + u32(pos.x)];
}

fn bilinear_sample_avg_energy(origin: vec2<f32>, pos: vec2<i32>) -> f32 {
    let x = vec2<i32>(max(0, min(i32(ctx.size.x) - 1, pos.x)), max(0, min(i32(ctx.size.x) - 1, pos.x + 1)));
    let y = vec2<i32>(max(0, min(i32(ctx.size.y) - 1, pos.y)), max(0, min(i32(ctx.size.y) - 1, pos.y + 1)));

    let s = vec4<f32>(
        nearest_neighbor_sample_avg_energy(vec2<i32>(x.x, y.x)),
        nearest_neighbor_sample_avg_energy(vec2<i32>(x.y, y.x)),
        nearest_neighbor_sample_avg_energy(vec2<i32>(x.x, y.y)),
        nearest_neighbor_sample_avg_energy(vec2<i32>(x.y, y.y))
    );

    let f = fract((origin - ctx.pan) * ctx.zoom);
    return mix(mix(s.x, s.y, f.x), mix(s.z, s.w, f.x), f.y);
}

fn sample_avg_energy(origin: vec2<f32>, pos: vec2<i32>) -> f32 {
    if (ctx.flags & 0x04) != 0 {
        return bilinear_sample_avg_energy(origin, pos);
    } else {
        return nearest_neighbor_sample_avg_energy(pos);
    }
}

fn nearest_neighbor_sample(pos: vec2<i32>, tick: u32) -> f32 {
    return states[index(u32(pos.x), u32(pos.y), tick % 3)];
}

fn bilinear_sample(origin: vec2<f32>, pos: vec2<i32>, tick: u32) -> f32 {
    let x = vec2<i32>(max(0, min(i32(ctx.size.x) - 1, pos.x)), max(0, min(i32(ctx.size.x) - 1, pos.x + 1)));
    let y = vec2<i32>(max(0, min(i32(ctx.size.y) - 1, pos.y)), max(0, min(i32(ctx.size.y) - 1, pos.y + 1)));

    let s = vec4<f32>(
        nearest_neighbor_sample(vec2<i32>(x.x, y.x), tick),
        nearest_neighbor_sample(vec2<i32>(x.y, y.x), tick),
        nearest_neighbor_sample(vec2<i32>(x.x, y.y), tick),
        nearest_neighbor_sample(vec2<i32>(x.y, y.y), tick)
    );

    let f = fract((origin - ctx.pan) * ctx.zoom);
    return mix(mix(s.x, s.y, f.x), mix(s.z, s.w, f.x), f.y);
}

fn sample(origin: vec2<f32>, pos: vec2<i32>) -> f32 {
    if (ctx.flags & 0x04) != 0 {
        return bilinear_sample(origin, pos, ctx.tick);
    } else {
        return nearest_neighbor_sample(pos, ctx.tick);
    }
}

@fragment
fn frag(in: VertexOutput) -> @location(0) vec4<f32> {
    let pos = vec2<i32>((in.position.xy - ctx.pan)  * ctx.zoom );

    if pos.x == -1 || pos.y == -1 || pos.x == i32(ctx.size.x) || pos.y == i32(ctx.size.y) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    } else if pos.x < 0 || pos.x > i32(ctx.size.x) || pos.y < 0 || pos.y > i32(ctx.size.y) {
        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
    }

    if (ctx.flags & 0x02) != 0 {
        var val = clamp(sample_avg_energy(in.position.xy, pos) * ctx.energy_gain, 0.0, 1.0);
        let scheme_index = u32(val * 3.0);
        val = val * 3.0 - f32(scheme_index);

        // clean code i swear
        var color = vec3<f32>(0.0, 0.0, 0.0);
        if (scheme_index == 0) {
            color = COLOR_SCHEME[0] * (1.0 - val) + COLOR_SCHEME[1] * val;
        } else if (scheme_index == 1) {
            color = COLOR_SCHEME[1] * (1.0 - val) + COLOR_SCHEME[2] * val;
        } else if (scheme_index == 2) {
            color = COLOR_SCHEME[2] * (1.0 - val) + COLOR_SCHEME[3] * val;
        } else if (scheme_index == 3) {
            color = COLOR_SCHEME[3] * (1.0 - val) + COLOR_SCHEME[4] * val;
        } else if (scheme_index == 4) {
            color = COLOR_SCHEME[4] * (1.0 - val) + COLOR_SCHEME[5] * val;
        }

        return vec4<f32>(color, 1.0);
    }

    let val = sample(in.position.xy, pos) * ctx.gain;
    let color = (
          vec3<f32>(0.0, 0.0, 1.0) * f32(val > 0.0)
        + vec3<f32>(1.0, 0.0, 0.0) * f32(val < 0.0)
    );

    let aval = abs(val);
    return vec4<f32>(color * aval + (1 - aval), 1.0);
}
