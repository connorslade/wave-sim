using Images
using Colors

STATE_PATH = "states/biconvex-lens"

WIDTH = 1440
HEIGHT = 900

function load_state(name)
	state = read(STATE_PATH * "/" * name)
	return reshape(reinterpret(Float32, state[9:end]), (WIDTH, HEIGHT))
end

states = readdir(STATE_PATH)
out = zeros(RGB, WIDTH, HEIGHT)

for state in states
    if !isfile(STATE_PATH * "/" * state)
        continue
    end

    println("Processing $(state)...")

    frequency = parse(Float64, state[1:length(state) - 4]) * 100 # in THz
    color = RGB(colormatch(299792.458 / frequency))

    state = clamp.(load_state(state), 0, 1)
    colored_state = RGB.(state .* color.r, state .* color.g, state .* color.b)

    for x in 1:WIDTH
        for y in 1:HEIGHT
            out[x, y] += colored_state[x, y]
        end
    end
end

for pixel in CartesianIndices(out)
    out[pixel] = RGB(clamp(out[pixel].r, 0, 1), clamp(out[pixel].g, 0, 1), clamp(out[pixel].b, 0, 1))
end

save("output_image.png", out)
