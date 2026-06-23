#!/usr/bin/env python3
"""
CartPole ↔ Genesis Neural Node Client
Population Coding: 4 float → 64 virtual axons (Gaussian Receptive Fields)
Motor Readout:     popcount(motor_left) vs popcount(motor_right) → WTA
"""
import socket
import struct
import gymnasium as gym
import numpy as np

# ─── Memory Contracts (Spec 08) ──────────────────────────────────
GSIO_MAGIC  = 0x4F495347  # "GSIO"
GSOO_MAGIC  = 0x4F4F5347  # "GSOO"
# magic(4) + zone_hash(4) + matrix_hash(4) + payload_size(4) + global_reward(2) + _pad(2) = 20 bytes
HEADER_FMT  = "<IIIIhH"
HEADER_SIZE = struct.calcsize(HEADER_FMT)  # 20

GENESIS_IP  = "127.0.0.1"
PORT_OUT    = 8081   # Node receives input here
PORT_IN     = 8092   # Node sends output here
BATCH_TICKS = 10     # Must match sync_batch_ticks in model.toml
NUM_NEURONS = 64     # neurons per variable


def fnv1a_32(data: bytes) -> int:
    h = 0x811c9dc5
    for b in data:
        h ^= b
        h = (h * 0x01000193) & 0xFFFFFFFF
    return h


ZONE_HASH = fnv1a_32(b"CartPoleCortex")


# ─── Population Coding ───────────────────────────────────────────
def encode_population(value: float, min_val: float, max_val: float,
                      n: int = NUM_NEURONS) -> int:
    """
    Float → N-bit bitmask via Gaussian Receptive Field (σ≈1 slot).
    Zero-allocation: pure integer arithmetic, no heap.
    """
    norm = max(0.0, min(1.0, (value - min_val) / (max_val - min_val)))
    center = int(norm * (n - 1))
    mask = 0
    for i in range(max(0, center - 1), min(n, center + 2)):
        mask |= (1 << i)
    return mask


def build_input_batch(cart_x: float, cart_v: float,
                      pole_a: float, pole_av: float) -> bytes:
    """
    Encode 4 variables → 64 bit each → 256 bits (32 bytes) total per tick.
    """
    x_bits = encode_population(cart_x, -2.4, 2.4, 64)
    v_bits = encode_population(cart_v, -3.0, 3.0, 64)
    a_bits = encode_population(pole_a, -0.209, 0.209, 64)
    av_bits = encode_population(pole_av, -3.0, 3.0, 64)

    packed_tick = struct.pack("<QQQQ", x_bits, v_bits, a_bits, av_bits)
    return packed_tick * BATCH_TICKS


def main():
    env = gym.make("CartPole-v1", render_mode="human")
    obs, _ = env.reset()

    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.bind(("127.0.0.1", PORT_IN))
    sock.settimeout(0.5)

    total_reward = 0.0
    episode = 1
    left_power = 0
    right_power = 0
    action = 0

    print("Starting CartPole client. Waiting for Genesis node to run...")

    while True:
        obs, reward, terminated, truncated, _ = env.step(action)
        total_reward += reward

        dopamine = int(reward * 10)
        if terminated or truncated:
            dopamine = -100

        # Send sensory payload
        cart_x, cart_v, pole_a, pole_av = obs
        payload = build_input_batch(cart_x, cart_v, pole_a, pole_av)

        header = struct.pack(HEADER_FMT, GSIO_MAGIC, ZONE_HASH, 0, len(payload), dopamine, 0)
        sock.sendto(header + payload, (GENESIS_IP, PORT_OUT))

        # Receive motor readout
        try:
            data, _ = sock.recvfrom(65535)
            if len(data) >= HEADER_SIZE:
                magic, z_hash, _, p_size, _, _ = struct.unpack(
                    HEADER_FMT, data[:HEADER_SIZE])

                if magic == GSOO_MAGIC and z_hash == ZONE_HASH:
                    out_payload = data[HEADER_SIZE : HEADER_SIZE + p_size]
                    print(f"DEBUG: Received GSOO magic={hex(magic)} zone={hex(z_hash)} p_size={p_size} payload_len={len(out_payload)}")
                    if len(out_payload) == BATCH_TICKS * 64:
                        spikes = np.frombuffer(out_payload, dtype=np.uint8).reshape((BATCH_TICKS, 64))
                        total = np.sum(spikes, axis=0)
                        left_power = int(np.sum(total[0:32]))
                        right_power = int(np.sum(total[32:64]))
                        print(f"DEBUG: Spikes sum={np.sum(spikes)}, L={left_power}, R={right_power}")
                        action = 0 if left_power >= right_power else 1
                    else:
                        print(f"DEBUG: Size mismatch, expected {BATCH_TICKS * 64}, got {len(out_payload)}")

        except socket.timeout:
            pass

        if terminated or truncated:
            print(f"Episode {episode:4d} | Score: {int(total_reward):5d} | "
                  f"L={left_power:4d} R={right_power:4d} | "
                  f"dopamine={dopamine:+4d}")
            obs, _ = env.reset()
            episode += 1
            total_reward = 0.0
            left_power = 0
            right_power = 0
            action = 0


if __name__ == "__main__":
    main()
