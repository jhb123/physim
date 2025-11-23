# Creating a Transform

A plugin contains one or more elements. `register_plugin!` must name all the elements in a plugin. In this example we are creating a single element, `ex_drag`, so we write `register_plugin!("ex_drag");`. If the plugin had more elements, you would list them all in the macro. Besides declaring the elements, `register_plugin!` sets up the message bus between them and lets the plugin use the same logger as `physim`.

Each element is defined by adding a macro to a struct. Because we are making a transform element, we need the `transform_element` macro. This macro also takes a short description which is used by `physcan` to show users what the element does. With `register_plugin!` and the `transform_element` macro, `physim` can load your element.
```rust.rs,ignore
{{#include ../../example_plugin/src/lib.rs:element_declaration}}
```
A simple model of acceleration due to drag is to scale it with the square of the entity's velocity. The acceleration, \\(\vec{A}\\), can be expressed as
\\[
\vec{A} = \frac{- \alpha v^2 }{m} \frac{\vec{v}}{\left|\vec{v}\right|}
\\]
where \\(\vec{v}\\) is velocity, \\(\alpha\\) is the coefficient of drag and \\(m\\) is the mass of the entity.

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
On each step of the simulation, the state of the system will be passed to this element. A transform in `physim` has read-only access to each entity in the simulation. Transforms use this state to calculate an acceleration to apply to each entity. You can use the velocity and mass of each entity to calculate how to update the entity's acceleration.
```rust.rs,ignore
{{#include ../../example_plugin/src/lib.rs:element_transform}}
```
`new` is called when an instance of the element is being created by `physim`. The element's configuration comes as a hash map, and this can be parsed with `serde`. `get_property_descriptions` serves purely as documentation for your plugin.
```rust.rs,ignore
{{#include ../../example_plugin/src/lib.rs:element_props}}
```
Finally, you should implement the `MessageClient` trait. We aren't interested in using `physim`'s inter-element communication bus, so you can leave it empty.

Running `cargo build -r` will generate a dynamic library. Place this library in the same directory as your `physim` installation and you will be able to include it in your simulations, e.g.

```bash
$ physim ex_drag alpha=0.01 ! cube n=1000 seed=2 a=2.0 ! \
    simple_astro ! rk4 ! glrender ! global dt=0.01 iterations=2500
```

## The whole plugin
```rust.rs,ignore
{{#include ../../example_plugin/src/lib.rs}}
```
