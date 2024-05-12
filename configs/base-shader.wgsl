// Shader has access to the following variables and functions:
// - `x` and `y` are the coordinates of the current cell
// - `ctx` is the context of the simulation
//     - `ctx.width` and `ctx.height` are the dimensions of the grid
//     - `ctx.window_width` and `ctx.window_height` are the dimensions of the window
//     - `ctx.time` is the number of time steps since the start of the simulation
//     - `ctx.c` is the c parameter of the simulation
//     - `ctx.amplitude` is the amplitude parameter of the simulation
//     - `ctx.oscillation` is the oscillation parameter of the simulation
// - `next_states` is the array of the next states of the cells
// - `states` is the array of the current states of the cells
// - `last_states` is the array of the states of the cells at the last time step
// - `index(x, y)` is a function that returns the index of the cell at coordinates (x, y)
// - `get_map(x, y)` is a function that returns the values (wall, distance, c, not_used) from the loaded map at coordinates (x, y)

let i = index(x, y);
if x == y {
    next_states[i] = 0.0;
}