import os
import toml
from typing import Dict
from .control import AxicorControl
from .memory import AxicorMemory
from .utils import fnv1a_32

class Zone:
    """Represents a single brain zone (e.g., SensoryCortex)."""
    def __init__(self, name: str, baked_dir: str):
        self.name = name
        self.hash = fnv1a_32(name.encode('utf-8'))
        
        self.manifest_path = os.path.join(baked_dir, "manifest.toml")
        
        # Control Plane is always available (even if the node is offline)
        self.control = AxicorControl(self.manifest_path)
        self._memory = None

    @property
    def memory(self) -> AxicorMemory:
        """Lazy initialization of Memory Plane (mmap). Will fail if node is not running."""
        if self._memory is None:
            self._memory = AxicorMemory(self.hash)
        return self._memory

class AxicorClusterControl:
    """Global controller: applies commands to all cluster zones simultaneously."""
    def __init__(self, zones: Dict[str, Zone]):
        self._zones = zones

    def set_night_interval(self, ticks: int):
        for zone in self._zones.values():
            zone.control.set_night_interval(ticks)

    def set_prune_threshold(self, threshold: int):
        for zone in self._zones.values():
            zone.control.set_prune_threshold(threshold)

    def set_dopamine_receptors(self, variant_id: int, d1_affinity: int, d2_affinity: int):
        for zone in self._zones.values():
            zone.control.set_dopamine_receptors(variant_id, d1_affinity, d2_affinity)

    def set_max_sprouts(self, max_sprouts: int):
        for zone in self._zones.values():
            zone.control.set_max_sprouts(max_sprouts)

class AxicorBrain:
    """
    Unified entry point for multi-zone connectome management.
    Automatically reads topology from brain.toml.
    """
    def __init__(self, brain_toml_path: str):
        self.brain_toml_path = brain_toml_path
        self.zones: Dict[str, Zone] = {}

        if not os.path.exists(brain_toml_path):
            raise FileNotFoundError(f"Brain configuration not found: {brain_toml_path}")

        with open(brain_toml_path, "r", encoding="utf-8") as f:
            data = toml.load(f)

        for zone_data in data.get("zone", []):
            name = zone_data["name"]
            baked_dir = zone_data.get("baked_dir", f"baked/{name}/")
            self.zones[name] = Zone(name, baked_dir)

        # Cluster management interface
        self.control = AxicorClusterControl(self.zones)
