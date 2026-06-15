class BuilderError(Exception):
    """Base exception for model design errors."""
    pass


class BuilderValidationError(BuilderError):
    """Base class for topology and physics invariant violations."""
    pass


class NeuronTypeLimitExceededError(BuilderValidationError):
    """Neuron type limit per shard exceeded (INV-CROSS-SHARD-LIMIT-001)."""

    def __init__(self, shard_name, count, limit):
        super().__init__(
            f"Validation Failed in Shard '{shard_name}': "
            f"Exceeded maximum allowed neuron types. "
            f"Used: {count}, Limit: {limit}"
        )
        self.shard_name = shard_name
        self.count = count
        self.limit = limit


class PhysicsDriftError(BuilderValidationError):
    """Fractional v_seg step threatening impulse physics drift (INV-BUILDER-PHYSICS-002)."""

    def __init__(self, raw_v_seg, suggested_speed):
        super().__init__(
            f"Physical Validation Failed: v_seg must be an integer (calculated: {raw_v_seg:.5f}). "
            f"Suggest changing signal_speed_m_s to {suggested_speed:.4f}"
        )
        self.raw_v_seg = raw_v_seg
        self.suggested_speed = suggested_speed


class InvalidAnatomyHeightError(BuilderValidationError):
    """Shard layer heights do not sum to 1.0 (INV-BUILDER-ANATOMY-003)."""
    pass


class InvalidLayerCompositionError(BuilderValidationError):
    """Neuron population fractions within a layer do not sum to 1.0 (INV-BUILDER-QUOTAS-004)."""
    pass


class InvalidDensityError(BuilderValidationError):
    """Negative neuron distribution density (INV-BUILDER-DENSITY-005)."""
    pass


class InvalidEntryZError(BuilderValidationError):
    """Invalid Z-coordinate or coordinate label for an I/O port (INV-BUILDER-ENTRYZ-006)."""
    pass


class SocketConnectionError(BuilderValidationError):
    """Socket switching/routing errors (INV-CROSS-TOPOLOGY-001)."""
    pass