using Meshes
using FileIO
using MeshIO
using GeometryBasics

STATE = "states/energy-4000.bin"
WIDTH = 1440
HEIGHT = 900

raw_state = read(STATE)
state = reshape(reinterpret(Float32, raw_state[9:end]), (WIDTH, HEIGHT))

points = GeometryBasics.Point3f[]
connec = TriangleFace{Int64}[]

function index_from_coords(x, y)
	return (y - 1) * WIDTH + x
end

for y in 1:HEIGHT
	for x in 1:WIDTH
		push!(points, GeometryBasics.Point3f((x, y, round(state[x, y] * 100))))

		if x != WIDTH && y != HEIGHT
			push!(connec, TriangleFace((index_from_coords(x, y), index_from_coords(x + 1, y), index_from_coords(x, y + 1))))
			push!(connec, TriangleFace((index_from_coords(x + 1, y), index_from_coords(x + 1, y + 1), index_from_coords(x, y + 1))))
		end
	end
end

println("Points: $(length(points))")
println("Connections: $(length(connec))")

mesh = GeometryBasics.Mesh(points, connec)
save("energy-4000.stl", mesh)
