# Generated automatically by wire/build.rs. Do not edit manually.
# ==============================================================================
# CONTRACT VERSIONING RULES (SemVer):
# Contract Version: 0.1.0
# - MAJOR (X): Incremented on breaking C-ABI layout changes (e.g., changing fields,
#             sizes, or alignments in existing structures).
# - MINOR (Y): Incremented when adding new structures without modifying existing ones.
# - PATCH (Z): Incremented on internal optimizations that do not affect binary layout.
# ==============================================================================

import ctypes

__version__ = "0.1.0"

class SpikeBatchHeaderV2(ctypes.LittleEndianStructure):
    _pack_ = 4
    _fields_ = [
        ("src_zone_hash", ctypes.c_uint32),
        ("dst_zone_hash", ctypes.c_uint32),
        ("epoch", ctypes.c_uint32),
        ("chunk_idx", ctypes.c_uint16),
        ("total_chunks", ctypes.c_uint16),
    ]

class SpikeEventV2(ctypes.LittleEndianStructure):
    _pack_ = 4
    _fields_ = [
        ("ghost_id", ctypes.c_uint32),
        ("tick_offset", ctypes.c_uint32),
    ]

class TelemetryFrameHeader(ctypes.LittleEndianStructure):
    _pack_ = 8
    _fields_ = [
        ("magic", (ctypes.c_uint8 * 4)),
        ("tick", ctypes.c_uint32),
        ("count", ctypes.c_uint32),
        ("_padding", ctypes.c_uint32),
    ]

class ExternalIoHeader(ctypes.LittleEndianStructure):
    _pack_ = 4
    _fields_ = [
        ("magic", (ctypes.c_uint8 * 4)),
        ("zone_hash", ctypes.c_uint32),
        ("matrix_hash", ctypes.c_uint32),
        ("payload_size", ctypes.c_uint32),
        ("global_reward", ctypes.c_int16),
        ("_padding", ctypes.c_uint16),
    ]

class ControlPacket(ctypes.LittleEndianStructure):
    _pack_ = 8
    _fields_ = [
        ("magic", (ctypes.c_uint8 * 4)),
        ("dopamine", ctypes.c_int16),
        ("_pad", ctypes.c_uint16),
    ]

class AxonHandoverEvent(ctypes.LittleEndianStructure):
    _pack_ = 4
    _fields_ = [
        ("origin_zone_hash", ctypes.c_uint32),
        ("local_axon_id", ctypes.c_uint32),
        ("entry_x", ctypes.c_uint16),
        ("entry_y", ctypes.c_uint16),
        ("vector_x", ctypes.c_int8),
        ("vector_y", ctypes.c_int8),
        ("vector_z", ctypes.c_int8),
        ("type_mask", ctypes.c_uint8),
        ("remaining_length", ctypes.c_uint16),
        ("entry_z", ctypes.c_uint8),
        ("_padding", ctypes.c_uint8),
    ]

class AxonHandoverPrune(ctypes.LittleEndianStructure):
    _pack_ = 4
    _fields_ = [
        ("target_zone_hash", ctypes.c_uint32),
        ("receiver_zone_hash", ctypes.c_uint32),
        ("dst_ghost_id", ctypes.c_uint32),
    ]

class BakeRequest(ctypes.LittleEndianStructure):
    _pack_ = 4
    _fields_ = [
        ("magic", (ctypes.c_uint8 * 4)),
        ("zone_hash", ctypes.c_uint32),
        ("current_tick", ctypes.c_uint32),
        ("prune_threshold", ctypes.c_int16),
        ("max_sprouts", ctypes.c_uint16),
    ]

class AxonHandoverAck(ctypes.LittleEndianStructure):
    _pack_ = 4
    _fields_ = [
        ("target_zone_hash", ctypes.c_uint32),
        ("receiver_zone_hash", ctypes.c_uint32),
        ("src_axon_id", ctypes.c_uint32),
        ("dst_ghost_id", ctypes.c_uint32),
    ]

class GhostConnection(ctypes.LittleEndianStructure):
    _pack_ = 4
    _fields_ = [
        ("src_soma_id", ctypes.c_uint32),
        ("target_ghost_id", ctypes.c_uint32),
    ]

