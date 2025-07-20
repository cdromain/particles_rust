# particles - Rust port

A Rust port of my [*particles*](https://github.com/cdromain/particles_nt) generative algorithm using `embedded-graphics-simulator`.

Based on the v0.2 of the original Lua script for the disting NT.

## Notes
- The simulator (`main.rs`) is separated from the core algorithm (`particles.rs`)
- Outputs normalized u16 values instead of pitch/scale for embedded system compatibility

## Quick Start (simulator)

1. Copy `particles.rs`, `main.rs` and `Cargo.toml` to the directory

2. Run the compilation & simulation :
   ```bash
   cargo run --release
   ```

### Controls

- `Space` : Toggle verbose mode
- `G` : Adjust gravity
-  `W` : Adjust wind
- `P` : Adjust max particles
- `Q` : Quit