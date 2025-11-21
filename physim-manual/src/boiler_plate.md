# The boiler plate
A physim plugin is a dynamic library. The following Cargo project is a good place to start. To build a `physim` plugin, you will need `physim-core` for the traits and structs. `physim-attribute` provides macros that generate the glue code for physim to load your plugin. `serde_json` is needed for parsing configuration. A `build.rs` and `rustc_version` is needed exposing compiler information to the plugin.

```toml
{{#include ../../example_plugin/Cargo.toml}}
```
The build.rs file should contain
```rust
# build.rs
{{#include ../../example_plugin/build.rs}}
```
Your plugin project will be laid out like this
```
example_plugin/
├── Cargo.toml
├── build.rs
└── src/
    └── lib.rs
```
