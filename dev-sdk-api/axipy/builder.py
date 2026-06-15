import ctypes
import toml
import subprocess
from pathlib import Path
import warnings
from typing import Union, List, Dict, Tuple, Optional, Any

from axipy.contract import (
    SpikeBatchHeaderV2,
    SpikeEventV2,
    TelemetryFrameHeader,
    ExternalIoHeader,
    ControlPacket,
    AxonHandoverEvent,
    AxonHandoverPrune,
    BakeRequest,
    AxonHandoverAck,
    GhostConnection,
    MAX_NEURON_TYPES_PER_SHARD,
    NEURON_SIZE,
    AXON_SIZE,
)
from axipy.errors import (
    BuilderError,
    BuilderValidationError,
    NeuronTypeLimitExceededError,
    PhysicsDriftError,
    InvalidAnatomyHeightError,
    InvalidLayerCompositionError,
    InvalidDensityError,
    InvalidEntryZError,
    SocketConnectionError,
)

class NeuronBlueprint:
    """Wrapper over a loaded and parsed TOML file describing a neuron type."""

    def __init__(self, filepath: str, data: Union[dict, list]) -> None:
        """
        Initialize a neuron blueprint.

        filepath: Path to the neuron type description file.
        data: List or dict containing neuron physical parameters.
        """
        self.filepath = filepath
        self.data = data

    def set_plasticity(self, pot: int, dep: int) -> 'NeuronBlueprint':
        """
        [COLD] Configure STDP plasticity parameters: potentiation and depression.
        Returns self to support the Fluent API.
        """
        if isinstance(self.data, dict):
            self.data["gsop_potentiation"] = pot
            self.data["gsop_depression"] = dep
        elif isinstance(self.data, list):
            for item in self.data:
                if isinstance(item, dict):
                    item["gsop_potentiation"] = pot
                    item["gsop_depression"] = dep
        return self


class Layer:
    """Anatomical layer within a shard."""

    def __init__(self, name: str, height_pct: float, density: float) -> None:
        self.name = name
        self.height_pct = height_pct
        self.density = density
        self.populations: List[Tuple[NeuronBlueprint, float]] = []


class Socket:
    """Switching point on a shard (plug or socket) for inter-shard connections."""

    def __init__(self, name: str, shard: 'Shard', width: int, height: int, entry_z: Optional[Union[str, float, int]] = None) -> None:
        self.name = name
        self.shard = shard
        self.width = width
        self.height = height
        self.entry_z = entry_z
        self.connections: List[str] = []

    def connect_to(self, target_path: str) -> 'Socket':
        """
        [COLD] Routes an axon tract to an incoming socket on another shard.

        target_path: Path in the format "ShardName.IncomingSocketName".
        Returns self.
        """
        self.connections.append(target_path)
        return self


