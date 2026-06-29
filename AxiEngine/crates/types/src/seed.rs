//! Deterministic simulation seed generator and stateless integer RNG primitives.

use bytemuck::{Pod, Zeroable};

/// Inline WyHash avalanche mixer for deterministic stateless pseudo-random generation.
#[inline(always)]
pub const fn wyhash_mix(s: u64) -> u64 {
    let t = (s as u128).wrapping_mul(0xa3b1_9535_4a39_b70d);
    let m1 = ((t >> 64) as u64) ^ (t as u64);
    let t2 = (m1 as u128).wrapping_mul(0x1b03_7387_12fa_d5c9);
    ((t2 >> 64) as u64) ^ (t2 as u64)
}

/// Root 64-bit seed of the network pseudo-random number generator.
#[repr(transparent)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, PartialEq, Eq, Hash)]
pub struct MasterSeed(pub u64);

impl MasterSeed {
    /// Generates a 64-bit pseudo-random number based on this root seed and a given salt.
    #[inline(always)]
    pub const fn random_u64(&self, salt: u64) -> u64 {
        let val = self
            .0
            .wrapping_add(salt)
            .wrapping_add(0x9e37_79b9_7f4a_7c15);
        wyhash_mix(val)
    }

    /// Generates a 32-bit pseudo-random number based on this root seed and a given salt.
    #[inline(always)]
    pub const fn random_u32(&self, salt: u64) -> u32 {
        (self.random_u64(salt) >> 32) as u32
    }
}

/// Hashes an arbitrary configuration string (ASCII/UTF-8) into a 64-bit `MasterSeed` using FNV-1a 64-bit.
#[inline]
pub const fn seed_from_str(s: &str) -> MasterSeed {
    let bytes = s.as_bytes();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut i = 0;
    while i < bytes.len() {
        hash = (hash ^ (bytes[i] as u64)).wrapping_mul(0x0000_0100_0000_01B3);
        i += 1;
    }
    MasterSeed(hash)
}

/// Computes a stateless unique entity seed for a neuron, axon, or synapse in O(1) time.
#[inline(always)]
pub const fn entity_seed(seed: MasterSeed, entity_id: u64) -> u64 {
    let val = seed
        .0
        .wrapping_add(entity_id)
        .wrapping_add(0x60be_e2be_e120_fc15);
    wyhash_mix(val)
}
