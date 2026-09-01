use std::sync::atomic::{AtomicUsize, Ordering};

use super::LoadError;

const MAX_CONCURRENT_FETCHES: usize = 16;
const MAX_CONCURRENT_DNS_LOOKUPS: usize = 4;

struct PermitPool {
    active: AtomicUsize,
    capacity: usize,
}

impl PermitPool {
    const fn new(capacity: usize) -> Self {
        Self {
            active: AtomicUsize::new(0),
            capacity,
        }
    }

    fn try_acquire(&'static self, phase: &'static str) -> Result<Permit, LoadError> {
        self.active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
                (active < self.capacity).then_some(active + 1)
            })
            .map_err(|_| LoadError::Request(format!("{phase} limit reached")))?;
        Ok(Permit { pool: self })
    }
}

pub(super) struct Permit {
    pool: &'static PermitPool,
}

impl Drop for Permit {
    fn drop(&mut self) {
        self.pool.active.fetch_sub(1, Ordering::AcqRel);
    }
}

static FETCH_PERMITS: PermitPool = PermitPool::new(MAX_CONCURRENT_FETCHES);
static DNS_PERMITS: PermitPool = PermitPool::new(MAX_CONCURRENT_DNS_LOOKUPS);

pub(super) fn acquire_fetch_permit() -> Result<Permit, LoadError> {
    FETCH_PERMITS.try_acquire("network capacity")
}

pub(super) fn acquire_dns_permit() -> Result<Permit, LoadError> {
    DNS_PERMITS.try_acquire("DNS capacity")
}

#[cfg(test)]
mod tests {
    use super::PermitPool;
    use crate::loading::LoadError;

    #[test]
    fn permit_pool_bounds_concurrent_work() {
        let pool = Box::leak(Box::new(PermitPool::new(1)));
        let permit = pool.try_acquire("test capacity").unwrap();

        let blocked = pool.try_acquire("test capacity");
        assert!(matches!(blocked, Err(LoadError::Request(reason)) if reason.contains("limit")));

        drop(permit);
        assert!(pool.try_acquire("test capacity").is_ok());
    }
}
