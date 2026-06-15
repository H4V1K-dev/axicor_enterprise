import os
import toml
import numpy as np
from typing import Dict, Any
from .builder import IoMatrixDesigner
from .encoders import PopulationEncoder, PwmEncoder
from .decoders import PwmDecoder, PopulationDecoder
from .axic import AxicReader
from .utils import fnv1a_32 # DOD FIX: Fixed Circular Import

class AxicorIoContract:
    def __init__(self, axic_path: str, zone_name: str):
        self.zone_hash = fnv1a_32(zone_name.encode('utf-8'))
        reader = AxicReader(axic_path)
        
        # [DOD] SDK loads the manifest from the archive. 
        # Path in archive: {zone_name}/io.toml (copied to BrainDNA during baking)
        # UPDATE: Instruction specifies f"{zone_name}/io.toml"
        io_bytes = reader.read_file(f"{zone_name}/io.toml")
        if not io_bytes:
            # Try alternative path for backward compatibility if the first one fails
            io_bytes = reader.read_file(f"baked/{zone_name}/BrainDNA/io.toml")
            
        if not io_bytes:
            raise FileNotFoundError(f"io.toml not found in {axic_path} for zone {zone_name}")
            
        self.data = toml.loads(io_bytes.decode('utf-8'))
        
        # [DOD FIX] SDK must map hashes to physical PIN names, not abstract matrix names.
        # We index self.inputs/outputs by PIN names for direct L7 resolution.
        self.inputs = {}
        for matrix in self.data.get("input", []):
            for pin in matrix.get("pin", []):
                self.inputs[pin["name"]] = pin

        self.outputs = {}
        for matrix in self.data.get("output", []):
            for pin in matrix.get("pin", []):
                self.outputs[pin["name"]] = pin

    def get_client_config(self, batch_size: int) -> Dict[str, Any]:
        """Returns kwargs for unpacking into AxicorMultiClient."""
        matrices = []
        rx_layout = []

        # 1. TX: Inputs (Bitmasks)
        for inp in self.inputs.values():
            w = inp.get("width", inp.get("shape", [1])[0])
            h = inp.get("height", inp.get("shape", [1])[-1])
            designer = IoMatrixDesigner(w, h, is_input=True)
            matrices.append({
                "zone_hash": self.zone_hash,
                "matrix_hash": fnv1a_32(inp["name"].encode('utf-8')),
                "payload_size": designer.bytes_per_tick * batch_size
            })

        # 2. RX: Outputs (Dynamic L7 fragmentation)
        current_offset = 0
        for out in self.outputs.values():
            w = out.get("width", out.get("shape", [1])[0])
            h = out.get("height", out.get("shape", [1])[-1])
            designer = IoMatrixDesigner(w, h, is_input=False)
            chunks = designer.fragment(sync_batch_ticks=batch_size)
            
            for i, chunk in enumerate(chunks):
                chunk_name = out["name"].encode('utf-8') if len(chunks) == 1 else f"{out['name']}_chunk_{i}".encode('utf-8')
                cw = chunk.get("width", chunk.get("shape", [1])[0])
                ch = chunk.get("height", chunk.get("shape", [1])[-1])
                chunk_size = cw * ch * batch_size
                rx_layout.append({
                    "matrix_hash": fnv1a_32(chunk_name),
                    "offset": current_offset,
                    "size": chunk_size
                })
                current_offset += chunk_size

        return {"matrices": matrices, "rx_layout": rx_layout}

    def create_population_encoder(self, name: str, vars_count: int, batch_size: int, sigma: float = 0.2) -> PopulationEncoder:
        inp = self.inputs[name]
        w = inp.get("width", inp.get("shape", [1])[0])
        h = inp.get("height", inp.get("shape", [1])[-1])
        neurons_per_var = (w * h) // vars_count
        return PopulationEncoder(vars_count, neurons_per_var, batch_size, sigma)

    def create_pwm_encoder(self, name: str, batch_size: int) -> PwmEncoder:
        inp = self.inputs[name]
        w = inp.get("width", inp.get("shape", [1])[0])
        h = inp.get("height", inp.get("shape", [1])[-1])
        return PwmEncoder(w * h, batch_size)

    def create_pwm_decoder(self, name: str, batch_size: int) -> PwmDecoder:
        # Direct match (unfragmented output or specific chunk)
        if name in self.outputs:
            out = self.outputs[name]
            w = out.get("width", out.get("shape", [1])[0])
            h = out.get("height", out.get("shape", [1])[-1])
            return PwmDecoder(w * h, batch_size)
        
        # Search for chunks: motor_out -> motor_out_chunk_0, motor_out_chunk_1, ...
        total_neurons = 0
        found = False
        for out_name, out in self.outputs.items():
            if out_name.startswith(name + "_chunk_"):
                w = out.get("width", out.get("shape", [1])[0])
                h = out.get("height", out.get("shape", [1])[-1])
                total_neurons += w * h
                found = True
        
        if not found:
            raise KeyError(f"Output '{name}' not found in contract. Available: {list(self.outputs.keys())}")
        
        return PwmDecoder(total_neurons, batch_size)

    def create_population_decoder(self, name: str, vars_count: int, batch_size: int) -> PopulationDecoder:
        if name in self.outputs:
            out = self.outputs[name]
            w = out.get("width", out.get("shape", [1])[0])
            h = out.get("height", out.get("shape", [1])[-1])
            total_neurons = w * h
        else:
            total_neurons = 0
            found = False
            for out_name, out in self.outputs.items():
                if out_name.startswith(name + "_chunk_"):
                    w = out.get("width", out.get("shape", [1])[0])
                    h = out.get("height", out.get("shape", [1])[-1])
                    total_neurons += w * h
                    found = True
            if not found:
                 raise KeyError(f"Output '{name}' not found in contract. Available: {list(self.outputs.keys())}")

        neurons_per_var = total_neurons // vars_count
        return PopulationDecoder(vars_count, neurons_per_var, batch_size)

    def create_input_facade(self, name: str, buffer: Any) -> Any:
        """
        [DOD] Generates a Zero-Cost facade for a preallocated input buffer.
        Class properties are strictly bound to array indices via closures.
        """
        if name not in self.inputs:
            raise KeyError(f"Input '{name}' not found in contract.")
        
        layout = self.inputs[name].get("layout", [])

        class DynamicInputFacade:
            def __init__(self, buf):
                self.raw_buffer = buf

        # Factory to strictly fix index in the lambda scope
        def make_prop(idx):
            return property(
                lambda self: self.raw_buffer[idx], 
                lambda self, val: self.raw_buffer.__setitem__(idx, val)
            )

        for i, var_name in enumerate(layout):
            if not var_name: 
                continue
            setattr(DynamicInputFacade, var_name, make_prop(i))

        return DynamicInputFacade(buffer)

    def create_output_facade(self, name: str, buffer: Any) -> Any:
        """
        [DOD] Generates a Read-Only facade for the motor output buffer.
        """
        # Consider that the output may be unfragmented or chunk 0
        target_out = None
        if name in self.outputs:
            target_out = self.outputs[name]
        elif f"{name}_chunk_0" in self.outputs:
            target_out = self.outputs[f"{name}_chunk_0"]
            
        if not target_out:
            raise KeyError(f"Output '{name}' not found in contract.")
            
        layout = target_out.get("layout", [])

        class DynamicOutputFacade:
            def __init__(self, buf):
                self.raw_buffer = buf

        # Read-only factory
        def make_prop(idx):
            return property(lambda self: self.raw_buffer[idx])

        for i, var_name in enumerate(layout):
            if not var_name: 
                continue
            setattr(DynamicOutputFacade, var_name, make_prop(i))

        return DynamicOutputFacade(buffer)
