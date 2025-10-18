```
██████╗ ██╗  ██╗██╗   ██╗███████╗██╗███╗   ███╗
██╔══██╗██║  ██║╚██╗ ██╔╝██╔════╝██║████╗ ████║
██████╔╝███████║ ╚████╔╝ ███████╗██║██╔████╔██║
██╔═══╝ ██╔══██║  ╚██╔╝  ╚════██║██║██║╚██╔╝██║
██║     ██║  ██║   ██║   ███████║██║██║ ╚═╝ ██║
╚═╝     ╚═╝  ╚═╝   ╚═╝   ╚══════╝╚═╝╚═╝     ╚═╝
```
An extensible framework for performing N-body simulations.

# Overview
Physim provides a framework for users to run N-body simulations. Users can build pipelines that define the properties of a simulation and have access to a variety of useful elements such as OpenGL renderers and gravity calculations.

The functionality of physim can be expanded with plugins to add functionality. Developers can make these plugins with `Rust` and there is support for some elements to be written in languages with a `C` ABI.

## Roadmap
There are many features that are under development or planned. The library should be considered unstable and backward compatibility is not guaranteed.

The current state of the framework is `pre-0.1.0`.

### 0.1.0
This is the proof-of-concept release. It'll demonstrate core concepts. There will be no patch versions and bugs discovered in this release will be fixed in `0.2.0`.
- `physim`: CLI program for running simulations.
- `physcan`: CLI program for getting documentation about elements
- Plugin loading via dynamic library loading.
- Transform API with a C ABI.
- Render API.
- Initialiser API.
- `glrender` plugin. Provides the `glrender` and `stdout` elements.
- `astro` plugin. Provides assorted elements for galaxy simulations.
- Rust macros for creating plugins.
### 0.2.0
This release will have more advanced elements as well as extend the functionality of the current elements. There will be no patch versions and bugs discovered in this release will be fixed in `0.3.0`.
- Inter-element communication bus. Include events such as "end-of-simulation" to enable clean finishes and dynamic updating of the properties of elements.
- Global simulation parameters like the time step, number of steps and "universe bounds" will be configurable.
- Transmute API. This will support events like entity mergers.
- Synthesis API. This will support events such as entity creation from external inputs.
- `glrender` elements will have more properties exposed to the user.
- Support multiple transform elements in one pipeline.
- Add more elements to the `astro` plugin. The `plummer` element will be added in this release.
### 0.3.0
This release will focus on performance, testability, bug fixing and stability along with expanding the functionality provided by the `astro` plugin. Things included in this release will be.
- demonstration plugins written in `C` and `Go`.
- The physical unit system will be properly established in the `astro` element.
- `glrender` with more advanced user inputs.

### Beyond.
No further features are planned yet. A gas simulation plugin would be interesting as would some electromagnetism simulations. Adding a simulation with relativistic effects might be possible with this framework. These features could result in `Entity` changing, so it is worth reiterating that this library is not in a stable form and backwards compatibility is not guaranteed.

# Running a simulation
`physim` simulations can be configured directly in the CLI. Each element is delimited by `!`, and the properties of the element can be configured as shown in the following example:

```bash
physim cube n=100000 seed=1 spin=1000 ! star mass=100000.0 radius=0.1 z=0.5 x=0.2 y=0.2 ! star mass=100000.0 radius=0.1 z=0.5 x=-0.2 y=-0.2 ! astro2 theta=1.5 e=0.5 ! verlet ! glrender ! global dt=0.00001 iterations=10000
```
Alternatively, the can be loaded via a file. The pipeline above can be expressed in toml as 

```toml
[global]
dt = 0.00001
iterations = 1000

[elements]

[[elements.cube]]
n = 100000
seed = 1
spin = 1000

[[elements.astro2]]
theta = 1.5
e = 0.5

[[elements.star]]
mass = 100000.0
x=0.2
y=0.2
z=0.5
radius = 0.1

[[elements.star]]
mass = 100000.0
x=-0.2
y=-0.2
z=0.5
radius = 0.1

[[elements.verlet]]

[[elements.glrender]]
```
and run with
```
physim -f pipeline.toml
```
## Encoding with FFMPEG
The following example shows how to use the `stdout` element from the `glrender` plugin.
```bash
cargo run -r --bin physim cube n=100000 seed=1 spin=500 ! star mass=100000.0 x=0.1 y=0.1 radius=0.1 z=0.5 ! star mass=100000.0 x=-0.1 y=-0.1 radius=0.1 z=0.5 ! star mass=100000.0 x=-0.1 y=0.1 z=0.5 ! star mass=100000 x=0.1 y=-0.1 z=0.5 ! astro theta=1.3 ! verlet ! stdout zoom=1.5 resolution=1080p | ffmpeg -y -f rawvideo -pixel_format bgra -video_size 1920x1080 -framerate 60 -i pipe:0 -vf "scale=in_range=full:out_range=full,format=yuv420p10le" -c:v libx265 -preset slow -pix_fmt yuv420p10le output.mp4
```
Add audio with `ffmpeg -i input.mp4 -i input.mp3 -c:v copy -c:a aac -shortest output.mp4`


# Element documentation
Specify the location of your plugin directory with the `PHYSIM_PLUGIN_DIR` environment variable. To determine what elements you have access to, run
```
cargo run -r --bin physcan
```
and to see more details about an element:
```
cargo run -r --bin physcan cube
```

# Development

## cbindgen

Run `cbindgen --lang c --crate physim-core --output physim.h` to generate a header file.

## Git

Commits should follow the [conventional commits
standard](https://www.conventionalcommits.org/en/v1.0.0/#summary).

The `.gitmessage` file provides guidance on this and it can be set
as your template with 
```bash
$ git config commit.template .gitmessage
```

We use the rebase strategy for pull requests.

Use `pre-commit` to keep the codebase  free of common style issues. 


TODO: create a plugin tutorial.

# Licence
MIT.
