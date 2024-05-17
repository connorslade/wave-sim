# Wave Sim: Configs

`wave-sim` lets you define walls, emitters, and the c value (wave speed) at every point with an image.
The red channel is used for walls, the green channel is used for emitters, and the blue channel defines what percent of the global c value is at the position ($0 \rightarrow 0.0$, $128 \rightarrow 1.0$, $256 \rightarrow 2.0$)
See [`base-map.png`](base-map.png) for an example.

You can also use a shader function to modify the simulation while it's running, for example for a moving emitter.
The shaders are written in [WSGL](https://webgpu.rocks/wgsl/language/types), and the example shader ([`base-shader.wsgl`](base-shader.wgsl)) explains all the variables and function you can access.

## Example Configs

To run these, just supply the path to a `params.toml` as the only command line argument when starting the program.

### Double Slit

The classic [double-slit experiment](https://en.wikipedia.org/wiki/Double-slit_experiment).
This example uses a shader, even though a map would be fine, to make it resizable.

![Screenshot](https://github.com/connorslade/wave-sim/assets/50306817/f421710d-edd4-4902-9e0b-4d0fb09fe341)

### Lens

This example uses a map file to define the lense shape.
In the params file, you can change map to any of the the map files to play with the other types of lenses.

![Screenshot](https://github.com/connorslade/wave-sim/assets/50306817/f73ceaad-68ad-44a5-bf2f-69ce76462c30)
