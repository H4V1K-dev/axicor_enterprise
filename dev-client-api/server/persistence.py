#!/usr/bin/env python3
"""
Persistence Helper for Axicor Visualizer.

Manages loading, merging, and saving layout overrides (json),
and handles compiling script projects into local JSON coordinates format.
"""

import json
import os
import sys

# Ensure root directory is in python path
ROOT_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
if ROOT_DIR not in sys.path:
    sys.path.insert(0, ROOT_DIR)

SCRIPTS_DIR = os.path.abspath(os.path.join(ROOT_DIR, "..", "examples"))
MODELS_DIR = os.path.abspath(os.path.join(ROOT_DIR, "..", "models"))
LOCAL_DIR = os.path.abspath(os.path.join(ROOT_DIR, "..", ".local-storage"))

# Ensure the directories exist
os.makedirs(SCRIPTS_DIR, exist_ok=True)
os.makedirs(MODELS_DIR, exist_ok=True)
os.makedirs(LOCAL_DIR, exist_ok=True)

def is_project_script(file_path):
    """Statically checks if a python file is a project recipe by looking for SDK signature."""
    try:
        with open(file_path, "r", encoding="utf-8", errors="ignore") as f:
            content = f.read()
        if "ModelBuilder" not in content:
            return False
        import re
        if not re.search(r"model\s*=\s*ModelBuilder", content):
            return False
        if "model.build" not in content:
            return False
        return True
    except Exception:
        return False

def list_projects():
    """Lists scripts, models and local projects from workspace directories."""
    os.makedirs(SCRIPTS_DIR, exist_ok=True)
    os.makedirs(MODELS_DIR, exist_ok=True)
    os.makedirs(LOCAL_DIR, exist_ok=True)

    all_files = [f for f in os.listdir(SCRIPTS_DIR) if f.endswith(".py")]
    scripts = [f for f in all_files if is_project_script(os.path.join(SCRIPTS_DIR, f))]
    models = [f for f in os.listdir(MODELS_DIR) if os.path.isdir(os.path.join(MODELS_DIR, f))]
    
    local_projects = []
    for name in os.listdir(LOCAL_DIR):
        proj_dir = os.path.join(LOCAL_DIR, name)
        if os.path.isdir(proj_dir):
            overrides_path = os.path.join(proj_dir, "layout_overrides.json")
            target_path = overrides_path if os.path.exists(overrides_path) else proj_dir
            mtime = os.path.getmtime(target_path)
            
            import datetime
            dt = datetime.datetime.fromtimestamp(mtime)
            formatted_time = dt.strftime("%Y-%m-%d %H:%M")
            
            has_preview = os.path.exists(os.path.join(proj_dir, "preview.png"))
            
            local_projects.append({
                "name": name,
                "mtime": mtime,
                "formatted_time": formatted_time,
                "has_preview": has_preview
            })
            
    # Sort local projects descending by mtime (newest first)
    local_projects.sort(key=lambda x: x["mtime"], reverse=True)

    return {
        "scripts": sorted(scripts),
        "models": sorted(models),
        "local": local_projects
    }

def compile_project(project_name, script_name):
    """
    Compiles a python script project into local placement and routes data,
    running placement and routing algorithms programmatically.
    """
    script_path = os.path.join(SCRIPTS_DIR, script_name)
    local_proj_dir = os.path.join(LOCAL_DIR, project_name)
    os.makedirs(local_proj_dir, exist_ok=True)

    overrides_path = os.path.join(local_proj_dir, "layout_overrides.json")
    placement_path = os.path.join(local_proj_dir, "placement.json")
    routes_path = os.path.join(local_proj_dir, "routes.json")

    # 1. Run placement
    from algorithms import placer
    print(f"Compiling project layout: {project_name} from {script_path}")
    model = placer.extract_model(script_path)
    result = placer.compute_placement(model, overrides_path)
    with open(placement_path, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=2)

    # 2. Run router
    from algorithms import router
    print(f"Compiling project routes: {project_name}")
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
            
        if conn.get("manual"):
            routes.append({
                "from": from_key,
                "to": to_key,
                "from_socket": conn["from_socket"],
                "to_socket": conn["to_socket"],
                "matrix_w": conn["matrix_w"],
                "matrix_h": conn["matrix_h"],
                "manual": True,
                "control_points": conn.get("control_points", []),
                "points": conn.get("points", [])
            })
            continue
            
        from_shard = shards_by_key[from_key]
        to_shard = shards_by_key[to_key]
        
        p0 = [from_shard["position"]["x"], from_shard["position"]["y"], from_shard["position"]["z"]]
        p3 = [to_shard["position"]["x"], to_shard["position"]["y"], to_shard["position"]["z"]]
        
        curve_points = router.compute_bezier_points(p0, p3)
        
        routes.append({
            "from": from_key,
            "to": to_key,
            "from_socket": conn["from_socket"],
            "to_socket": conn["to_socket"],
            "matrix_w": conn["matrix_w"],
            "matrix_h": conn["matrix_h"],
            "points": curve_points
        })

    with open(routes_path, "w", encoding="utf-8") as f:
        json.dump(routes, f, indent=2)

    print(f"Project compilation done: {project_name}")

