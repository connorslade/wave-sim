using Meshes
import CairoMakie as Mke

STATE = "states/energy-4000.bin"
WIDTH = 1440
HEIGHT = 900

raw_state = read(STATE)
state = reshape(reinterpret(Float32, raw_state[9:end]), (WIDTH, HEIGHT))[:, 5:end]

grid = CartesianGrid(1440, 900)

# for x in 1:WIDTH
#     for y in 1:HEIGHT
#         grid[x, y] = state[x, y]
#     end
# end


viz(grid)

