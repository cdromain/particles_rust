## [v0.0.2] - 2025-07-14

### Changed
- Restructured project to separate simulator (`main.rs`) from core algorithm (`particles.rs`)
- Core algorithm is now a `no_std` library with zero heap allocation, suitable for embedded systems
- Replaced all `std` dependencies with `heapless` collections
- Externalized all configuration into comprehensive `Settings` struct
- Removed domain-specific pitch/scale logic; outputs now normalized to u16 range (0-65535)
- Optimized for 32-bit microcontrollers
- Maintains 100% behavioral compatibility through careful proportional mappings
- Desktop simulator still available via `cargo run --features simulator`

## [v0.0.1] - 2025-07-13

- Initial commit
- Runs on macOS with embedded-graphics-simulator and keyboard controls