def load_project_overrides(project_name):
    """Load layout overrides for a specific local project."""
    local_proj_dir = os.path.join(LOCAL_DIR, project_name)
    overrides_path = os.path.join(local_proj_dir, "layout_overrides.json")
    if os.path.exists(overrides_path):
        try:
            with open(overrides_path, "r", encoding="utf-8") as f:
                return json.load(f)
        except Exception as e:
            print(f"Error reading layout_overrides.json for project {project_name}: {e}")
    return {}

def save_project_overrides(project_name, overrides):
    """Save layout overrides for a specific local project."""
    local_proj_dir = os.path.join(LOCAL_DIR, project_name)
    overrides_path = os.path.join(local_proj_dir, "layout_overrides.json")
    os.makedirs(local_proj_dir, exist_ok=True)
    with open(overrides_path, "w", encoding="utf-8") as f:
        json.dump(overrides, f, indent=2)
    print(f"Saved layout overrides for project {project_name} to {overrides_path}")

def update_and_regenerate(payload):
    """
    Merge the new layout overrides payload and regenerate placement.json
    and routes.json for the specific project.
    """
    project_name = payload.get("project", "octopus")
    overrides = load_project_overrides(project_name)

    # Merge shards and sockets overrides
    if "shards" not in overrides:
        overrides["shards"] = {}
    if "sockets" not in overrides:
        overrides["sockets"] = {}

    # Merge deleted trackers
    if "deleted_shards" not in overrides:
        overrides["deleted_shards"] = []
    for key in payload.get("deleted_shards", []):
        if key not in overrides["deleted_shards"]:
            overrides["deleted_shards"].append(key)

    # If a shard is present in payload.shards, it's not deleted
    new_shards = payload.get("shards", {})
    for key in new_shards.keys():
        if key in overrides["deleted_shards"]:
            overrides["deleted_shards"].remove(key)

    # Remove any shards from overrides that are not in new_shards
    overrides_shard_keys = list(overrides["shards"].keys())
    for key in overrides_shard_keys:
        if key not in new_shards:
            del overrides["shards"][key]

    # Update shards
    for key, shard_data in new_shards.items():
        if key not in overrides["shards"]:
            overrides["shards"][key] = {}
        # Update position
        if "position" in shard_data:
            overrides["shards"][key]["position"] = shard_data["position"]
        # Update size
        if "size" in shard_data:
            overrides["shards"][key]["size"] = shard_data["size"]
        # Update layer proportions
        if "layer_proportions" in shard_data:
            overrides["shards"][key]["layer_proportions"] = shard_data["layer_proportions"]
        # Update layer order
        if "layer_order" in shard_data:
            overrides["shards"][key]["layer_order"] = shard_data["layer_order"]
        # Save custom metadata fields for added shards
        for field in ["orbit", "dept", "shard", "layers", "sockets"]:
            if field in shard_data:
                overrides["shards"][key][field] = shard_data[field]

    # Merge deleted connections trackers
    if "deleted_connections" not in overrides:
        overrides["deleted_connections"] = []
    for conn_str in payload.get("deleted_connections", []):
        if conn_str not in overrides["deleted_connections"]:
            overrides["deleted_connections"].append(conn_str)

    # Merge deleted sockets trackers
    if "deleted_sockets" not in overrides:
        overrides["deleted_sockets"] = []
    for key in payload.get("deleted_sockets", []):
        if key not in overrides["deleted_sockets"]:
            overrides["deleted_sockets"].append(key)

    # Update sockets
    new_sockets = payload.get("sockets", {})
    # If a socket is present in payload.sockets, it's not deleted
    for key in new_sockets.keys():
        if key in overrides["deleted_sockets"]:
            overrides["deleted_sockets"].remove(key)

    # Remove any sockets from overrides that are not in new_sockets
    overrides_socket_keys = list(overrides["sockets"].keys())
    for key in overrides_socket_keys:
        if key not in new_sockets:
            del overrides["sockets"][key]

    for key, socket_data in new_sockets.items():
        if key not in overrides["sockets"]:
            overrides["sockets"][key] = {}
        
        # Update properties
        for prop in ["width", "height", "pitch", "offset", "rotation", "faceSign"]:
            if prop in socket_data:
                overrides["sockets"][key][prop] = socket_data[prop]

    # Save simulation and world settings from settings panel
    if "simulation" in payload:
        overrides["simulation"] = payload["simulation"]
    if "world" in payload:
        overrides["world"] = payload["world"]

    # Save connection overrides
    if "connections" in payload:
        overrides["connections"] = payload["connections"]

    # Save to disk
    save_project_overrides(project_name, overrides)

    # Save history cache if present
    if "history" in payload:
        local_proj_dir = os.path.join(LOCAL_DIR, project_name)
        history_path = os.path.join(local_proj_dir, "history_cache.json")
        try:
            with open(history_path, "w", encoding="utf-8") as f:
                json.dump(payload["history"], f, indent=2)
            print(f"Saved history cache for project {project_name} to {history_path}")
        except Exception as e:
            print(f"Error saving history cache for project {project_name}: {e}")

    # Save preview image if provided
    preview_data = payload.get("preview")
    if preview_data and preview_data.startswith("data:image/png;base64,"):
        try:
            import base64
            img_data = base64.b64decode(preview_data.split(",")[1])
            preview_path = os.path.join(LOCAL_DIR, project_name, "preview.png")
            with open(preview_path, "wb") as img_file:
                img_file.write(img_data)
            print(f"Saved project preview image to {preview_path}")
        except Exception as e:
            print(f"Error saving project preview image: {e}")

    # Determine script path to compile from (default to octopus.py)
    script_name = f"{project_name}.py"
    script_path = os.path.join(SCRIPTS_DIR, script_name)
    if not os.path.exists(script_path):
        script_name = "octopus.py"

    # Re-compile project files
    compile_project(project_name, script_name)

