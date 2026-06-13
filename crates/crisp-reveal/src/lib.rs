mod ownership;
mod lifetimes;

pub use lifetimes::reveal_lifetimes;
pub use ownership::reveal_ownership;
pub use types::reveal_types;

mod types;
