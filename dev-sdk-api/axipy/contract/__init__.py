from .contract_generated import (
    SpikeBatchHeaderV2,
    SpikeEventV2,
    TelemetryFrameHeader,
    ExternalIoHeader,
    ControlPacket,
    AxonHandoverEvent,
    AxonHandoverPrune,
    BakeRequest,
    AxonHandoverAck,
    GhostConnection
)

# Hardware limits and structure sizes not included in auto-generation
MAX_NEURON_TYPES_PER_SHARD = 16  # Limit of 4-bit type mask
NEURON_SIZE = 1166               # Physical size of Neuron struct in Rust
AXON_SIZE = 32                   # Physical size of Axon struct in Rust
