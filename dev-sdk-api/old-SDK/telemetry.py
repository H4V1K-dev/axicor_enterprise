import struct
import threading
import asyncio
import numpy as np
from typing import Optional

try:
    import websockets
except ImportError:
    raise ImportError("Telemetry requires the websockets package (pip install websockets)")

# Contract from axicor-core/src/ipc.rs and axicor-lab/src/telemetry.rs
# 0..4: Magic "SPIK" (0x4B495053)
# 4..12: Tick (u64)
# 12..16: Spikes Count (u32)
HEADER_FORMAT = "<IQI"
HEADER_SIZE = 16
TELE_MAGIC = int.from_bytes(b"SPIK", "little")

class TelemetryListener:
    """
    Lock-Free background spike listener.
    Isolates slow WebSocket I/O from the HFT environment loop.
    """
    def __init__(self, host: str = "127.0.0.1", port: int = 9003, max_neurons: int = 1_000_000):
        self.ws_url = f"ws://{host}:{port}/ws"
        self.max_neurons = max_neurons
        
        # Shared state. 
        # Background thread writes ones, main thread applies decay and reads.
        self._heatmap = np.zeros(self.max_neurons, dtype=np.float32)
        self._latest_tick = 0
        
        self._stop_event = threading.Event()
        self._thread = threading.Thread(target=self._run_loop, daemon=True, name="Axicor-Telemetry-Rx")
        self._thread.start()

    def _run_loop(self):
        """Entry point for the isolated OS thread."""
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        loop.run_until_complete(self._ws_listener())

    async def _ws_listener(self):
        while not self._stop_event.is_set():
            try:
                async with websockets.connect(self.ws_url) as ws:
                    print(f" [Telemetry] Connected to {self.ws_url}")
                    async for message in ws:
                        if self._stop_event.is_set():
                            break
                        if isinstance(message, bytes):
                            self._process_frame(message)
            except Exception:
                # Silent reconnect to avoid console spam
                await asyncio.sleep(1.0)

    def _process_frame(self, frame: bytes):
        if len(frame) < HEADER_SIZE:
            return

        magic, tick, count = struct.unpack_from(HEADER_FORMAT, frame, 0)
        
        if magic != TELE_MAGIC:
            return

        expected_size = HEADER_SIZE + count * 4
        if len(frame) < expected_size:
            return

        # Zero-Copy cast of packet tail into u32
        spikes = np.frombuffer(frame, dtype=np.uint32, count=count, offset=HEADER_SIZE)
        
        # Update state
        self._latest_tick = tick
        if count > 0:
            # Bulk (vectorized) lock-free write
            # Filter network garbage to prevent SegFaults
            valid_spikes = spikes[spikes < self.max_neurons]
            self._heatmap[valid_spikes] = 1.0

    def get_snapshot(self, decay: float = 0.8) -> tuple[int, np.ndarray]:
        """
        Called in the main thread (RL Agent / Dashboard).
        Applies fade-out to the heatmap and returns it.
        decay: persistence multiplier (0.0 = current tick spikes only, 1.0 = infinite glow)
        """
        # In-place multiplication to avoid allocations
        self._heatmap *= decay
        return self._latest_tick, self._heatmap

    def stop(self):
        self._stop_event.set()
        if self._thread.is_alive():
            self._thread.join(timeout=1.0)
