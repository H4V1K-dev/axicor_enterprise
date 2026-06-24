#!/usr/bin/env python3
"""
levels.py — Pure Python function for stacking levels and shards along the Z-axis.
"""

import copy

def layout_levels_and_shards(levels_list, shards, old_z_starts=None):
    """
    Pure function to stack levels and shards vertically.
    Calculates z_start and height for each level, updating absolute position of shards.
    Preserves local Z height above level floor.
    """
    if old_z_starts is None:
        old_z_starts = {}

    next_levels = copy.deepcopy(levels_list)
    next_shards = copy.deepcopy(shards)

    current_z = 0

    for lvl in next_levels:
        lvl_id = lvl["id"]
        lvl["z_start"] = current_z

        # Find shards belonging to this level
        lvl_shards = [s for s in next_shards if s["orbit"] == lvl_id]

        # Auto-detect old floor if not specified in old_z_starts
        old_floor = old_z_starts.get(lvl_id)
        if old_floor is None:
            if lvl_shards:
                old_floor = min(s["position"]["z"] for s in lvl_shards)
            else:
                old_floor = 0

        max_lvl_h = 10  # Default height if level is empty
        for s in lvl_shards:
            # Calculate local Z height above level floor using the old floor position
            local_z = max(0, s["position"]["z"] - old_floor)
            
            shard_top = local_z + s["size"]["h"]
            if shard_top > max_lvl_h:
                max_lvl_h = shard_top

            # Translate shard to the new absolute Z position
            s["position"]["z"] = lvl["z_start"] + local_z

        lvl["height"] = max_lvl_h
        padding = max(0, int(lvl.get("padding", 0)))
        current_z = lvl["z_start"] + lvl["height"] + padding

    return {"levels": next_levels, "shards": next_shards}
