pub mod core;
pub mod paths;
pub mod profiles;

/// `Instant` multi-plateforme (std en natif, performance.now en wasm).
pub use web_time::Instant;