class Shard:
    """Anatomical voxel volume (zone) containing layers and neuron populations."""

    def __init__(self, name: str, x: int, y: int, z: int) -> None:
        self.name = name
        self.x = x
        self.y = y
        self.z = z
        self.layers: List[Layer] = []
        self.input_ports: List[Dict[str, Any]] = []
        self.output_ports: List[Dict[str, Any]] = []
        self.sockets: List[Socket] = []

    def _validate_entry_z(self, entry_z: Any) -> None:
        if isinstance(entry_z, str):
            if entry_z not in ["top", "mid", "bottom"]:
                raise InvalidEntryZError(f"Invalid entry_z string label: '{entry_z}'")
        elif isinstance(entry_z, (int, float)):
            if not (0.0 <= float(entry_z) <= 1.0):
                raise InvalidEntryZError(f"Invalid entry_z numeric value: {entry_z} (must be in [0.0, 1.0])")
        else:
            raise InvalidEntryZError(f"Invalid entry_z type: {type(entry_z)}")

    def add_layer(self, name: str, height_pct: float, density: float) -> 'Shard':
        if density < 0.0:
            raise InvalidDensityError(f"Negative density is not allowed: {density}")
        layer = Layer(name, height_pct, density)
        self.layers.append(layer)
        return self

    def add_population(self, layer_name: str, blueprint: NeuronBlueprint, fraction: float) -> 'Shard':
        layer = None
        for l in self.layers:
            if l.name == layer_name:
                layer = l
                break
        if layer is None:
            raise BuilderValidationError(f"Layer '{layer_name}' not found in Shard '{self.name}'")
        layer.populations.append((blueprint, fraction))
        return self

    def add_input_port(self, name: str, width: int, height: int, entry_z: Union[str, float, int],
                       target_type: str = "All", growth_steps: int = 1000) -> 'Shard':
        self._validate_entry_z(entry_z)
        self.input_ports.append({
            "name": name,
            "width": width,
            "height": height,
            "entry_z": entry_z,
            "target_type": target_type,
            "growth_steps": growth_steps
        })
        return self

    def add_output_port(self, name: str, width: int, height: int) -> 'Shard':
        self.output_ports.append({
            "name": name,
            "width": width,
            "height": height
        })
        return self

    def add_socket(self, name: str, width: int, height: int, entry_z: Optional[Union[str, float, int]] = None) -> 'Socket':
        if entry_z is not None:
            self._validate_entry_z(entry_z)
        socket = Socket(name, self, width, height, entry_z)
        self.sockets.append(socket)
        return socket

    def estimate_neurons(self) -> int:
        total = 0.0
        for layer in self.layers:
            layer_volume = self.x * self.y * (self.z * layer.height_pct)
            total += layer_volume * layer.density
        return int(round(total))


class Department:
    """Logical group (brain region) aggregating related shards."""

    def __init__(self, name: str) -> None:
        self.name = name
        self.shards: List[Shard] = []

    def add_shard(self, *shards: Shard) -> 'Department':
        for s in shards:
            self.shards.append(s)
        return self


