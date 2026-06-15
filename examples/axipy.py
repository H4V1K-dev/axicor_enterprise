#!/usr/bin/env python3
"""
axipy — Local Mock SDK Stub.

This file provides lightweight mock implementations of the target Axicor Python SDK classes
(ModelBuilder, Shard, Department). This allows IDE linters and local python runs of
scripts (e.g., octopus.py) to succeed without needing the fully installed SDK package.
"""

class _GnmRef:
    def __init__(self, path, *args, **kw):
        self.path = path


class Socket:
    def __init__(self, name, width=1, height=1, *args, **kw):
        self.name = name
        self.width = width
        self.height = height
        self.target = None
        self.targets = []

    def connect_to(self, target, *args, **kw):
        self.target = target
        self.targets.append(target)
        return self


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
    def __init__(self, project_name="", output_dir="", **kw):
        self.project_name = project_name
        self.output_dir = output_dir
        self.departments = []

    def gnm_lib(self, path, *args, **kw):
        return _GnmRef(path, *args, **kw)

    def add_department(self, *depts, **kw):
        self.departments.extend(depts)

    def build(self, dry_run=False, **kw):
        print(f"Mock build completed successfully for project: '{self.project_name}' (dry_run={dry_run})")
        print(f"Mock build completed successfully for project: '{self.project_name}' (dry_run={dry_run})")
