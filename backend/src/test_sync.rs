//! Test synchronization: global mutex to serialize tests that share in-memory stores.
//! Without this, parallel cargo test threads race on VERSION_STORE / AUDIT_STORE.

use std::sync::{Mutex, MutexGuard, OnceLock};

#[allow(dead_code)]
static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[allow(dead_code)]
pub fn lock() -> MutexGuard<'static, ()> {
    // Poison-tolerant so one failing test does not cascade into all others.
    match TEST_LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}