class ModelBuilder:
    """Root configurator and validator for the neural network structure."""

    def __init__(self, project_name: str, output_dir: str) -> None:
        self.project_name = project_name
        self.output_dir = output_dir
        self.departments: List[Department] = []

        # Default simulation physics parameters
        self.sim_params = {
            "signal_speed_m_s": 2.0,
            "tick_duration_us": 1000.0,
            "voxel_size_um": 1000.0,
            "segment_length_voxels": 2
        }

    def add_department(self, *depts: Department) -> 'ModelBuilder':
        for d in depts:
            self.departments.append(d)
        return self

    def gnm_lib(self, path: str) -> NeuronBlueprint:
        p = Path(path)
        if not p.is_absolute():
            p = (Path(self.output_dir).parent / path).resolve()
            if not p.exists():
                p = Path(path).resolve()

        if not p.exists():
            raise BuilderError(f"Blueprint file not found: {p}")

        try:
            with open(p, "r", encoding="utf-8") as f:
                data = toml.load(f)
        except Exception as e:
            raise BuilderError(f"Failed to load blueprint TOML from {p}: {e}")

        return NeuronBlueprint(str(p), data)
        
    def build(self, dry_run: bool = False, print_level: str = "all") -> 'ModelBuilder':
        # 1. Validate layer heights in each shard
        for dept in self.departments:
            for shard in dept.shards:
                if not shard.layers:
                    continue
                total_height = sum(layer.height_pct for layer in shard.layers)
                if abs(total_height - 1.0) > 1e-4:
                    raise InvalidAnatomyHeightError(
                        f"Validation Failed in Shard '{shard.name}': "
                        f"Sum of layer heights must be 1.0 (calculated: {total_height:.5f})"
                    )

        # 2. Validate layer composition (population fractions)
        for dept in self.departments:
            for shard in dept.shards:
                for layer in shard.layers:
                    if not layer.populations:
                        continue
                    total_fraction = sum(fraction for _, fraction in layer.populations)
                    if abs(total_fraction - 1.0) > 1e-4:
                        raise InvalidLayerCompositionError(
                            f"Validation Failed in Layer '{layer.name}' of Shard '{shard.name}': "
                            f"Sum of population fractions must be 1.0 (calculated: {total_fraction:.5f})"
                        )

        # 3. Validate neuron type limit per shard
        for dept in self.departments:
            for shard in dept.shards:
                unique_blueprints = set()
                for layer in shard.layers:
                    for bp, _ in layer.populations:
                        unique_blueprints.add(bp.filepath)
                if len(unique_blueprints) > MAX_NEURON_TYPES_PER_SHARD:
                    raise NeuronTypeLimitExceededError(shard.name, len(unique_blueprints), MAX_NEURON_TYPES_PER_SHARD)

        # 4. Validate physics drift (v_seg)
        signal_speed_m_s = self.sim_params["signal_speed_m_s"]
        tick_duration_us = self.sim_params["tick_duration_us"]
        voxel_size_um = self.sim_params["voxel_size_um"]
        segment_length_voxels = self.sim_params.get("segment_length_voxels", 2)

        v_seg_raw = (signal_speed_m_s * tick_duration_us) / (voxel_size_um * segment_length_voxels)

        if abs(v_seg_raw - round(v_seg_raw)) > 1e-5:
            suggested_speed = (round(v_seg_raw) * voxel_size_um * segment_length_voxels) / tick_duration_us
            raise PhysicsDriftError(v_seg_raw, suggested_speed)

        # 5. Topological validation of socket connections (INV-CROSS-TOPOLOGY-001)
        incoming_sockets = {}  # path -> socket
        outgoing_sockets = []

        for dept in self.departments:
            for shard in dept.shards:
                for socket in shard.sockets:
                    path = f"{shard.name}.{socket.name}"
                    if socket.entry_z is not None:
                        incoming_sockets[path] = socket
                    else:
                        outgoing_sockets.append(socket)

        incoming_connections = {path: [] for path in incoming_sockets.keys()}

        for out_socket in outgoing_sockets:
            for target_path in out_socket.connections:
                if target_path not in incoming_sockets:
                    target_exists = False
                    for dept in self.departments:
                        for shard in dept.shards:
                            for s in shard.sockets:
                                if f"{shard.name}.{s.name}" == target_path:
                                    target_exists = True
                                    break
                    if not target_exists:
                        raise SocketConnectionError(f"Connection target '{target_path}' not found in model.")
                    else:
                        raise SocketConnectionError(f"Connection target '{target_path}' is not an incoming socket (entry_z is None).")

                # Validate socket dimension match
                in_socket = incoming_sockets[target_path]
                if out_socket.width != in_socket.width or out_socket.height != in_socket.height:
                    raise BuilderValidationError(
                        f"Dimension mismatch between socket '{out_socket.shard.name}.{out_socket.name}' "
                        f"({out_socket.width}x{out_socket.height}) and target socket '{target_path}' "
                        f"({in_socket.width}x{in_socket.height})."
                    )

                incoming_connections[target_path].append(out_socket)

        for path, out_list in incoming_connections.items():
            if len(out_list) == 0:
                raise SocketConnectionError(f"Incoming socket '{path}' is not connected to any outgoing socket (orphan).")
            elif len(out_list) > 1:
                connected_names = [f"{s.shard.name}.{s.name}" for s in out_list]
                raise SocketConnectionError(
                    f"Incoming socket '{path}' is connected to multiple outgoing sockets: {connected_names}"
                )

        # 6. Write TOML configurations (if dry_run=False)
        if not dry_run:
            out_path = Path(self.output_dir)
            out_path.mkdir(parents=True, exist_ok=True)

            # Write simulation.toml
            sim_config = {
                "project_name": self.project_name,
                "simulation": self.sim_params,
                "departments": [dept.name for dept in self.departments]
            }
            with open(out_path / "simulation.toml", "w", encoding="utf-8") as f:
                toml.dump(sim_config, f)

            # Write <dept_name>.toml for each department
            for dept in self.departments:
                dept_config = {
                    "name": dept.name,
                    "shards": []
                }
                for shard in dept.shards:
                    shard_data = {
                        "name": shard.name,
                        "size": [shard.x, shard.y, shard.z],
                        "layers": [
                            {
                                "name": l.name,
                                "height_pct": l.height_pct,
                                "density": l.density,
                                "populations": [
                                    {
                                        "blueprint": bp.filepath,
                                        "fraction": frac
                                    } for bp, frac in l.populations
                                ]
                            } for l in shard.layers
                        ],
                        "input_ports": shard.input_ports,
                        "output_ports": shard.output_ports,
                        "sockets": [
                            {
                                "name": s.name,
                                "width": s.width,
                                "height": s.height,
                                "entry_z": s.entry_z,
                                "connections": s.connections
                            } for s in shard.sockets
                        ]
                    }
                    dept_config["shards"].append(shard_data)

                with open(out_path / f"{dept.name}.toml", "w", encoding="utf-8") as f:
                    toml.dump(dept_config, f)

            # Generate model passport (model_pass.md)
            pass_content = f"# Model Passport: {self.project_name}\n\n"
            pass_content += "## Simulation Parameters\n"
            for k, v in self.sim_params.items():
                pass_content += f"- **{k}**: {v}\n"
            pass_content += "\n## Resource Estimates\n"
            stats = self.dry_run_stats()
            for k, v in stats.items():
                pass_content += f"- **{k}**: {v}\n"

            with open(out_path / "model_pass.md", "w", encoding="utf-8") as f:
                f.write(pass_content)

        return self

    def dry_run_stats(self) -> Dict[str, Any]:
        total_neurons = 0
        total_axons = 0

        for dept in self.departments:
            for shard in dept.shards:
                neurons = shard.estimate_neurons()
                padded_neurons = ((neurons + 63) // 64) * 64

                ports_capacity = sum(p["width"] * p["height"] for p in shard.input_ports)
                ghost_capacity = sum(s.width * s.height for s in shard.sockets if s.entry_z is not None)

                total_neurons += neurons
                total_axons += (padded_neurons + ports_capacity + ghost_capacity)

        # Compute VRAM requirements using physical constants from contracts
        vram_neurons = total_neurons * NEURON_SIZE
        vram_axons = total_axons * AXON_SIZE
        total_vram = vram_neurons + vram_axons

        # IPC buffer overhead is measured using ctypes.sizeof.
        # Header structs SpikeBatchHeaderV2, ExternalIoHeader, ControlPacket
        # are imported and measured dynamically via ctypes.sizeof(SpikeBatchHeaderV2).
        ipc_buffer_overhead = (
            ctypes.sizeof(SpikeBatchHeaderV2) +
            ctypes.sizeof(ExternalIoHeader) +
            ctypes.sizeof(ControlPacket)
        )

        return {
            "total_neurons": total_neurons,
            "total_axons": total_axons,
            "vram_neurons_bytes": vram_neurons,
            "vram_axons_bytes": vram_axons,
            "vram_total_bytes": total_vram,
            "ipc_buffer_overhead_bytes": ipc_buffer_overhead
        }

    def bake(self, filename: str) -> 'ModelBuilder':
        config_file = Path(self.output_dir) / "simulation.toml"
        if not config_file.exists():
            raise BuilderValidationError("Cannot run bake(): simulation.toml config not found. Run build() first.")

        try:
            cmd = ["baker-cli", "-c", str(config_file), "-o", filename]
            result = subprocess.run(cmd, capture_output=True, text=True, check=True)
            if print_level := getattr(self, "_print_level", "all"):
                if print_level != "none":
                    print(f"baker-cli output: {result.stdout}")
        except FileNotFoundError:
            raise BuilderError("baker-cli executable not found in PATH.")
        except subprocess.CalledProcessError as e:
            raise BuilderError(f"baker-cli compilation failed: {e.stderr}")

        return self