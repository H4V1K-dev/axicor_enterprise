import os
import sys
import tempfile

def get_shm_path(zone_hash: int) -> str:
    filename = f"axicor_shard_{zone_hash:08X}"
    if sys.platform == "win32":
        return os.path.join(tempfile.gettempdir(), filename)
    return f"/dev/shm/{filename}"

def get_manifest_path(zone_hash: int) -> str:
    filename = f"axicor_manifest_{zone_hash:08X}.toml"
    if sys.platform == "win32":
        return os.path.join(tempfile.gettempdir(), filename)
    return f"/dev/shm/{filename}"
