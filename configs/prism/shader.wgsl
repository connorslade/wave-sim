let speed_mult = f32(get_map(x, y).b) / 255.0 * 2.0;

// Use custom wave speed for the lens.
if speed_mult < 1.0 { *c = bitcast<f32>(ctx.user); }
