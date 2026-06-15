def fnv1a_32(data: bytes) -> int:
    """Deterministic hash (matches the Rust implementation)."""
    hash_val = 0x811c9dc5
    for b in data:
        hash_val ^= b
        hash_val = (hash_val * 0x01000193) & 0xFFFFFFFF
    return hash_val
