#!/usr/bin/env python3
"""
Phase 2: Route Curves

Reads shard physical positions and connections from placement.json,
computes 3D Bezier curves between the connection endpoints,
and outputs routes.json for visualizer and subsequent compiler phases.
"""

import json
import math
import os

def compute_bezier_points(p0, p3, num_points=48):
    """
    Computes points along a cubic Bezier curve.
    p0: start point (x, y, z)
    p3: end point (x, y, z)
    """
    # Calculate midpoint
    mid_x = (p0[0] + p3[0]) / 2.0
    mid_y = (p0[1] + p3[1]) / 2.0
    mid_z = (p0[2] + p3[2]) / 2.0

    # Bulge height based on distance
    dx = p3[0] - p0[0]
    dy = p3[1] - p0[1]
    dz = p3[2] - p0[2]
    dist = math.sqrt(dx*dx + dy*dy + dz*dz)
    mid_y += dist * 0.3

    # Control points
    cp1_x = p0[0] + (mid_x - p0[0]) * 0.5
    cp1_y = mid_y * 0.8
    cp1_z = p0[2] + (mid_z - p0[2]) * 0.5

    cp2_x = p3[0] + (mid_x - p3[0]) * 0.5
    cp2_y = mid_y * 0.8
    cp2_z = p3[2] + (mid_z - p3[2]) * 0.5

    points = []
    for i in range(num_points):
        t = i / (num_points - 1)
        # Cubic Bezier formula
        mt = 1.0 - t
        x = mt**3 * p0[0] + 3 * mt**2 * t * cp1_x + 3 * mt * t**2 * cp2_x + t**3 * p3[0]
        y = mt**3 * p0[1] + 3 * mt**2 * t * cp1_y + 3 * mt * t**2 * cp2_y + t**3 * p3[1]
        z = mt**3 * p0[2] + 3 * mt**2 * t * cp1_z + 3 * mt * t**2 * cp2_z + t**3 * p3[2]
        
        points.append([round(x, 2), round(y, 2), round(z, 2)])
    
    return points

def main():
    import argparse
    parser = argparse.ArgumentParser(description="Route Curves")
    parser.add_argument("placement", nargs="?", default=None, help="Path to placement JSON")
    parser.add_argument("routes", nargs="?", default=None, help="Path to output routes JSON")
    args = parser.parse_args()

    # Determine paths
    script_dir = os.path.dirname(os.path.abspath(__file__))
    placement_path = args.placement
    if placement_path is None:
        placement_path = os.path.join(script_dir, "..", "..", "dev-js-api", "placement.json")
    placement_path = os.path.abspath(placement_path)

    routes_path = args.routes
    if routes_path is None:
        routes_path = os.path.join(script_dir, "..", "..", "dev-js-api", "routes.json")
    routes_path = os.path.abspath(routes_path)

    print(f"Loading placement from {placement_path}...")
    if not os.path.exists(placement_path):
        print(f"Error: {placement_path} not found. Run placer.py first.")
        return

    with open(placement_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    # Index shards by key for easy position lookup
    shards_by_key = {s["key"]: s for s in data["shards"]}

    routes = []
    print(f"Routing {len(data['connections'])} connections...")
    
    for conn in data["connections"]:
        from_key = conn["from"]
        to_key = conn["to"]
        
        if from_key not in shards_by_key or to_key not in shards_by_key:
            continue
            
        from_shard = shards_by_key[from_key]
        to_shard = shards_by_key[to_key]
        
        p0 = [from_shard["position"]["x"], from_shard["position"]["y"], from_shard["position"]["z"]]
        p3 = [to_shard["position"]["x"], to_shard["position"]["y"], to_shard["position"]["z"]]
        
        # In a real routing implementation, we would route from the specific sockets.
        # For now, we compute the Bezier curve between shard centers.
        curve_points = compute_bezier_points(p0, p3)
        
        routes.append({
            "from": from_key,
            "to": to_key,
            "from_socket": conn["from_socket"],
            "to_socket": conn["to_socket"],
            "matrix_w": conn["matrix_w"],
            "matrix_h": conn["matrix_h"],
            "points": curve_points
        })

    os.makedirs(os.path.dirname(routes_path), exist_ok=True)
    with open(routes_path, "w", encoding="utf-8") as f:
        json.dump(routes, f, indent=2)

    print(f"Successfully computed routes! Output saved to {routes_path}")

if __name__ == "__main__":
    main()
