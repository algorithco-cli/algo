//! Library crate for `algo-daemon` — exports pipeline/cache/jev_pool/transport for the binary,
//! benches, and integration tests. Modules have a single compiled source under `src/*.rs`.

pub mod cache;
pub mod jev_pool;
pub mod pipeline;
pub mod transport;
