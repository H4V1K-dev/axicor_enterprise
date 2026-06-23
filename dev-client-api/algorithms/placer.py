#!/usr/bin/env python3
"""
Orbital Placement Algorithm.

Reads model topology, computes 3D placement of shards on nested spherical
orbits, and outputs JSON for the browser visualizer.
"""

import json
import math
import sys
import os
# Ensure parent directory is in python path when run as standalone script
ROOT_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
if ROOT_DIR not in sys.path:
    sys.path.insert(0, ROOT_DIR)

from server.model_loader import extract_model



# ─── Placement Algorithm ─────────────────────────────────────────────────

def pack_rectangles(rectangles, gap):
    """
    rectangles: list of dicts with 'id', 'w', 'd'
    gap: spacing between rectangles

    Returns: (width_used, depth_used, positions)
      positions: dict of id -> (u, v) representing bottom-left of the rectangle (including spacing)
    """
    if not rectangles:
        return 0, 0, {}

    # Sort by depth (d) descending to pack using a shelf algorithm
    sorted_rects = sorted(rectangles, key=lambda r: r['d'], reverse=True)

    # Calculate target width of the packed area:
    # We want it to be roughly square, so target_w is sqrt(sum_of_areas)
    total_area = sum((r['w'] + gap) * (r['d'] + gap) for r in sorted_rects)
    max_w = max(r['w'] + gap for r in sorted_rects)
    target_w = max(math.ceil(math.sqrt(total_area)), max_w)

    # Pack into shelves
    shelves = []  # each shelf: {"y_start": y, "height": h, "x_cursor": x}
    positions = {}

    for r in sorted_rects:
        rid = r['id']
        rw = r['w'] + gap
        rd = r['d'] + gap

        # Try to place in an existing shelf
        placed = False
        for shelf in shelves:
            if shelf["x_cursor"] + rw <= target_w:
                # Fits in this shelf
                positions[rid] = (shelf["x_cursor"], shelf["y_start"])
                shelf["x_cursor"] += rw
                shelf["height"] = max(shelf["height"], rd)
                placed = True
                break

        if not placed:
            # Create a new shelf
            y_start = 0
            if shelves:
                prev = shelves[-1]
                y_start = prev["y_start"] + prev["height"]

            new_shelf = {
                "y_start": y_start,
                "height": rd,
                "x_cursor": rw
            }
            positions[rid] = (0, y_start)
            shelves.append(new_shelf)

    # Calculate bounding box of packed area
    width_used = max(shelf["x_cursor"] for shelf in shelves) if shelves else 0
    depth_used = sum(shelf["height"] for shelf in shelves) if shelves else 0

    return width_used, depth_used, positions


