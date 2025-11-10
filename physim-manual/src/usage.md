# Usage

In physim,the state of a system is represented by **entities**, and entities have various parameters like position, velocity and mass. The way the system evolves in physim simulations is determined by **elements**. Elements are configurable, reusable blocks which usually perform one kind of action on the entities, and elements are made available to physim through **plugins**. The state of the system is updated at fixed timesteps and the number of timesteps can be configured. Each kind of element is summarised below. 
| element      |                                            description                                                   |
|--------------|----------------------------------------------------------------------------------------------------------|
| initialiser  | Creates the initial state of the simulation                                                              |
| synth        | Creates new entities during the simulation                                                               |
| transform    | Calculate a force to apply to the entities. These can be chained together.                               |
| integrator   | A numerical integrator that uses the force calculated by the chain of transforms to update the entities  |
| transmute    | Directly manipulate the entities                                                                         |
| renderer     | Post processing of the state e.g. render to a window or save to file                                     |

Simulations have the following requirements:
- One integrator must be specified.
- One renderer must be specified.
- At least one transform or one transmute must be specified. You can have as many of these as you like.


## TOML configuration

The easiest way to construct your simulations is by using a toml file

```bash
$ physim -f /path/to/simulation.toml
```
The syntax of this file is as follows:
```toml
# global parameters that control the length and timestep of the simulation
[global]
dt = 0.002
iterations = 2000

# You need a map of named elements which will be in included in your simulation
[elements]
# each element can be have it's parameters configured
[[elements.foo]]
a = 1
[[elements.bar]]
b = 2
...
```
The next example will produce a cube of 1000 stars using the `cube` element. The force on each star is calculated using the Barnes-Hut algorithm with the `astro` element. A 4th order Runge-Kutta numerical integrator, `rk4`, is used to calculate the new location of each star at each time step. The simulation is rendered to a window with the `glrender` element.
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

physim can be used through the CLI. The example above can be launched with
```bash
$ physim global dt=0.01 iterations=2500 ! \
    cube n=10000 seed=2 a=2.0 !  astro theta=0.4 e=0.01 ! \
    rk4 ! glrender resolution="1080p" shader="velocity"
```

## Physcan

physcan is a utility for checking what elements you have available in physim. The output is `<plugin>: <element> <element_kind>`. Below is an example of what you will see if physcan finds plugins.
```bash
$ physcan
          astro: solar initialiser
          astro: plummer initialiser
          astro: simple_astro transform
          astro: astro2 transform
          astro: astro transform
          astro: star initialiser
          astro: cube initialiser
       glrender: stdout renderer
       glrender: glrender renderer
    integrators: euler integrator
    integrators: verlet integrator
    integrators: rk4 integrator
      mechanics: impulse transform
      mechanics: collisions transmute
      mechanics: shm transform
      utilities: bbox transmute
      utilities: idset transmute
      utilities: csvsink renderer
      utilities: wrapper transmute
      utilities: bpm transmute
```
To inspect the properties of an element, you can run `physim <element>`, for example
```bash
$ physcan simple_astro
Overview
      Name - simple_astro
     Blurb - Compute exact gravitational accelerations
      Kind - Transform

Properties
         e - Easing factor. Modify G*Ma*Mb*(r-e)^-2. Default=1.0

Meta data
   Authors - Joseph Briggs <jhbriggs23@gmail.com>
   License - MIT
   Version - 0.4.1
Repository - https://github.com/jhb123/physim

Loaded from the astro plugin located in /path/to/physim/libastro.dylib
