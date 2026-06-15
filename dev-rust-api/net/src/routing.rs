use std::sync::atomic::{AtomicPtr, Ordering};
use std::collections::HashMap;
use std::net::SocketAddr;

/// Routing table for lock-free O(1) route lookup in the Data Plane.
///
/// # Lock-Free Routing Invariant (INV-NET-001)
/// Under INV-NET-001, the HFT cycle (Data Plane) must not block or acquire OS locks
/// when routing spikes. Lookups are performed lock-free in O(1) using an AtomicPtr
/// load with Acquire memory ordering. Updating routes (e.g. migration, node resurrection)
/// is handled in the background by copying the table, updating the copy, and atomically
/// swapping the pointer with Release ordering. Memory reclamation of the old table is
/// deferred using Epoch-Based Reclamation (EBR) via `crossbeam::epoch` to prevent use-after-free
/// while concurrent readers access the old pointer.
pub struct RoutingTable {
    /// Atomic pointer to the immutable HashMap containing the target routes.
    pub routes: AtomicPtr<HashMap<u32, SocketAddr>>,
}

impl RoutingTable {
    /// Create a new empty RoutingTable.
    pub fn new() -> Self {
        let map = Box::new(HashMap::new());
        Self {
            routes: AtomicPtr::new(Box::into_raw(map)),
        }
    }

    /// Read address for a zone hash in O(1) lock-free.
    pub fn get_address(&self, zone_hash: u32) -> Option<SocketAddr> {
        let _guard = crossbeam::epoch::pin();
        let ptr = self.routes.load(Ordering::Acquire);
        if ptr.is_null() {
            return None;
        }
        let map = unsafe { &*ptr };
        map.get(&zone_hash).copied()
    }

    /// Update routes using the RCU pattern.
    pub fn update_routes(&self, new_routes: HashMap<u32, SocketAddr>) {
        let new_ptr = Box::into_raw(Box::new(new_routes));
        let old_ptr = self.routes.swap(new_ptr, Ordering::Release);

        if !old_ptr.is_null() {
            let old_usize = old_ptr as usize;
            unsafe {
                crossbeam::epoch::pin().defer(move || {
                    let raw_ptr = old_usize as *mut HashMap<u32, SocketAddr>;
                    let _ = Box::from_raw(raw_ptr);
                });
            }
        }
    }
}

impl Default for RoutingTable {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for RoutingTable {
    fn drop(&mut self) {
        let ptr = self.routes.swap(std::ptr::null_mut(), Ordering::Relaxed);
        if !ptr.is_null() {
            unsafe {
                let _ = Box::from_raw(ptr);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rcu_routing_o1_read() {
        let routing_table = RoutingTable::new();

        assert_eq!(routing_table.get_address(1), None);

        let mut routes = HashMap::new();
        routes.insert(1, "127.0.0.1:8080".parse().unwrap());
        routes.insert(2, "127.0.0.1:8081".parse().unwrap());
        routing_table.update_routes(routes);

        assert_eq!(routing_table.get_address(1), Some("127.0.0.1:8080".parse().unwrap()));
        assert_eq!(routing_table.get_address(2), Some("127.0.0.1:8081".parse().unwrap()));
        assert_eq!(routing_table.get_address(3), None);

        let mut new_routes = HashMap::new();
        new_routes.insert(1, "127.0.0.1:9090".parse().unwrap());
        routing_table.update_routes(new_routes);

        assert_eq!(routing_table.get_address(1), Some("127.0.0.1:9090".parse().unwrap()));
        assert_eq!(routing_table.get_address(2), None);
    }
}
