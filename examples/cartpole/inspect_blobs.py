#!/usr/bin/env python3
import sys
import struct
import os
import collections

AXON_SENTINEL = 0x80000000

def align_offset(offset):
    return (offset + 63) & ~63

def compute_state_offsets(padded_n):
    # Port of Rust's compute_state_offsets
    current = 0
    
    soma_voltage = current
    current = align_offset(current + padded_n * 4) # i32
    
    flags = current
    current = align_offset(current + padded_n * 1) # u8
    
    threshold_offset = current
    current = align_offset(current + padded_n * 4) # i32
    
    timers = current
    current = align_offset(current + padded_n * 1) # u8
    
    soma_to_axon = current
    current = align_offset(current + padded_n * 4) # u32
    
    dendrite_targets = current
    current = align_offset(current + padded_n * 128 * 4) # u32
    
    dendrite_weights = current
    current = align_offset(current + padded_n * 128 * 4) # i32
    
    dendrite_timers = current
    current = align_offset(current + padded_n * 128 * 1) # u8
    
    return {
        "soma_voltage": soma_voltage,
        "flags": flags,
        "threshold_offset": threshold_offset,
        "timers": timers,
        "soma_to_axon": soma_to_axon,
        "dendrite_targets": dendrite_targets,
        "dendrite_weights": dendrite_weights,
        "dendrite_timers": dendrite_timers,
        "total_size": current
    }

def inspect_state(state_data):
    if len(state_data) < 16:
        print("Error: State file is truncated.")
        return
        
    s_magic, s_version, s_padded_n, s_total_axons = struct.unpack("<4sIII", state_data[0:16])
    if s_magic != b"GSNS":
        print(f"Error: Invalid state magic {s_magic}")
        return
        
    print(f"\n================================================================================")
    print(f"                       State Blob Inspection (GSNS)")
    print(f"================================================================================")
    print(f"Header: Version={s_version}, Padded N={s_padded_n}, Total Axons={s_total_axons}")
    
    pn = s_padded_n
    offsets = compute_state_offsets(pn)
    data_base = 64
    
    # Read arrays
    def read_i32_array(offset, count):
        start = data_base + offset
        end = start + count * 4
        return list(struct.unpack(f"<{count}i", state_data[start:end]))
        
    def read_u32_array(offset, count):
        start = data_base + offset
        end = start + count * 4
        return list(struct.unpack(f"<{count}I", state_data[start:end]))
        
    def read_u8_array(offset, count):
        start = data_base + offset
        end = start + count
        return list(state_data[start:end])
        
    voltages = read_i32_array(offsets["soma_voltage"], pn)
    flags_raw = read_u8_array(offsets["flags"], pn)
    thresholds = read_i32_array(offsets["threshold_offset"], pn)
    timers = read_u8_array(offsets["timers"], pn)
    soma_to_axon = read_u32_array(offsets["soma_to_axon"], pn)
    
    d_targets = read_u32_array(offsets["dendrite_targets"], pn * 128)
    d_weights = read_i32_array(offsets["dendrite_weights"], pn * 128)
    d_timers = read_u8_array(offsets["dendrite_timers"], pn * 128)
    
    # Analyze Neurons
    print("\n--- NEURONS (SOMA) STATISTICS ---")
    active_voltages = [v for v in voltages if v != -70000] # resting potential is -70000
    print(f"Voltage (resting potential is -70000):")
    print(f"  Min: {min(voltages)} | Max: {max(voltages)} | Mean: {sum(voltages)/len(voltages):.1f}")
    print(f"  Neurons out of rest: {len(active_voltages)} / {pn} ({len(active_voltages)/pn*100:.1f}%)")
    
    # Flags analysis
    spiking_count = sum(1 for f in flags_raw if (f & 0x01) != 0)
    variant_distribution = collections.Counter(f >> 4 for f in flags_raw)
    burst_distribution = collections.Counter((f >> 1) & 0x07 for f in flags_raw)
    
    print(f"Flags distributions:")
    print(f"  Spiking neurons:  {spiking_count} / {pn}")
    print(f"  Variant IDs:      " + ", ".join(f"Type {k}: {v}" for k, v in sorted(variant_distribution.items())))
    print(f"  Burst counts:     " + ", ".join(f"Burst {k}: {v}" for k, v in sorted(burst_distribution.items())))
    
    # Threshold offsets
    active_thresholds = [t for t in thresholds if t != 0]
    print(f"Threshold Offsets:")
    print(f"  Min: {min(thresholds)} | Max: {max(thresholds)} | Mean: {sum(thresholds)/len(thresholds):.1f}")
    print(f"  Modified threshold neurons: {len(active_thresholds)} / {pn}")
    
    # Soma Timers
    active_timers = [t for t in timers if t > 0]
    print(f"Soma Timers:")
    print(f"  Active timers (refractory/homeostasis): {len(active_timers)} / {pn}")
    if active_timers:
        print(f"    Max timer value: {max(active_timers)}")
        
    # Soma to Axon connections
    has_axon = sum(1 for a in soma_to_axon if a != AXON_SENTINEL)
    print(f"Soma-to-Axon Mapping:")
    print(f"  Neurons mapped to axons: {has_axon} / {pn}")
    
    # Dendrites statistics
    print("\n--- DENDRITES (SYNAPSES) STATISTICS ---")
    total_slots = pn * 128
    connected_slots = 0
    connected_weights = []
    connected_timers = 0
    
    # PackTarget: axon_id = (t & 0x00FFFFFF) - 1, segment = t >> 24.
    # If t == 0, empty.
    for i in range(total_slots):
        t = d_targets[i]
        if t != 0:
            connected_slots += 1
            connected_weights.append(d_weights[i])
            if d_timers[i] > 0:
                connected_timers += 1
                
    print(f"Dendrite Slot Usage:")
    print(f"  Active connections: {connected_slots} / {total_slots} ({connected_slots/total_slots*100:.2f}%)")
    print(f"  Average synapses per neuron: {connected_slots / pn:.1f} (out of 128 max)")
    
    if connected_slots > 0:
        print(f"Weights of active synapses:")
        print(f"  Min: {min(connected_weights)} | Max: {max(connected_weights)} | Mean: {sum(connected_weights)/len(connected_weights):.2f}")
        print(f"  Active dendrite timers: {connected_timers}")

