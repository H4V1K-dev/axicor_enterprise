#!/usr/bin/env python3
"""
departments.py — Pure Python functions for department placement and AABB bounds calculation.
"""

from algorithms.placement.shards import pack_rectangles

def pack_departments_on_level(dept_rects, gap_depts):
    """
    Packs department rectangles relative to each other.
    Returns: positions (dict of dept_name -> (u, v))
    """
    _, _, positions = pack_rectangles(dept_rects, gap_depts)
    return positions


def compute_department_bounds(shards_out):
    """
    Calculates dynamic AABB boundaries for departments based on final shard coordinates.
    Returns: list of dicts representing departments with position and size
    """
    departments_out = []
    resolved_depts = {}

    for s in shards_out:
        lvl_id = s["orbit"]
        dname = s["dept"]
        
        px = s["position"]["x"]
        py = s["position"]["y"]
        sw = s["size"]["w"]
        sd = s["size"]["d"]

        if dname not in resolved_depts:
            resolved_depts[dname] = {
                "name": dname,
                "orbit": lvl_id,
                "x_min": px,
                "x_max": px + sw,
                "y_min": py,
                "y_max": py + sd
            }
        else:
            d_obj = resolved_depts[dname]
            d_obj["x_min"] = min(d_obj["x_min"], px)
            d_obj["x_max"] = max(d_obj["x_max"], px + sw)
            d_obj["y_min"] = min(d_obj["y_min"], py)
            d_obj["y_max"] = max(d_obj["y_max"], py + sd)

    for d in resolved_depts.values():
        departments_out.append({
            "name": d["name"],
            "orbit": d["orbit"],
            "position": {"x": d["x_min"], "y": d["y_min"]},
            "size": {"w": d["x_max"] - d["x_min"], "d": d["y_max"] - d["y_min"]}
        })

    return departments_out
