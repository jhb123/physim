# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

Requires nightly Rust (pinned in `rust-toolchain.toml`).

```bash
# Build everything in release mode (produces plugin dylibs + binaries)
cargo build -r

# Build a specific crate
cargo build -p astro -r

# Run tests for all crates
cargo test --workspace

# Run tests for a specific crate
cargo test -p astro

# Run a single test by name
cargo test -p astro test_name

# Run benchmarks in the astro crate
cargo bench -p astro
cargo bench -p astro -- morton_oct   # filter to a specific bench

# Lint
cargo clippy --workspace

# Build the debug crate (not in default-members, must be explicit)
cargo build -p debug
```

The `physim` binary and all plugin `.dylib` files are placed in the same output directory (e.g. `target/release/`). The binary discovers plugins at runtime from the directory containing the executable, plus any paths in `PHYSIM_PLUGIN_DIR` (colon-separated).

## Architecture

### Plugin system

physim is built around a **plugin/element** model. The core binary loads pipeline elements at runtime from `.dylib`/`.so` files using `libloading`. Every crate except `physim-core` and `physim-attribute` compiles to a `cdylib` (plugin) as well as an `rlib`.

Discovery happens in `physim-core/src/plugin/discover.rs`: on startup the binary scans its own directory for shared libraries, calls `register_plugin()` from each, and builds an element registry keyed by element name.

### Element kinds

Each element must be one of these kinds (`ElementKind` enum in `physim-core/src/plugin/mod.rs`):

| Kind | Role |
|------|------|
| `Initialiser` | Creates the initial set of `Entity` values (e.g. `cube`, `star`) |
| `Synth` | Generates new entities each timestep |
| `Transform` | Computes accelerations given current state (e.g. gravity) |
| `Transmute` | Mutates entities in-place each timestep (e.g. collisions) |
| `Integrator` | Advances positions/velocities using accelerations (`euler`, `verlet`, `rk4`) |
| `Render` | Consumes frames from the simulation thread (`glrender`, `stdout`) |

A valid pipeline requires at least one integrator, one renderer, and at least one transform or transmute element.

### Writing a new element (Rust)

1. Create a struct and implement the appropriate trait (e.g. `TransformElement` from `physim-core::plugin::transform`).
2. Annotate it with the corresponding proc-macro from `physim-attribute` (e.g. `#[transform_element(name = "my_el", blurb = "...")]`).
3. Call `register_plugin!("my_el")` once in `lib.rs`.
4. Implement `MessageClient` for the struct (can be empty if no messaging needed).

See `example_plugin/src/lib.rs` for a minimal working example.

### Pipeline execution model

`pipeline.rs` runs two threads:
- **Simulation thread**: calls initialisers → integrators → transmutes each tick, sends state snapshots over an `mpsc::sync_channel(2)`.
- **Main thread**: drives the render element, which blocks consuming frames from the channel.

A third background thread drives a `MessageBus` that routes inter-element messages (pause, quit, configuration). Elements communicate via the bus using the `post_bus_msg!` macro and by implementing `MessageClient::recv_message`.

### Core data types

Defined in `physim-core/src/lib.rs` and `#[repr(C)]` for FFI:
- `Entity` — position (x,y,z), velocity (vx,vy,vz), radius, mass, id, fixed flag
- `Acceleration` — x,y,z components; transform elements accumulate into a `&mut [Acceleration]` slice

### Crates

| Crate | Contents |
|-------|----------|
| `physim-core` | `Entity`, `Acceleration`, pipeline engine, plugin loader, `register_plugin!` macro, `physim` + `physcan` binaries |
| `physim-attribute` | Proc-macros (`#[transform_element]`, etc.) that generate FFI glue for element structs |
| `astro` | Gravity transforms (`astro`, `astro2`, `simple_astro`), initialisers (`cube`, `star`, `plummer`, `solar`, `bar`), octree/quadtree/Morton-code internals |
| `integrators` | `euler`, `verlet`, `rk4` |
| `mechanics` | `collisions`, `impulse`, `shm` |
| `glrender` | OpenGL renderer and `stdout` raw-frame element |
| `utilities` | Misc pipeline utilities |
| `debug` | Diagnostic elements (not in default workspace members) |
| `example_plugin` | Minimal plugin template (`ex_drag` drag-force element) |

### C plugin support

Plugins can also be written in C using the header at `c_plugin/physim.h`. C plugins must export `get_plugin_abi_info` returning the string `"C"` to pass ABI validation.

### `physcan` binary

`physim-core` also builds `physcan`, which scans for available plugins and prints element metadata. Useful for debugging plugin discovery.
