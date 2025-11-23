# Usage

In `physim`,the state of a system is represented by **entities**, and entities have various parameters like position, velocity, and mass. The way the system evolves in `physim` simulations is determined by **elements**. Elements are configurable, reusable blocks which usually perform one kind of action on the entities, and elements are made available to `physim` through **plugins**. The state of the system is updated at fixed timesteps and the number of timesteps can be configured. Each kind of element is summarised below. 
| element kind |                                            description                                                   |
|--------------|----------------------------------------------------------------------------------------------------------|
| initialiser  | Creates the initial state of the simulation                                                              |
| synth        | Creates new entities during the simulation                                                               |
| transform    | Calculate a force to apply to the entities. These can be chained together.                               |
| integrator   | A numerical integrator that uses the force calculated by the chain of transforms to update the entities  |
| transmute    | Directly manipulate the entities                                                                         |
| renderer     | Post processing of the state e.g. render to a window or save to file                                     |

Simulations have the following requirements:
1. One integrator must be specified.
2. One renderer must be specified.
3. At least one transform or one transmute must be specified.
   
A pipeline can use a mixture of transforms and transmutes. For example, `astro` is a transform which calculates the gravitational force acting on entities. `collision` is a transmute which calculates elastic collisions. `astro` indirectly changes each entity through the integrator selected for the simulation whereas `collision` directly modifies the velocities of the entities.
## TOML configuration

The easiest way to construct your simulations is with a TOML file.
```bash
$ physim -f /path/to/simulation.toml
```
The syntax of this file is as follows:
```toml
# simulation.toml
# global parameters that control the length and timestep of the simulation
[global]
dt = 0.002
iterations = 2000

# You need a map of named elements which will be in included in your simulation
[elements]
# each element's parameters can be configured
[[elements.foo]]
a = 1
[[elements.bar]]
b = 2
...
```
The next example will produce a cube of 10,000 stars using the `cube` element. The force on each star is calculated using the Barnes-Hut algorithm with the `astro` element. A 4th order Runge-Kutta numerical integrator, `rk4`, is used to calculate the new location of each star at each time step. The simulation is rendered to a window with the `glrender` element.
```toml
[global]
dt = 0.01
iterations = 2500

[elements]

[[elements.cube]]
n = 10000
seed = 2
a = 2.0 # side length of the cube in screen-coordinates

[[elements.astro]]
theta = 0.4
e = 0.01

[[elements.rk4]]


[[elements.glrender]]
resolution="1080p"
shader="velocity"
```

## From the CLI
`physim` simulations can be configured directly in the CLI. Each element is delimited by `!`, and the properties of the element can be configured as shown below. The same simulation above can be launched with
```bash
$ physim global dt=0.01 iterations=2500 ! \
    cube n=10000 seed=2 a=2.0 !  astro theta=0.4 e=0.01 ! \
    rk4 ! glrender resolution="1080p" shader="velocity"
```
## Physcan
`physcan` is for checking what elements you have available in `physim`. To inspect an element's documentation, you can run `physim <element>`, e.g. `physcan astro`.
