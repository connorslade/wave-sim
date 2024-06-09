using GLMakie

STATE_PATH = "states"
WIDTH = 1440
HEIGHT = 900
Z_SCALE = 30

states = readdir(STATE_PATH)

function load_state(name)
	state = read(STATE_PATH * "/" * name)
	return reshape(reinterpret(Float32, state[9:end]), (WIDTH, HEIGHT))[:, 5:end]
end

println("Found $(length(states)) states")

sort!(states, by = x -> begin
	parts = split(x, "-")

	if length(parts) == 3
		(parse(Int, parts[2]), parse(Int, parts[3][1:end-4]))
	else
		(parse(Int, parts[2][1:end-4]), 0)
	end
end)

GLMakie.activate!()
GLMakie.closeall()

data = Observable(load_state(states[1]))

fig = Figure(resolution = (1920, 1080))
axis = Axis3(fig[1, 1], aspect = (WIDTH, HEIGHT, Z_SCALE), azimuth = 6.275pi, xlabel = "x", ylabel = "y", zlabel = "z")
surface!(axis, data, colormap = :viridis)

record(fig, "3d_plot.mp4", 1:length(states)) do frame
	print("\r$(round(Int, 100 * frame / length(states)))%")
	data[] = load_state(states[frame]) * Z_SCALE
end

