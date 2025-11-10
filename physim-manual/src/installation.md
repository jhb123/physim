# Installation

## macOS
The only platform that physim has been pre-built for is macOS (arm64). The latest release can be found on
the [GitHub release page](https://github.com/jhb123/physim/releases). To download, install and add physim to your `PATH`, run
```bash
$ curl -L https://github.com/jhb123/physim/releases/latest/download/physim-macos.tar.gz \
  -o physim-macos.tar.gz && tar -xzf physim-macos.tar.gz && bash physim-macos/install.sh
```
## Other platforms
If you are planning to use physim on other platforms, you will need to compile the binaries and plugins yourself. This can be done by [installing Rust](https://rust-lang.org/tools/install/) and then
```bash
$ git clone https://github.com/jhb123/physim.git
$ cd physim
$ cargo build -r
```
The compiled binaries and plugins will be in `physim/target/release`.
## Adding Plugins
To add plugins, you can place them in the same directory as physim
```bash
$ ls 
libastro.dylib            # astro plugin 
libglrender.dylib         # glrender plugin 
libintegrators.dylib      # integrator plugin 
libmechanics.dylib        # classical mechanics plugin 
libphysim_attribute.dylib
libphysim_core.dylib      # core library
libutilities.dylib        # utilities plugin
physcan                   # binary for inspecting plugins
physim                    # binary for running simulations
```
You can specify additional directories that will be search for plugins, each one separated by `:`, with the `PHYSIM_PLUGIN_DIR` environment variable.
