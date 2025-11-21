# Creating a Transform

A plugin is composed of one or more elements. The `register_plugin!` macro is the first place you need to define your element. In this example, we are making a an element called `ex_drag`, so we only need to write `register_plugin!("ex_drag");`

The other place you need to specify the name of your element is in the `transform_element` macro. This transforms a struct into something which can be picked up by `physim` as a transform element. The `name` must match what is defined in `register_plugin!` and the blurb will be used by `physcan` so user's of your plugin can find out what it is for.

With these two macros, all of the glue code needed for `physim` to find your element will be generated so you can focus on implementing the functionality of your element.
```rust.rs,ignore
{{#include ../../example_plugin/src/lib.rs:element_declaration}}
```
A simple model of drag a force, \\(\vec{F}\\), is a quadratic relationship between velocity, \\(\vec{v}\\),
\\[
\vec{F} = - \alpha v^2  \frac{\vec{v}}{\left|\vec{v}\right|}
\\]
where \\(\alpha\\) is the coefficient of drag. In terms of acceleration, drag can be expressed as
\\[
\vec{A} = \frac{- \alpha v^2 }{m} \frac{\vec{v}}{\left|\vec{v}\right|}
\\]
To implement this force, you need to implement the `TransformElement` for `Drag`.
```rust
impl TransformElement for Drag {
    fn transform(&self, state: &[Entity], accelerations: &mut [Acceleration]) {
        todo!()
    }
    
    fn new(properties: HashMap<String, Value>) -> Self {
        todo!()
    }

    fn get_property_descriptions(&self) -> HashMap<String, String> {
        todo!()
    }
}
```
A transform in `physim` has read-only access to each entity in the simulation. Transforms use this state to calculate an acceleration to apply to each entity. You can access velocity of each entity and the entities mass to calculate how to update the entity's acceleration.
```rust.rs,ignore
{{#include ../../example_plugin/src/lib.rs:element_transform}}
```
The `TransformElement` trait requires you to implement the `new` and `get_property_descriptions` methods. `new` is called when the element is being created by the pipeline, so you can parse the properties available to this element with `serde`. `get_property_descriptions` serves purely as documentation for the user of your plugin.
```rust.rs,ignore
{{#include ../../example_plugin/src/lib.rs:element_props}}
```
Finally, you should implement the `MessageClient` trait. We aren't interested in using physim's inter-element communication bus, so you can just leave it empty

Running `cargo build -r` will generate a dynamic library. Place this library in the same directory as your physim installation and you will be able to include it in your simulations, e.g.

```bash
$ physim ex_drag alpha=0.01 ! cube n=1000 seed=2 a=2.0 ! \
    simple_astro ! rk4 ! glrender ! global dt=0.01 iterations=2500
```

## The whole plugin
```rust.rs,ignore
{{#include ../../example_plugin/src/lib.rs}}
```