def rename_project(old_name, new_name):
    """
    Renames a local project folder and updates the seed parameter inside
    layout_overrides.json to regenerate placement.json and routes.json.
    """
    old_path = os.path.join(LOCAL_DIR, old_name)
    new_path = os.path.join(LOCAL_DIR, new_name)
    if not os.path.exists(old_path):
        raise FileNotFoundError(f"Project {old_name} not found")
    if os.path.exists(new_path):
        raise FileExistsError(f"Project {new_name} already exists")

    # Rename the directory
    os.rename(old_path, new_path)

    # Load layout_overrides.json, update seed value
    overrides_path = os.path.join(new_path, "layout_overrides.json")
    overrides = {}
    if os.path.exists(overrides_path):
        try:
            with open(overrides_path, "r", encoding="utf-8") as f:
                overrides = json.load(f)
        except Exception as e:
            print(f"Warning: Failed to load overrides during rename: {e}")

    # Generate new seed (random positive integer)
    import random
    new_seed = random.randint(100000, 999999)
    overrides["seed"] = new_seed

    with open(overrides_path, "w", encoding="utf-8") as f:
        json.dump(overrides, f, indent=2)

    # Determine script path to compile from
    script_name = f"{new_name}.py"
    if not os.path.exists(os.path.join(SCRIPTS_DIR, script_name)):
        script_name = "octopus.py"

    compile_project(new_name, script_name)
    print(f"Renamed project {old_name} to {new_name} successfully, new seed is {new_seed}")

def delete_project(name):
    """Recursively deletes a local project directory."""
    proj_path = os.path.join(LOCAL_DIR, name)
    if os.path.exists(proj_path):
        import shutil
        shutil.rmtree(proj_path)
        print(f"Deleted local project folder: {name}")
    else:
        raise FileNotFoundError(f"Project {name} not found")

def create_project(name):
    """
    Creates a new local project by copying the 'Project empty' template folder
    if it exists, otherwise falls back to generating a blank layout.
    """
    template_path = os.path.join(LOCAL_DIR, "Project empty")
    new_project_path = os.path.join(LOCAL_DIR, name)
    
    if os.path.exists(new_project_path):
        raise FileExistsError(f"Project '{name}' already exists")
        
    import shutil
    if os.path.exists(template_path):
        shutil.copytree(template_path, new_project_path)
        print(f"Created project '{name}' from template 'Project empty'")
    else:
        os.makedirs(new_project_path, exist_ok=True)
        # write default json files
        with open(os.path.join(new_project_path, "placement.json"), "w", encoding="utf-8") as f:
            json.dump({"orbits": [], "departments": [], "shards": [], "connections": []}, f)
        with open(os.path.join(new_project_path, "routes.json"), "w", encoding="utf-8") as f:
            json.dump([], f)
        with open(os.path.join(new_project_path, "layout_overrides.json"), "w", encoding="utf-8") as f:
            json.dump({"shards": {}, "sockets": {}}, f)
        print(f"Created blank project '{name}' (no template found)")
