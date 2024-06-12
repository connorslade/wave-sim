using Meshes
using FileIO
using MeshIO
using GeometryBasics

STATE = "states/energy-4000.bin"
WIDTH = 1440
HEIGHT = 900

VALUE_SCALE = 350
BASE_HEIGHT = 14

raw_state = read(STATE)
state = reshape(reinterpret(Float32, raw_state[9:end]), (WIDTH, HEIGHT))

points = GeometryBasics.Point3f[]
connec = TriangleFace{Int64}[]

function index_from_coords(x, y)
	return (y - 1) * WIDTH + x
end

function push_quad!(to, x1, y1, x2, y2)
	push!(to, TriangleFace((index_from_coords(x1, y1), index_from_coords(x2, y1), index_from_coords(x1, y2))))
	push!(to, TriangleFace((index_from_coords(x2, y1), index_from_coords(x2, y2), index_from_coords(x1, y2))))
end

for y in 1:HEIGHT
	for x in 1:WIDTH
		push!(points, GeometryBasics.Point3f((x, y, state[x, y] * VALUE_SCALE + BASE_HEIGHT)))

		if x != WIDTH && y != HEIGHT
			push_quad!(connec, x, y, x + 1, y + 1)
		end
	end
end

if BASE_HEIGHT != 0
	last_index = index_from_coords(WIDTH, HEIGHT)
	push!(points, GeometryBasics.Point3f((0, 0, 0)))
	push!(points, GeometryBasics.Point3f((WIDTH, 0, 0)))
	push!(points, GeometryBasics.Point3f((0, HEIGHT, 0)))
	push!(points, GeometryBasics.Point3f((WIDTH, HEIGHT, 0)))

	# Left wall
	push!(connec, TriangleFace((index_from_coords(1, 1), last_index + 1, last_index + 3)))
	push!(connec, TriangleFace((index_from_coords(1, 1), index_from_coords(1, HEIGHT), last_index + 3)))

	# Right wall
	push!(connec, TriangleFace((index_from_coords(WIDTH, 1), index_from_coords(WIDTH, HEIGHT), last_index + 2)))
	push!(connec, TriangleFace((index_from_coords(WIDTH, HEIGHT), last_index + 2, last_index + 4)))

	# Bottom wall
	push!(connec, TriangleFace((index_from_coords(1, 1), index_from_coords(WIDTH, 1), last_index + 2)))
	push!(connec, TriangleFace((index_from_coords(1, 1), last_index + 2, last_index + 1)))

	# Top wall
	push!(connec, TriangleFace((index_from_coords(1, HEIGHT), index_from_coords(WIDTH, HEIGHT), last_index + 3)))
	push!(connec, TriangleFace((index_from_coords(WIDTH, HEIGHT), last_index + 4, last_index + 3)))

	# Bottom cap
	push!(connec, TriangleFace((last_index + 1, last_index + 3, last_index + 4)))
	push!(connec, TriangleFace((last_index + 1, last_index + 4, last_index + 2)))
end

println("Points: $(length(points))")
println("Connections: $(length(connec))")

mesh = GeometryBasics.Mesh(points, connec)
save("energy.stl", mesh);
