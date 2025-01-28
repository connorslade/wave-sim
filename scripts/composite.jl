using Images

STATE_PATH = "states"

WIDTH = 1920
HEIGHT = 1080

function load_state(name)
	state = read(STATE_PATH * "/" * name)
	return reshape(reinterpret(Float32, state[9:end]), (WIDTH, HEIGHT))
end

function wavelength_to_rgb(wavelength)
    if wavelength >= 380 && wavelength < 440
        r = -(wavelength - 440) / (440 - 380)
        g = 0.0
        b = 1.0
    elseif wavelength >= 440 && wavelength < 490
        r = 0.0
        g = (wavelength - 440) / (490 - 440)
        b = 1.0
    elseif wavelength >= 490 && wavelength < 510
        r = 0.0
        g = 1.0
        b = -(wavelength - 510) / (510 - 490)
    elseif wavelength >= 510 && wavelength < 580
        r = (wavelength - 510) / (580 - 510)
        g = 1.0
        b = 0.0
    elseif wavelength >= 580 && wavelength < 645
        r = 1.0
        g = -(wavelength - 645) / (645 - 580)
        b = 0.0
    elseif wavelength >= 645 && wavelength <= 780
        r = 1.0
        g = 0.0
        b = 0.0
    else
        r = 0.0
        g = 0.0
        b = 0.0
    end

    if wavelength > 780 || wavelength < 380
        factor = 0.0
    elseif wavelength < 420
        factor = 0.3 + 0.7 * (wavelength - 380) / (420 - 380)
    elseif wavelength < 645
        factor = 1.0
    else
        factor = 0.3 + 0.7 * (780 - wavelength) / (780 - 645)
    end

    r = r * factor
    g = g * factor
    b = b * factor

    return RGB(r, g, b)
end

states = readdir(STATE_PATH)
out = zeros(RGB, WIDTH, HEIGHT)

for state in states
    if !isfile(STATE_PATH * "/" * state)
        continue
    end

    println("Processing $(state)...")

    frequency = parse(Float64, state[1:length(state) - 4]) * 100 # in THz
    color = wavelength_to_rgb(299792.458 / frequency)

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
