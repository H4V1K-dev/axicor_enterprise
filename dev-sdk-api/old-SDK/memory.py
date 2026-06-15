import os
import mmap
import struct
import numpy as np

from .platform import get_shm_path

class AxicorMemory:
    # Strict C-ABI v3 (128 bytes) -> axicor-core/src/ipc.rs
    SHM_HEADER_FMT = "<IBBHIIIIQIIIIIIIIIII13I"
    SHM_HEADER_SIZE = 128
    MAGIC = 0x41584943  # "AXIC"

    def __init__(self, zone_hash: int, read_only: bool = False):
        self.zone_hash = zone_hash
        path = get_shm_path(zone_hash)
        
        mode = os.O_RDONLY if read_only else os.O_RDWR
        access = mmap.ACCESS_READ if read_only else mmap.ACCESS_WRITE
        
        fd = os.open(path, mode)
        self._mm = mmap.mmap(fd, 0, access=access)
        os.close(fd)
        
        # 1. Read header
        header = struct.unpack_from(self.SHM_HEADER_FMT, self._mm, 0)
        assert header[0] == self.MAGIC, f"Invalid SHM Magic in {path}"
        
        self.padded_n = header[4]
        self.dendrite_slots = header[5]
        self.weights_offset = header[6]
        self.targets_offset = header[7]
        self.flags_offset = header[16]
        self.voltage_offset = header[17]
        self.threshold_offset_offset = header[18]
        self.timers_offset = header[19]
        
        assert self.dendrite_slots == 128, "C-ABI violation: dendrite_slots != 128"
        
        # 2. Map matrices directly onto OS shared memory
        self.weights = np.ndarray(
            (self.dendrite_slots, self.padded_n), 
            dtype=np.int32, 
            buffer=self._mm, 
            offset=self.weights_offset
        )
        
        self.targets = np.ndarray(
            (self.dendrite_slots, self.padded_n), 
            dtype=np.uint32, 
            buffer=self._mm, 
            offset=self.targets_offset
        )
        
        self.flags = np.ndarray(
            (self.padded_n,),
            dtype=np.uint8,
            buffer=self._mm,
            offset=self.flags_offset
        )

        self.voltage = np.ndarray(
            (self.padded_n,),
            dtype=np.int32,
            buffer=self._mm,
            offset=self.voltage_offset
        )

        self.threshold_offset = np.ndarray(
            (self.padded_n,),
            dtype=np.int32,
            buffer=self._mm,
            offset=self.threshold_offset_offset
        )

        self.timers = np.ndarray(
            (self.padded_n,),
            dtype=np.uint8,
            buffer=self._mm,
            offset=self.timers_offset
        )

    def save_checkpoint(self, filepath: str):
        """
        Zero-Copy dump of physical graph state into a compressed NumPy archive.
        Dumps weights, targets (topology), and homeostasis flags.
        """
        np.savez_compressed(filepath, 
                            weights=self.weights, 
                            targets=self.targets, 
                            flags=self.flags)

    def load_checkpoint(self, filepath: str):
        """
        In-Place state loading from checkpoint directly into Shared Memory.
        Strictly adheres to C-ABI offsets, maintaining mmap integrity.
        """
        # data  dictionary of objects from the archive
        with np.load(filepath) as data:
            # Shape Safety: Protection against Silent Data Corruption on connectome mismatch
            assert data['weights'].shape == self.weights.shape, f"Weights shape mismatch: {data['weights'].shape} vs {self.weights.shape}"
            assert data['targets'].shape == self.targets.shape, f"Targets shape mismatch: {data['targets'].shape} vs {self.targets.shape}"
            assert data['flags'].shape == self.flags.shape, f"Flags shape mismatch: {data['flags'].shape} vs {self.flags.shape}"

            # LAW: Strict In-Place Copy. 
            # self.weights = data['weights'] is FORBIDDEN as it breaks mmap linkage.
            self.weights[:] = data['weights']
            self.targets[:] = data['targets']
            self.flags[:] = data['flags']

            # Force memory page flush to /dev/shm for immediate Rust kernel reaction
            self._mm.flush()

    @staticmethod
    def pack_targets(axon_ids: np.ndarray, segment_offsets: np.ndarray) -> np.ndarray:
        """
        Strict C-ABI Packer (Zero-Index Trap Protection).
        [31..24] segment_offset (8 bit) | [23..0] axon_id + 1 (24 bit)
        """
        assert np.all(axon_ids >= -1) and np.all(axon_ids <= 0x00FFFFFE), "axon_id out of range (-1..16777214)"
        assert np.all(segment_offsets >= 0) and np.all(segment_offsets <= 255), "segment_offset out of range (0..255)"

        # [DOD FIX] Hardware Early Exit Protection. Empty slots must be strictly 0.
        packed = (segment_offsets.astype(np.uint32) << 24) | (axon_ids.astype(np.int32) + 1).astype(np.uint32)
        return np.where(axon_ids == -1, np.uint32(0), packed)

    @staticmethod
    def unpack_targets(packed_targets: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
        """
        Strict C-ABI Unpacker.
        Returns: (axon_ids, segment_offsets)
        """
        axon_ids = (packed_targets & 0x00FFFFFF).astype(np.int32) - 1
        segment_offsets = packed_targets >> 24
        return axon_ids, segment_offsets

    def extract_topology(self, soma_idx: int) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
        """
        Vectorized geometry extraction for a specific neuron.
        Solves the Zero-Index Trap without Python loops.
        Returns: (axon_ids, segment_offsets, weights)
        """
        # Dendrite column slice for a single soma (O(1) pointer view)
        raw_targets = self.targets[:, soma_idx]
        raw_weights = self.weights[:, soma_idx]
        
        # Filter empty slots
        valid_mask = raw_targets != 0
        valid_targets = raw_targets[valid_mask]
        active_weights = raw_weights[valid_mask]
        
        # Unpack u32 via static method
        axon_ids, seg_offsets = self.unpack_targets(valid_targets)
        
        return axon_ids, seg_offsets, active_weights

    def get_network_stats(self) -> dict:
        """Scans VRAM (Zero-Copy) and returns graph statistics."""
        # Filter zero targets (empty slots)
        valid_mask = self.targets != 0
        active_weights = self.weights[valid_mask]
        
        if len(active_weights) == 0:
            return {"active_synapses": 0, "avg_weight": 0.0, "max_weight": 0}
            
        # [DOD FIX] Shift Mass Domain back to Charge Domain for human-readable telemetry
        return {
            "active_synapses": int(np.sum(valid_mask)),
            "avg_weight": float(np.mean(np.abs(active_weights))) / 65536.0,
            "max_weight": int(np.max(np.abs(active_weights))) // 65536
        }

    def distill_graph(self, prune_threshold: int) -> int:
        # [DOD FIX] Translate threshold into Mass Domain (i32)
        mass_threshold = prune_threshold << 16
        
        # Vectorized search for weak noise
        weak_mask = (self.targets != 0) & (np.abs(self.weights) < mass_threshold)
        pruned_count = int(np.sum(weak_mask))
        
        # Physical destruction
        self.targets[weak_mask] = 0
        self.weights[weak_mask] = 0
        
        return pruned_count

    def clear_weights(self):
        """
        Tabula Rasa: Total zero-out of all weights in the shard.
        Retains topology (connections) but makes them "weightless".
        """
        self.weights.fill(0)

    def close(self):
        self._mm.close()
