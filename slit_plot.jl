using Plots
using Plots.PlotMeasures
using FileIO
using ImageIO


data = reinterpret(Float32, read("out.f32"))
max_value = sqrt(maximum(data))

image_width = 100
image_color = RGB(0.4549019607843137, 0.1607843137254902, 0.4588235294117647)

image = zeros(RGB{Float64}, image_width, length(data))

for i in 1:length(data)
	value = sqrt(data[i]) / max_value

	color = image_color * value
	for j in 1:image_width
		image[j, i] = color
	end
end

save("slit_plot.png", image)

plot(sqrt.(data), legend = false, axis = false, grid = false, color = :red, margin = 0px, linewidth = 2)
