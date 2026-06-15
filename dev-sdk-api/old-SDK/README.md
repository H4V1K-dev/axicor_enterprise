# Axicor Python SDK

Official Python interface for the Axicor Spiking Neural Network (SNN) Engine.

> ### CRITICAL: THE 10ms BUDGET & ZERO-GARBAGE LAW
> Axicor operates on a hard real-time BSP (Barrier Synchronization) cycle. You have exactly **10ms** to complete your observation-action loop.
>
> **You MUST NOT perform any heap allocations** (creating new numpy arrays, tuples, or large objects) inside the hot loop. Triggering the Python Garbage Collector (GC) will cause a micro-stutter, resulting in dropped UDP packets and **Biological Amnesia** (loss of brain state).

## Zero-Garbage Production Pattern

### The Correct Way (Zero-Allocation)
```python
import numpy as np
from axicor.encoders import PopulationEncoder

# 1. PRE-ALLOCATE everything outside the loop. Reused every tick.
encoder = PopulationEncoder(variables_count=4, neurons_per_var=16, batch_size=20)
# ✅ Pre-allocate buffer once
frame_buf = np.zeros(encoder.total_bytes + 20, dtype=np.uint8)

while True:
    obs = env.get_observation()
    # 2. ✅ encode_into() writes directly into the pre-allocated buffer
    # Offset=20 leaves space for the C-ABI ExternalIoHeader
    encoder.encode_into(obs, frame_buf, offset=20)
    
    # 3. Send via zero-copy UDP
    client.send(frame_buf)
```

### The Wrong Way (Triggers GC)
```python
while True:
    obs = env.get_observation()
    # ❌ WRONG: This creates a NEW array object every tick.
    # This triggers malloc() and will eventually force a GC pause.
    frame = encoder.encode(obs)
    client.send(frame)
```

## Installation
```bash
pip install axicor-client
```

## License
Dual-licensed under MIT or Apache 2.0.