def matrix_to_quaternion(m):
    """
    Converts a 3x3 rotation matrix to a quaternion.
    m: list of columns, i.e., m[col][row]
    """
    m00, m01, m02 = m[0][0], m[0][1], m[0][2]
    m10, m11, m12 = m[1][0], m[1][1], m[1][2]
    m20, m21, m22 = m[2][0], m[2][1], m[2][2]

    tr = m00 + m11 + m22
    if tr > 0:
        s = math.sqrt(tr + 1.0) * 2
        qw = 0.25 * s
        qx = (m12 - m21) / s
        qy = (m20 - m02) / s
        qz = (m01 - m10) / s
    elif (m00 > m11) and (m00 > m22):
        s = math.sqrt(1.0 + m00 - m11 - m22) * 2
        qw = (m12 - m21) / s
        qx = 0.25 * s
        qy = (m01 + m10) / s
        qz = (m20 + m02) / s
    elif m11 > m22:
        s = math.sqrt(1.0 + m11 - m00 - m22) * 2
        qw = (m20 - m02) / s
        qx = (m01 + m10) / s
        qy = 0.25 * s
        qz = (m12 + m21) / s
    else:
        s = math.sqrt(1.0 + m22 - m00 - m11) * 2
        qw = (m01 - m10) / s
        qx = (m20 + m02) / s
        qy = (m12 + m21) / s
        qz = 0.25 * s

    length = math.sqrt(qx*qx + qy*qy + qz*qz + qw*qw)
    if length > 0:
        return {"x": qx/length, "y": qy/length, "z": qz/length, "w": qw/length}
    return {"x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0}


def compute_placement(model, overrides_path=None):
    """
    Takes a ModelBuilder with departments and computes 3D placement.

    Returns a dict ready to be serialized to JSON.
    """
    # Load overrides if they exist
    overrides = {}
    if overrides_path is None:
        overrides_path = os.path.join(os.path.dirname(__file__), "layout_overrides.json")
    if os.path.exists(overrides_path):
        try:
            with open(overrides_path, "r", encoding="utf-8") as f:
                overrides = json.load(f)
            print(f"Loaded layout overrides from {overrides_path}")
        except Exception as e:
            print(f"Warning: Failed to load layout_overrides.json: {e}")

    # Filter out deleted shards and sockets
    deleted_shards = set(overrides.get("deleted_shards", []))
    deleted_sockets = set(overrides.get("deleted_sockets", []))

    for dept in model.departments:
        dept.shards = [s for s in dept.shards if f"{dept.name}.{s.name}" not in deleted_shards]
        for s in dept.shards:
            s_key = f"{dept.name}.{s.name}"
            s.sockets = {sname: sock for sname, sock in s.sockets.items() if f"{s_key}.{sname}" not in deleted_sockets}

    # Inject custom/new shards saved in overrides but not present in the model script
    existing_keys = set()
    for dept in model.departments:
        for s in dept.shards:
            existing_keys.add(f"{dept.name}.{s.name}")

    overrides_shards = overrides.get("shards", {})
    overrides_levels = overrides.get("levels", [])

    for key, shard_override in overrides_shards.items():
        if key not in existing_keys:
            dept_name = shard_override.get("dept")
            shard_name = shard_override.get("shard")
            orbit = shard_override.get("orbit", 1)

            if not dept_name or not shard_name:
                parts = key.split(".", 1)
                if len(parts) == 2:
                    dept_name, shard_name = parts
                else:
                    continue

            from server.model_loader import Shard, Department
            sw = shard_override.get("size", {}).get("w", 32)
            sd_d = shard_override.get("size", {}).get("d", 32)
            sh = shard_override.get("size", {}).get("h", 16)

            new_s = Shard(shard_name, x=sw, y=sd_d, z=sh)
            new_s.add_layer("default", height_pct=1.0, density=1.0)

            target_dept = None
            for dept in model.departments:
                if dept.name == dept_name:
                    target_dept = dept
                    break

            if not target_dept:
                target_dept = Department(dept_name, orbit=orbit)
                model.departments.append(target_dept)

            target_dept.add_shard(new_s)

    # 1. Resolve level ordering from overrides.levels list
    levels_list = []
    if isinstance(overrides_levels, list):
        levels_list = json.loads(json.dumps(overrides_levels))

    # Find all unique level IDs used by shards
    active_level_ids = set()
    for dept in model.departments:
        active_level_ids.add(dept.orbit)
    for key, shard_override in overrides_shards.items():
        if "orbit" in shard_override:
            active_level_ids.add(int(shard_override["orbit"]))

    # Ensure all active levels are registered in levels_list
    for lvl_id in active_level_ids:
        if not any(l.get("id") == lvl_id for l in levels_list):
            default_name = f"Level {lvl_id}"
            for dept in model.departments:
                if dept.orbit == lvl_id:
                    default_name = dept.name
                    break
            colors = ["#34d399", "#38bdf8", "#f472b6"]
            levels_list.append({
                "id": lvl_id,
                "name": default_name,
                "color": colors[len(levels_list) % len(colors)]
            })

    # 2. Calculate dynamic heights and z_start for levels in order
    levels_map = {}
    current_z = 0
    gap_between_levels = 20

    for lvl in levels_list:
        lvl_id = lvl["id"]
        lvl["z_start"] = current_z

        # Gather shards on this level to compute height
        max_lvl_h = 40
        for dept in model.departments:
            for s in dept.shards:
                key = f"{dept.name}.{s.name}"
                shard_override = overrides_shards.get(key, {})
                orbit = shard_override.get("orbit", dept.orbit)
                if orbit == lvl_id:
                    h = shard_override.get("size", {}).get("h", s.z)
                    if "position" in shard_override and "z" in shard_override["position"]:
                        local_z = shard_override["position"]["z"] - lvl["z_start"]
                        if local_z + h > max_lvl_h:
                            max_lvl_h = local_z + h
                    else:
                        if h > max_lvl_h:
                            max_lvl_h = h
        
        lvl["height"] = max_lvl_h
        levels_map[lvl_id] = lvl
        current_z = lvl["z_start"] + lvl["height"] + gap_between_levels

    # 3. Perform default packing for shards and departments if no overrides exist
    gap_shards = 0
    gap_depts = 1
    default_positions = {}

    for lvl in levels_list:
        lvl_id = lvl["id"]
        depts_on_level = [d for d in model.departments if d.orbit == lvl_id]
        if not depts_on_level:
            continue

        dept_rects = []
        dept_packings = {}

        # Step A: Pack shards within each department locally
        for dept in depts_on_level:
            shards_list = []
            for s in dept.shards:
                key = f"{dept.name}.{s.name}"
                shard_override = overrides_shards.get(key, {})
                sw = shard_override.get("size", {}).get("w", s.x)
                sd_d = shard_override.get("size", {}).get("d", s.y)
                shards_list.append({"id": s.name, "w": sw, "d": sd_d})

            w_dept, d_dept, shard_positions = pack_rectangles(shards_list, gap_shards)
            dept_packings[dept.name] = {"w": w_dept, "d": d_dept, "positions": shard_positions}
            dept_rects.append({"id": dept.name, "w": w_dept, "d": d_dept})

        # Step B: Pack departments within this level
        w_orbit, d_orbit, dept_positions = pack_rectangles(dept_rects, gap_depts)

        # Step C: Assign default packed coordinates relative to level origin
        for dept in depts_on_level:
            du, dv = dept_positions.get(dept.name, (0, 0))
            shard_pos = dept_packings[dept.name]["positions"]

            for s in dept.shards:
                key = f"{dept.name}.{s.name}"
                su, sv = shard_pos.get(s.name, (0, 0))
                default_positions[key] = {
                    "x": du + su,
                    "y": dv + sv,
                    "z": levels_map[lvl_id]["z_start"]
                }

    # 4. Construct final shard list with absolute positions
    shards_out = []
    for dept in model.departments:
        for s in dept.shards:
            key = f"{dept.name}.{s.name}"
            shard_override = overrides_shards.get(key, {})
            orbit = shard_override.get("orbit", dept.orbit)
            level = levels_map.get(orbit)

            px, py, pz = 0, 0, 0
            def_pos = default_positions.get(key, {"x": 0, "y": 0, "z": level["z_start"] if level else 0})

            if "position" in shard_override:
                px = int(round(shard_override["position"]["x"]))
                py = int(round(shard_override["position"]["y"]))
                pz = int(round(shard_override["position"]["z"]))
            else:
                px = def_pos["x"]
                py = def_pos["y"]
                pz = def_pos["z"]

            w = shard_override.get("size", {}).get("w", s.x)
            d = shard_override.get("size", {}).get("d", s.y)
            h = shard_override.get("size", {}).get("h", s.z)

            # Layers metadata
            layers_data = []
            overridden_layers = shard_override.get("layer_proportions", {})
            layer_order = shard_override.get("layer_order", [])
            
            raw_layers = s.layers
            if layer_order:
                layer_map = {l["name"]: l for l in raw_layers}
                sorted_layers = []
                for name in layer_order:
                    if name in layer_map:
                        sorted_layers.append(layer_map[name])
                for l in raw_layers:
                    if l["name"] not in layer_order:
                        sorted_layers.append(l)
                raw_layers = sorted_layers

            for l in raw_layers:
                lname = l["name"]
                lpct = overridden_layers.get(lname, l["height_pct"])
                layers_data.append({
                    "name": lname,
                    "height_pct": lpct,
                    "density": l.get("density", 1.0)
                })

            shards_out.append({
                "key": key,
                "dept": dept.name,
                "shard": s.name,
                "orbit": orbit,
                "position": {"x": px, "y": py, "z": pz},
                "size": {"w": w, "d": d, "h": h},
                "layers": layers_data
            })

    # 5. Calculate actual dynamic heights for levels and AABB bounds for departments
    departments_out = []
    resolved_depts = {} # dept_name -> { orbit, x_min, x_max, y_min, y_max }

    for s in shards_out:
        lvl_id = s["orbit"]

        # Accumulate AABB for department
        dname = s["dept"]
        if dname not in resolved_depts:
            resolved_depts[dname] = {
                "name": dname,
                "orbit": lvl_id,
                "x_min": s["position"]["x"],
                "x_max": s["position"]["x"] + s["size"]["w"],
                "y_min": s["position"]["y"],
                "y_max": s["position"]["y"] + s["size"]["d"]
            }
        else:
            d_obj = resolved_depts[dname]
            d_obj["x_min"] = min(d_obj["x_min"], s["position"]["x"])
            d_obj["x_max"] = max(d_obj["x_max"], s["position"]["x"] + s["size"]["w"])
            d_obj["y_min"] = min(d_obj["y_min"], s["position"]["y"])
            d_obj["y_max"] = max(d_obj["y_max"], s["position"]["y"] + s["size"]["d"])

    # Convert resolved departments to output structure
    for d in resolved_depts.values():
        departments_out.append({
            "name": d["name"],
            "orbit": d["orbit"],
            "position": {"x": d["x_min"], "y": d["y_min"]},
            "size": {"w": d["x_max"] - d["x_min"], "d": d["y_max"] - d["y_min"]}
        })

    return {
        "levels": levels_list,
        "departments": departments_out,
        "shards": shards_out,
        "connections": [],
        "seed": overrides.get("seed", 42),
        "simulation": overrides.get("simulation", {}),
        "world": overrides.get("world", {})
    }




# ─── Main ────────────────────────────────────────────────────────────────

def main():
    import argparse
    parser = argparse.ArgumentParser(description="Orbital Placement Algorithm")
    parser.add_argument("script", nargs="?", default=None, help="Path to octopus script")
    parser.add_argument("overrides", nargs="?", default=None, help="Path to layout overrides JSON")
    parser.add_argument("output", nargs="?", default=None, help="Path to output placement JSON")
    args = parser.parse_args()

    # Determine script path
    script_path = args.script
    if script_path is None:
        script_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "examples", "octopus.py"))
    script_path = os.path.abspath(script_path)

    # Determine overrides path
    overrides_path = args.overrides
    if overrides_path is None:
        overrides_path = os.path.join(os.path.dirname(__file__), "layout_overrides.json")
    overrides_path = os.path.abspath(overrides_path)

    # Determine output path
    output_path = args.output
    if output_path is None:
        output_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "dev-js-api", "placement.json"))
    output_path = os.path.abspath(output_path)

    print(f"Extracting model from {script_path}...")
    model = extract_model(script_path)

    print(f"  Departments: {len(model.departments)}")
    total_shards = sum(len(d.shards) for d in model.departments)
    print(f"  Shards: {total_shards}")

    print("Computing orbital placement...")
    result = compute_placement(model, overrides_path)

    print(f"  Levels: {len(result['levels'])}")
    for lvl in result['levels']:
      print(f"    L{lvl['id']}: name={lvl['name']}, z_start={lvl['z_start']}, height={lvl['height']}")
    print(f"  Connections: {len(result['connections'])}")

    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as f:
      json.dump(result, f, indent=2)

    print(f"\nDone! Written to {output_path}")


if __name__ == "__main__":
    main()
