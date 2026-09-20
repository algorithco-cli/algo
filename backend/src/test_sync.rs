//! Test synchronization: global mutex to serialize tests that share in-memory stores.
//! Without this, parallel cargo test threads race on VERSION_STORE / AUDIT_STORE.

use std::sync::{Mutex, MutexGuard, OnceLock};

static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub fn lock() -> MutexGuard<'static, ()> {
    TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("test lock poisoned")
}
