//! Deterministic protocol string and identifier hashing helpers.

/// Deterministic 32-bit FNV-1a hash for protocol zone names, I/O matrices, and packet identification.
#[inline]
pub const fn hash_name_fnv1a(name: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    let mut i = 0;
    while i < name.len() {
        hash = (hash ^ (name[i] as u32)).wrapping_mul(0x0100_0193);
        i += 1;
    }
    hash
}

/// Alias for `hash_name_fnv1a` for 32-bit FNV-1a hashing.
#[inline]
pub const fn fnv1a_32(bytes: &[u8]) -> u32 {
    hash_name_fnv1a(bytes)
}
