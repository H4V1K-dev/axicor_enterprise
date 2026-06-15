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

    # 0. Filter out deleted shards and sockets
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

    for key, shard_override in overrides.get("shards", {}).items():
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

            # Reconstruct layers from overrides
            layers_data = shard_override.get("layers", [])
            if layers_data:
                for l in layers_data:
                    new_s.add_layer(l["name"], height_pct=l["height_pct"], density=l.get("density", 1.0))
            else:
                new_s.add_layer("default", height_pct=1.0, density=1.0)

            # Reconstruct sockets from overrides
            sockets_data = shard_override.get("sockets", [])
            for sock in sockets_data:
                new_s.add_socket(sock["name"], width=sock.get("width", 1), height=sock.get("height", 1))

            # Find or create department mock
            target_dept = None
            for dept in model.departments:
                if dept.name == dept_name:
                    target_dept = dept
                    break

            if not target_dept:
                target_dept = Department(dept_name, orbit=orbit)
                model.departments.append(target_dept)

            target_dept.add_shard(new_s)

    # 1. Group departments by orbit
    orbit_buckets = {}  # orbit_idx -> [dept, ...]
    for dept in model.departments:
        orbit_buckets.setdefault(dept.orbit, []).append(dept)

    sorted_orbits = sorted(orbit_buckets.keys())

    # 2. Pack each orbit flat (shards inside dept, then depts inside orbit)
    packed_orbits = {}  # orbit_idx -> {w, d, shard_positions}
    orbit_max_h = {}    # orbit_idx -> max shard thickness (height)

    gap_shards = 0
    gap_depts = 1

    for orbit_idx in sorted_orbits:
        depts = orbit_buckets[orbit_idx]
        
        # Pack shards of each department
        dept_rects = []
        dept_packings = {}  # dept_name -> (w, d, shard_positions)
        max_h = 0

        for dept in depts:
            shards_list = []
            for s in dept.shards:
                key = f"{dept.name}.{s.name}"
                shard_override = overrides.get("shards", {}).get(key, {})
                sw = shard_override.get("size", {}).get("w", s.x)
                sd_d = shard_override.get("size", {}).get("d", s.y)
                shards_list.append({"id": s.name, "w": sw, "d": sd_d})

            w_dept, d_dept, shard_positions = pack_rectangles(shards_list, gap_shards)
            dept_packings[dept.name] = (w_dept, d_dept, shard_positions)
            
            # Track max thickness (height, z in octopus.py)
            for s in dept.shards:
                key = f"{dept.name}.{s.name}"
                shard_override = overrides.get("shards", {}).get(key, {})
                sh = shard_override.get("size", {}).get("h", s.z)
                max_h = max(max_h, sh)

            dept_rects.append({"id": dept.name, "w": w_dept, "d": d_dept})

        # Pack departments inside this orbit
        w_orbit, d_orbit, dept_positions = pack_rectangles(dept_rects, gap_depts)
        
        # Combine flat positions for shards in this orbit
        orbit_shards = {}
        for dept in depts:
            w_dept, d_dept, shard_pos = dept_packings[dept.name]
            du, dv = dept_positions[dept.name]
            for sname, (su, sv) in shard_pos.items():
                # Store absolute center of the shard and its size
                shard_obj = next(s for s in dept.shards if s.name == sname)
                key = f"{dept.name}.{sname}"
                shard_override = overrides.get("shards", {}).get(key, {})
                sw = shard_override.get("size", {}).get("w", shard_obj.x)
                sd_d = shard_override.get("size", {}).get("d", shard_obj.y)
                sh = shard_override.get("size", {}).get("h", shard_obj.z)

                orbit_shards[key] = {
                    "dept": dept.name,
                    "shard": sname,
                    "u": du + su,
                    "v": dv + sv,
                    "w": sw,
                    "d": sd_d,
                    "h": sh,
                    "raw_shard": shard_obj
                }

        packed_orbits[orbit_idx] = {
            "w": w_orbit,
            "d": d_orbit,
            "shards": orbit_shards
        }
        orbit_max_h[orbit_idx] = max_h

    # 3. Compute layer heights dynamically stacked along Y
    heights = {}
    for i, orb in enumerate(sorted_orbits):
        if i == 0:
            heights[orb] = 0.0
        else:
            prev_orb = sorted_orbits[i - 1]
            prev_max_h = orbit_max_h[prev_orb]
            
            # Floor of current layer = floor of prev + max thickness of prev
            heights[orb] = heights[prev_orb] + prev_max_h

    # 4. Lay out coordinates flat per layer and compute orientations
    shards_out = []
    departments_out = []
    
    # Store position dictionary for connections
    shard_positions_3d = {}
    
    # Store overridden socket dimensions for connection lookup
    overridden_sockets = {} # "shard_key.socket_name" -> (w, h)

    for orbit_idx in sorted_orbits:
        packed = packed_orbits[orbit_idx]
        w_orb, d_orb = packed["w"], packed["d"]
        radius = heights[orbit_idx]  # We keep 'radius' variable name to map to height

        # Register departments for this orbit
        for dept in orbit_buckets[orbit_idx]:
            departments_out.append({
                "name": dept.name,
                "orbit": orbit_idx,
                "shard_count": len(dept.shards),
            })

        for key, sd in packed["shards"].items():
            # Get center of the shard in flat coordinate space
            u_c = sd["u"] + sd["w"] / 2.0
            v_c = sd["v"] + sd["d"] / 2.0

            # Flat layered layout mapping (stacked along Y)
            px = u_c - w_orb / 2.0
            py = radius + sd["h"] / 2.0  # Shift up by half of the thickness so the bottom lies on the layer grid
            pz = v_c - d_orb / 2.0

            # Apply manual position overrides
            shard_override = overrides.get("shards", {}).get(key, {})
            if "position" in shard_override:
                px = shard_override["position"]["x"]
                py = radius + shard_override["position"]["y"]
                pz = shard_override["position"]["z"]

            # Store for connection routing
            shard_positions_3d[key] = {"x": px, "y": py, "z": pz}

            # Generate orientation quaternion:
            # We want local Z (thickness h) to align with world Y (up).
            # local X (width w) -> world X
            # local Y (depth d) -> world -Z
            # This is a 90 degree rotation around X axis
            rot_matrix = [
                [1.0, 0.0, 0.0],
                [0.0, 0.0, -1.0],
                [0.0, 1.0, 0.0]
            ]
            quat = matrix_to_quaternion(rot_matrix)

            # Sockets metadata
            raw_shard = sd["raw_shard"]
            sockets_data = []
            for sname, sock in raw_shard.sockets.items():
                sock_key = f"{key}.{sname}"
                sock_override = overrides.get("sockets", {}).get(sock_key, {})
                
                # Retrieve overridden values
                sw = sock_override.get("width", sock.width)
                sh = sock_override.get("height", sock.height)
                pitch = sock_override.get("pitch", 1)
                offset = sock_override.get("offset", None)
                rotation = sock_override.get("rotation", 0)
                face_sign = sock_override.get("faceSign", None)
                
                overridden_sockets[sock_key] = (sw, sh)

                sockets_data.append({
                    "name": sname,
                    "width": sw,
                    "height": sh,
                    "pitch": pitch,
                    "offset": offset,
                    "rotation": rotation,
                    "faceSign": face_sign
                })

            # Layers metadata
            layers_data = []
            shard_override = overrides.get("shards", {}).get(key, {})
            overridden_layers = shard_override.get("layer_proportions", {})
            layer_order = shard_override.get("layer_order", [])
            
            raw_layers = raw_shard.layers
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
                    "density": l["density"]
                })

            shards_out.append({
                "key": key,
                "dept": sd["dept"],
                "shard": sd["shard"],
                "orbit": orbit_idx,
                "radius": round(radius, 2),
                "position": {"x": round(px, 2), "y": round(py, 2), "z": round(pz, 2)},
                # Raw voxel dimensions: w (width, X), d (depth, Y), h (height, Z)
                "size": {"w": sd["w"], "d": sd["d"], "h": sd["h"]},
                "flat_position": {"u": sd["u"], "v": sd["v"]},
                "quaternion": quat,
                "sockets": sockets_data,
                "input_ports": raw_shard.input_ports,
                "output_ports": raw_shard.output_ports,
                "layers": layers_data,
                "populations": raw_shard.populations,
            })

    # 5. Extract connections
    connections_out = []
    for dept in model.departments:
        for shard in dept.shards:
            for sname, sock in shard.sockets.items():
                targets = sock.targets if hasattr(sock, "targets") and sock.targets else ([sock.target] if sock.target is not None else [])
                for t in targets:
                    if t is None:
                        continue

                    from_key = f"{dept.name}.{shard.name}"
                    parts = t.split(".")
                    if len(parts) == 2:
                        target_shard_name, target_socket = parts
                        to_key = f"{dept.name}.{target_shard_name}"
                    elif len(parts) == 3:
                        target_dept, target_shard_name, target_socket = parts
                        to_key = f"{target_dept}.{target_shard_name}"
                    else:
                        continue

                    from_sock_key = f"{from_key}.{sname}"
                    to_sock_key = f"{to_key}.{target_socket}"
                    
                    # Filter out deleted connections
                    deleted_conns = set(overrides.get("deleted_connections", []))
                    conn_key = f"{from_sock_key} -> {to_sock_key}"
                    conn_key_rev = f"{to_sock_key} -> {from_sock_key}"
                    if conn_key in deleted_conns or conn_key_rev in deleted_conns:
                        continue
                    
                    # Fetch socket dimensions (prefer overridden values)
                    matrix_w = overridden_sockets.get(from_sock_key, (sock.width, sock.height))[0]
                    matrix_h = overridden_sockets.get(from_sock_key, (sock.width, sock.height))[1]

                    # Check if this connection has an override
                    matched_override = None
                    for conn_override in overrides.get("connections", []):
                        o_from = conn_override.get("from")
                        o_to = conn_override.get("to")
                        o_from_sock = conn_override.get("from_socket")
                        o_to_sock = conn_override.get("to_socket")
                        if ((o_from == from_key and o_from_sock == sname and o_to == to_key and o_to_sock == target_socket) or
                            (o_from == to_key and o_from_sock == target_socket and o_to == from_key and o_to_sock == sname)):
                            matched_override = conn_override
                            break

                    conn_obj = {
                        "from": from_key,
                        "to": to_key,
                        "from_socket": sname,
                        "to_socket": target_socket,
                        "matrix_w": matrix_w,
                        "matrix_h": matrix_h,
                    }
                    if matched_override:
                        if "manual" in matched_override:
                            conn_obj["manual"] = matched_override["manual"]
                        if "control_points" in matched_override:
                            conn_obj["control_points"] = matched_override["control_points"]
                    
                    connections_out.append(conn_obj)

    # Append any manual/custom connections from overrides that were not in the model
    for conn_override in overrides.get("connections", []):
        o_from = conn_override.get("from")
        o_to = conn_override.get("to")
        o_from_sock = conn_override.get("from_socket")
        o_to_sock = conn_override.get("to_socket")
        
        # Check if already added
        already_added = False
        for c in connections_out:
            if ((c["from"] == o_from and c["from_socket"] == o_from_sock and c["to"] == o_to and c["to_socket"] == o_to_sock) or
                (c["from"] == o_to and c["from_socket"] == o_to_sock and c["to"] == o_from and c["to_socket"] == o_from_sock)):
                already_added = True
                break
        
        if not already_added:
            # Check if this connection was deleted
            deleted_conns = set(overrides.get("deleted_connections", []))
            conn_key = f"{o_from}.{o_from_sock} -> {o_to}.{o_to_sock}"
            conn_key_rev = f"{o_to}.{o_to_sock} -> {o_from}.{o_from_sock}"
            if conn_key in deleted_conns or conn_key_rev in deleted_conns:
                continue
                
            # Add to output
            conn_obj = {
                "from": o_from,
                "to": o_to,
                "from_socket": o_from_sock,
                "to_socket": o_to_sock,
                "matrix_w": conn_override.get("matrix_w", 1),
                "matrix_h": conn_override.get("matrix_h", 1),
            }
            if "manual" in conn_override:
                conn_obj["manual"] = conn_override["manual"]
            if "control_points" in conn_override:
                conn_obj["control_points"] = conn_override["control_points"]
            connections_out.append(conn_obj)

    # 6. Build orbit metadata
    orbits_out = []
    for orb in sorted_orbits:
        orbits_out.append({
            "index": orb,
            "radius": round(heights[orb], 2),
            "w": packed_orbits[orb]["w"],
            "d": packed_orbits[orb]["d"],
            "area": round(packed_orbits[orb]["w"] * packed_orbits[orb]["d"], 1),
            "dept_count": len(orbit_buckets[orb]),
        })

    return {
        "orbits": orbits_out,
        "departments": departments_out,
        "shards": shards_out,
        "connections": connections_out,
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

    print(f"  Orbits: {len(result['orbits'])}")
    for o in result['orbits']:
      print(f"    L{o['index']}: height={o['radius']:.2f}, area={o['area']:.0f}, depts={o['dept_count']}")
    print(f"  Connections: {len(result['connections'])}")

    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as f:
      json.dump(result, f, indent=2)

    print(f"\nDone! Written to {output_path}")


if __name__ == "__main__":
    main()
