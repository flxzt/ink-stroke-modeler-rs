# ink-stroke-modeler-rs

[![main docs](https://img.shields.io/badge/docs-main-informational)](https://flxzt.github.io/ink-stroke-modeler-rs/ink_stroke_modeler_rs/)
[![CI](https://github.com/flxzt/ink-stroke-modeler-rs/actions/workflows/ci.yaml/badge.svg)](https://github.com/flxzt/ink-stroke-modeler-rs/actions/workflows/ci.yaml)

WIP Rust bindings for [https://github.com/google/ink-stroke-modeler](https://github.com/google/ink-stroke-modeler), using `autocxx`

# External Dependencies
- `cmake`
- `libclang`

# Usage

Run `cargo doc --open` to view the documentation.

It is possible to choose between building the absl-cpp dependency in the crate and statically link it,
or use the system dependency. Then the `absl-cpp-dev` (or equivalent) package must be installed.

Toggled via the cargo feature `build_absl`.

### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

### Contribution

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
