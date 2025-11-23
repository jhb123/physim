# The boiler plate
This Cargo project contains the dependencies needed to build a plugin for `physim`. `physim-core` provides traits and types. `physim-attribute` provides macros that generate the code which lets `physim` use the plugin. `serde_json` is used to parse an element's configuration at run time. The plugin needs a `build.rs` script and the `rustc_version` crate to expose compiler information which `physim` checks for compatibility. Because the plugin is a dynamically loaded library, you should specify `crate-type = ["dylib"]`.
```toml
{{#include ../../example_plugin/Cargo.toml}}
```
The `build.rs` script should contain
```rust
# build.rs
{{#include ../../example_plugin/build.rs}}
```
Your plugin project can be laid out like this
```
example_plugin/
├── Cargo.toml
├── build.rs
└── src/
    └── lib.rs
```
