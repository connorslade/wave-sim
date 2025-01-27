let slit_height = 100u;
let slit_gap = 50u;

let next = ctx.tick % 3;
let i = index(x, y, next);

// The slits
let slit_x = ctx.size.x / 3;
let slit_y = ctx.size.y / 2;

let slit = x == slit_x
    && !(y > slit_y - slit_height - slit_gap && y < slit_y - slit_gap)
    && !(y > slit_y + slit_gap && y < slit_y + slit_gap + slit_height);
*mul = f32(!slit);

// Liniar emitter on left wall
*distance = f32(x);
