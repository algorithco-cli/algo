//! Library crate for `algo-daemon` — re-exports pipeline/cache/jev_pool/transport for benches and integration tests.
//! Binary `algo-daemon` (src/main.rs) uses `mod cache` directly; this lib mirrors the same modules
//! so `cargo bench --bench pipeline_L0L1` can `use algo_daemon::pipeline::Pipeline`.
//! No logic is duplicated; modules are the single source under `src/*.rs`.

pub mod cache;
pub mod jev_pool;
pub mod pipeline;
pub mod transport;
