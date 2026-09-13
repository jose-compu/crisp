//! Tiny local Rust crate depended on from `examples/path_dep` via `crisp.toml` path (#105).

pub fn answer() -> i64 {
    42
}

/// Discrete Laplacian stencil (`(u[i-1] + u[i+1] - 2 u[i]) / dx^2`).
pub fn lap3(um: f64, uc: f64, up: f64, dx: f64) -> f64 {
    (um + up - 2.0 * uc) / (dx * dx)
}

/// Sum a borrowed float field (#153).
pub fn sum_f64(xs: &[f64]) -> f64 {
    xs.iter().sum()
}

/// Owned `vec<float>` return (#153).
pub fn ones(n: i64) -> Vec<f64> {
    vec![1.0; n.max(0) as usize]
}

/// Heatmap-style call: pass `u.data` without joining CSV (#153).
pub fn plot_heatmap(path: &str, nx: i64, ny: i64, data: &[f64], cmap: &str) -> String {
    format!("{path}:{nx}x{ny}:n={}:cmap={cmap}", data.len())
}
