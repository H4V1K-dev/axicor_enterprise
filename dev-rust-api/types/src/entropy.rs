//! Deterministic entropy generation, hashing, and stateless pseudo-random number generation.

/// Root cluster seed.
///
/// Serves as the stateless base for all deterministic pseudo-random number generation and
/// entity seed derivation throughout the cluster, ensuring bit-to-bit simulation reproducibility.
///
/// Enforces `#[repr(transparent)]` to guarantee C-ABI layout equivalence with a raw `u64`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MasterSeed(pub u64);

impl MasterSeed {
    /// Generates a master seed from a string slice using 64-bit FNV-1a hashing.
    ///
    /// This allows reproducible cluster initialization from human-readable configuration seeds
    /// without runtime allocations.
    #[inline]
    pub const fn from_str(s: &str) -> Self {
        let bytes = s.as_bytes();
        let mut hash: u64 = 0xcbf29ce484222325;
        let mut i = 0;
        while i < bytes.len() {
            hash ^= bytes[i] as u64;
            hash = hash.wrapping_mul(0x00000100000001B3);
            i += 1;
        }
        Self(hash)
    }

    /// Returns the raw `u64` value of this seed.
    #[inline]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Computes a 32-bit FNV-1a hash of the provided byte slice.
///
/// Used for mapping string identifiers (like zone names or I/O matrices) to stable,
/// reproducible indices without requiring runtime heap allocations.
#[inline]
pub const fn fnv1a_32(data: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c9dc5;
    let mut i = 0;
    while i < data.len() {
        hash ^= data[i] as u32;
        hash = hash.wrapping_mul(0x01000193);
        i += 1;
    }
    hash
}

/// Computes a fast non-collisional hash using the standard wyhash algorithm.
///
/// Directly delegates to the external `wyhash` library implementation.
#[inline]
pub fn wyhash64(data: &[u8], seed: u64) -> u64 {
    wyhash::wyhash(data, seed)
}

/// Computes a unique entity seed by mixing a master seed and an entity ID.
///
/// Employs a wyhash-style avalanche bit mixer to guarantee high entropy and uniform
/// bit distribution, ensuring that even consecutive entity IDs yield completely
/// uncorrelated seeds.
#[inline]
pub const fn entity_seed(master_seed: u64, entity_id: u32) -> u64 {
    let seed = master_seed
        .wrapping_add(entity_id as u64)
        .wrapping_add(0x60bee2bee120fc15);
    
    // Avalanche bit mixing (wyhash-style)
    let tmp = (seed as u128).wrapping_mul(0xa3b195354a39b70d);
    let m1 = ((tmp >> 64) as u64) ^ (tmp as u64);
    let tmp2 = (m1 as u128).wrapping_mul(0x1b03738712fad5c9);
    ((tmp2 >> 64) as u64) ^ (tmp2 as u64)
}

/// Generates a pseudo-random floating-point value in the range `[0.0, 1.0)`.
///
/// # IEEE 754 Mantissa Masking (E-005)
/// Rather than using division (which is slow and can lead to boundary inclusivity issues like returning `1.0`),
/// this function extracts the upper 23 bits of the seed to form the mantissa of a float.
/// The exponent is set to `1.0` (represented as `0x3F800000`). Subtracting `1.0` yields a float
/// strictly bounded within `[0.0, 1.0)`. The maximum possible return value is `0.99999994`.
///
/// # Stateless RNG Guarantee
/// This function is entirely deterministic and stateless. It relies solely on the input `seed`
/// and executes no system calls (e.g., to retrieve hardware entropy or system time).
#[inline]
pub fn random_f32(seed: u64) -> f32 {
    let bits = ((seed >> 41) as u32) | 0x3F800000;
    f32::from_bits(bits) - 1.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{size_of, align_of};

    #[test]
    fn test_entropy_memory_layout() {
        // INV-TYPES-004: size_of::<MasterSeed>() == 8 bytes
        assert_eq!(size_of::<MasterSeed>(), 8);
        assert_eq!(align_of::<MasterSeed>(), 8);
    }

    #[test]
    fn test_hash_determinism() {
        // Golden vectors check for FNV-1a hashing of the reference string "AXICOR"
        let hash_32 = fnv1a_32(b"AXICOR");
        assert_eq!(hash_32, 2178398265); // Standard FNV-1a 32-bit for "AXICOR"

        let seed = MasterSeed::from_str("AXICOR");
        assert_eq!(seed.raw(), 969268877276664313); // Standard FNV-1a 64-bit for "AXICOR"

        // Golden contract check for 08_io_matrix.md
        // "SensoryCortex" must hash to 0x273fd103
        assert_eq!(fnv1a_32(b"SensoryCortex"), 0x273fd103);
    }

    #[test]
    fn test_avalanche_effect() {
        // Verify that a single-bit change in the entity ID generates widely distributed seeds
        let s1 = entity_seed(0x123456789ABCDEF0, 42);
        let s2 = entity_seed(0x123456789ABCDEF0, 43);
        let diff = s1 ^ s2;
        // Check that at least 16 bits changed
        assert!(diff.count_ones() >= 16);
    }

    #[test]
    fn test_float_bound_exclusivity() {
        // INV-TYPES-008, E-005: Verify random_f32 bounds over a range of seeds
        // We use a simple LCG to feed seeds to random_f32
        let mut seed = 0x5a5a5a5a5a5a5a5au64;
        let mut i = 0;
        while i < 1_000_000 {
            // LCG step to generate next seed
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let val = random_f32(seed);
            assert!(val >= 0.0, "Value below 0.0: {}", val);
            assert!(val < 1.0, "Value reached or exceeded 1.0: {}", val);
            i += 1;
        }
    }

    #[test]
    fn test_legacy_seed_messy_and_empty() {
        // Messy strings and spaces
        let messy = "   AXICOR   __ 2026      \n\t_!!   $#@%   ";
        let s1 = MasterSeed::from_str(messy);
        assert_ne!(s1.raw(), 0);
        let s2 = MasterSeed::from_str(messy);
        assert_eq!(s1.raw(), s2.raw());

        // Empty string
        let empty1 = MasterSeed::from_str("");
        let empty2 = MasterSeed::from_str("");
        assert_eq!(empty1, empty2);

        // Different strings must yield different seeds
        assert_ne!(MasterSeed::from_str("A"), MasterSeed::from_str("B"));
    }
}
