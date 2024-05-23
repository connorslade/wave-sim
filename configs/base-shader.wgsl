// Shader has access to the following variables and functions:
// - `x` and `y` are the coordinates of the current cell
// - `ctx` is the context of the simulation
//     - `ctx.width` and `ctx.height` are the dimensions of the grid
//     - `ctx.window_width` and `ctx.window_height` are the dimensions of the window
//     - `ctx.tick` is the number of time steps since the start of the simulation
//     - `ctx.c` is the c parameter of the simulation
//     - `ctx.amplitude` is the amplitude parameter of the simulation
//     - `ctx.oscillation` is the oscillation parameter of the simulation
// - `states` is the array of the current states of the cells the next state is t%3, the current state is (t+2)%3, the previous state is (t+1)%3
// - `index(x, y, n)` is a function that returns the index of the cell at coordinates (x, y) with the specified state n 0..3 (see above)
// - `get_map(x, y)` is a function that returns the values (wall, distance, c, not_used) from the loaded map at coordinates (x, y)

let next = ctx.tick % 3;
let i = index(x, y, next);

if x == y {
    states[i] = 0.0;
}