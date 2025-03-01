# Physim
A program for performing many-body simulations.

## Design goals:
The aim is to make a program the functions similar to gstreamer.
- CLI interface. The interface should work like 
  - `physim plummer n=1000 ! barneshut theta=10.0 ! video`
  - `physim cube n=1000 seed=2 ! exact ! video`
  - `physim spiral ! barneshut ! stdout`
  - `physim file location="stars.csv" ! fileparse units="units.txt" ! barneshut ! stdout`  
  - It should have autocomple and help for each element. Each element should have parameters which can be passed to it.
- Plugin style architecture. This will be acheived with dynamic library loading. Plugins will provide elements that can be combined to produce different simulations. These plugins should be writable in any language, but specically in Rust, C, and Go.
- A renderer which supports camera movements.
- Can output to ffmpeg for creation of nice videos.
- Events which can affect the simulation e.g. create new entities on click, terminate the simulation gracefully.
- Be highly performant.

## Licence
MIT.