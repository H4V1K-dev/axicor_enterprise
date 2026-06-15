#!/usr/bin/env python3
"""
Model Loader for Axicor Visualizer Server.

Mocks the SDK classes to capture the model topology from octopus.py
without needing the full axipy package.
"""

import os
import sys
import importlib.util

class _GnmRef:
    def __init__(self, path, *args, **kw):
        self.path = path

class Socket:
    def __init__(self, name, width, height, *args, **kw):
        self.name = name
        self.width = width
        self.height = height
        self.target = None
        self.targets = []

    def connect_to(self, target, *args, **kw):
        self.target = target
        self.targets.append(target)


class Shard:
    def __init__(self, name, x=1, y=1, z=1, *args, **kw):
        self.name = name
        self.x = x
        self.y = y
        self.z = z
        self.layers = []
        self.populations = []
        self.input_ports = []
        self.output_ports = []
        self.sockets = {}

    def add_layer(self, name, **kw):
        self.layers.append({
            "name": name,
            "height_pct": kw.get("height_pct", 1.0),
            "density": kw.get("density", 1.0)
        })

    def add_population(self, layer, gnm_ref, **kw):
        self.populations.append((layer, gnm_ref.path if isinstance(gnm_ref, _GnmRef) else str(gnm_ref)))

    def add_input_port(self, name, **kw):
        self.input_ports.append({"name": name, **kw})

    def add_output_port(self, name, **kw):
        self.output_ports.append({"name": name, **kw})

    def add_socket(self, name, width=1, height=1, *args, **kw):
        s = Socket(name, width, height, *args, **kw)
        self.sockets[name] = s
        return s


class DepartmentMeta(type):
    def __getattr__(cls, name):
        # Dynamically support L0, L1, ..., L999 classmethods
        if name.startswith("L") and name[1:].isdigit():
            orbit = int(name[1:])
            return lambda dept_name: cls(dept_name, orbit=orbit)
        raise AttributeError(f"type object '{cls.__name__}' has no attribute '{name}'")


class Department(metaclass=DepartmentMeta):
    def __init__(self, name, orbit=0, *args, **kw):
        self.name = name
        self.orbit = orbit
        self.shards = []

    def add_shard(self, *shards, **kw):
        self.shards.extend(shards)


class ModelBuilder:
    def __init__(self, **kw):
        self.departments = []

    def gnm_lib(self, path, *args, **kw):
        return _GnmRef(path, *args, **kw)

    def add_department(self, *depts, **kw):
        self.departments.extend(depts)

    def build(self, **kw):
        pass


def extract_model(octopus_script_path=None):
    """
    Import and run the octopus build function using our mock classes,
    then extract the topology.
    """
    # Inject our mocks into a fake 'axipy' module
    import types
    axipy_mock = types.ModuleType("axipy")
    axipy_mock.ModelBuilder = ModelBuilder
    axipy_mock.Shard = Shard
    axipy_mock.Department = Department
    sys.modules["axipy"] = axipy_mock

    # Import and run
    if octopus_script_path is None:
        octopus_script_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "examples", "octopus.py"))
    octopus_script_path = os.path.abspath(octopus_script_path)

    if not os.path.exists(octopus_script_path):
        raise FileNotFoundError(f"octopus.py not found at {octopus_script_path}")

    spec = importlib.util.spec_from_file_location("octopus", octopus_script_path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)

    # Capture our mock objects during model builder instantiation and build() call
    _captured_model = [None]
    _orig_build = ModelBuilder.build

    def _capture_build(self, **kw):
        _captured_model[0] = self

    ModelBuilder.build = _capture_build
    
    # Dynamically find and call any function starting with "build_"
    build_funcs = [
        getattr(mod, attr) for attr in dir(mod)
        if attr.startswith("build_") and callable(getattr(mod, attr))
    ]
    
    if build_funcs:
        for func in build_funcs:
            func()
    else:
        # Fallback to the old default
        if hasattr(mod, "build_octopus_brain"):
            mod.build_octopus_brain()
            
    ModelBuilder.build = _orig_build

    model = _captured_model[0]
    if model is None:
        raise RuntimeError("Failed to capture model from octopus.py")

    return model
