//! Tiny local Rust crate depended on from `examples/path_dep` via `crisp.toml` path (#105).

pub fn answer() -> i64 {
    42
}

/// Discrete Laplacian stencil (`(u[i-1] + u[i+1] - 2 u[i]) / dx^2`).
pub fn lap3(um: f64, uc: f64, up: f64, dx: f64) -> f64 {
    (um + up - 2.0 * uc) / (dx * dx)
}
