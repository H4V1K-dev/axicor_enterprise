#!/usr/bin/env python3
"""
shards.py — Pure Python functions for shard packing.
"""

import math

def pack_rectangles(rectangles, gap):
    """
    Packs rectangles with a given spacing to fit in a roughly square bounding box (2D shelf packing).
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


def pack_shards_locally(dept_buckets, overrides_shards, gap_shards):
    """
    Packs shards locally within their respective departments.
    Returns: (dept_packings, dept_rects)
    """
    dept_packings = {}
    dept_rects = []

    for dept_name, dept_shards in dept_buckets.items():
        rects = []
        for s in dept_shards:
            s_key = f"{dept_name}.{s.name}" if hasattr(s, 'name') else s.get('key')
            # Extract names depending on whether we got objects or dicts
            s_name = s.name if hasattr(s, 'name') else s.get('shard')
            s_w = s.x if hasattr(s, 'x') else s.get('size', {}).get('w', 32)
            s_d = s.y if hasattr(s, 'y') else s.get('size', {}).get('d', 32)

            shard_override = overrides_shards.get(s_key, {})
            w = shard_override.get("size", {}).get("w", s_w)
            d = shard_override.get("size", {}).get("d", s_d)
            rects.append({"id": s_name, "w": w, "d": d})

        w_dept, d_dept, shard_positions = pack_rectangles(rects, gap_shards)
        dept_packings[dept_name] = {"w": w_dept, "d": d_dept, "positions": shard_positions}
        dept_rects.append({"id": dept_name, "w": w_dept, "d": d_dept})

    return dept_packings, dept_rects