def inspect_axons(axons_data):
    if len(axons_data) < 16:
        print("Error: Axons file is truncated.")
        return
        
    a_magic, a_version, a_total_axons, _pad = struct.unpack("<4sIII", axons_data[0:16])
    if a_magic != b"GSAX":
        print(f"Error: Invalid axons magic {a_magic}")
        return
        
    print(f"\n================================================================================")
    print(f"                       Axons Blob Inspection (GSAX)")
    print(f"================================================================================")
    print(f"Header: Version={a_version}, Total Axons={a_total_axons}")
    
    ta = a_total_axons
    # BurstHeads8 occupies bytes 32..end, each 32 bytes
    active_axons = 0
    signal_counts = []
    
    for i in range(ta):
        start = 32 + i * 32
        end = start + 32
        heads = struct.unpack("<8I", axons_data[start:end])
        
        # Count non-sentinel values (active signal wavefronts)
        active_heads = [h for h in heads if h != AXON_SENTINEL]
        if len(active_heads) > 0:
            active_axons += 1
            signal_counts.append(len(active_heads))
            
    print(f"\nAxon Burst Heads:")
    print(f"  Active axons (with signal wavefronts): {active_axons} / {ta} ({active_axons/ta*100:.2f}%)")
    if active_axons > 0:
        print(f"  Active wavefronts distribution: min={min(signal_counts)}, max={max(signal_counts)}, mean={sum(signal_counts)/len(signal_counts):.2f}")

def main():
    archive_path = "cartpole.axic"
    if len(sys.argv) > 1:
        archive_path = sys.argv[1]
        
    if not os.path.exists(archive_path) and os.path.exists(os.path.join("w:\\Workspace", archive_path)):
        archive_path = os.path.join("w:\\Workspace", archive_path)
        
    if not os.path.exists(archive_path):
        print(f"Error: Archive '{archive_path}' not found.")
        sys.exit(1)
        
    # Open and extract blobs from axic
    with open(archive_path, "rb") as f:
        # Read header
        f.seek(0)
        magic, version, file_count = struct.unpack("<III", f.read(12))
        
        toc = []
        for _ in range(file_count):
            entry_bytes = f.read(272)
            path_bytes = entry_bytes[0:256]
            null_idx = path_bytes.find(b'\0')
            path = path_bytes[:null_idx].decode("utf-8", errors="replace")
            offset, size = struct.unpack("<QQ", entry_bytes[256:272])
            toc.append((path, offset, size))
            
        state_data = None
        axons_data = None
        
        for path, offset, size in toc:
            if path.endswith(".state"):
                f.seek(offset)
                state_data = f.read(size)
            elif path.endswith(".axons"):
                f.seek(offset)
                axons_data = f.read(size)
                
        if state_data:
            inspect_state(state_data)
        else:
            print("No .state file found in archive.")
            
        if axons_data:
            inspect_axons(axons_data)
        else:
            print("No .axons file found in archive.")

if __name__ == "__main__":
    main()
