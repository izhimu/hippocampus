/// Shared utility functions

/// Generate a pseudo-unique hex ID from nanosecond timestamp
pub fn uuid_hex() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:016x}{:016x}", nanos, nanos.wrapping_mul(0x9e3779b97f4a7c15))
}

/// Fast xorshift64 PRNG seeded from system time
pub fn fast_random() -> u64 {
    use std::cell::Cell;
    use std::time::{SystemTime, UNIX_EPOCH};

    thread_local! {
        static STATE: Cell<u64> = Cell::new(0);
    }

    STATE.with(|s| {
        let mut x = s.get();
        if x == 0 {
            let ns = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64;
            x = ns | 1; // ensure nonzero
        }
        // xorshift64
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        s.set(x);
        x
    })
}

/// Return a random f64 in [0, 1)
pub fn fast_random_f64() -> f64 {
    (fast_random() >> 11) as f64 / (1u64 << 53) as f64
}

/// Return a random usize
pub fn fast_random_usize() -> usize {
    fast_random() as usize
}
