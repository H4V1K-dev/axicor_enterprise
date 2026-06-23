#!/usr/bin/env python3
import sys
import struct
import os

def parse_axic(file_path):
    if not os.path.exists(file_path):
        print(f"Error: File '{file_path}' does not exist.")
        sys.exit(1)
        
    print(f"\n=== Inspecting Axicor Archive: {file_path} ===")
    file_size = os.path.getsize(file_path)
    print(f"Archive physical size: {file_size} bytes ({file_size / (1024*1024):.2f} MB)")
    
    with open(file_path, "rb") as f:
        # Read header (12 bytes)
        header_bytes = f.read(12)
        if len(header_bytes) < 12:
            print("Error: Archive header is truncated.")
            sys.exit(1)
            
        magic, version, file_count = struct.unpack("<III", header_bytes)
        
        # Verify magic: 0x43495841 ("AXIC")
        if magic != 0x43495841:
            print(f"Error: Invalid magic signature 0x{magic:08X} (expected 0x43495841 'AXIC')")
            sys.exit(1)
            
        print(f"Header: Magic='AXIC', Version={version}, File Count={file_count}")
        print("-" * 80)
        
        toc = []
        for i in range(file_count):
            entry_bytes = f.read(272)
            if len(entry_bytes) < 272:
                print(f"Error: Truncated TOC entry at index {i}")
                sys.exit(1)
                
            path_bytes = entry_bytes[0:256]
            # Decode path until first null byte
            null_idx = path_bytes.find(b'\0')
            if null_idx == -1:
                null_idx = 256
            path = path_bytes[:null_idx].decode("utf-8", errors="replace")
            
            offset, size = struct.unpack("<QQ", entry_bytes[256:272])
            toc.append((path, offset, size))
            
        # Sort TOC by offset to show physical layout
        toc.sort(key=lambda x: x[1])
        
        print(f"{'Path':<45} | {'Offset (Hex)':<12} | {'Size (Bytes)':<12} | {'Align Check'}")
        print("-" * 80)
        for path, offset, size in toc:
            align = "OK (4KB)" if offset % 4096 == 0 else f"BAD (off={offset%4096})"
            print(f"{path:<45} | 0x{offset:08X} | {size:<12} | {align}")
            
        print("-" * 80)
        
        # Read and display specific file details
        manifest_data = None
        for path, offset, size in toc:
            if path.endswith("manifest.toml"):
                f.seek(offset)
                toml_bytes = f.read(size)
                manifest_data = toml_bytes.decode("utf-8", errors="replace")
                print(f"\n--- Content of '{path}' ---")
                print(manifest_data)
                print("-" * 80)
            elif "BrainDNA" in path and path.endswith(".toml"):
                f.seek(offset)
                toml_bytes = f.read(size)
                toml_str = toml_bytes.decode("utf-8", errors="replace")
                print(f"\n--- Content of '{path}' ---")
                # Show first 25 lines or all
                lines = toml_str.splitlines()
                if len(lines) > 25:
                    print("\n".join(lines[:25]))
                    print("... [truncated]")
                else:
                    print(toml_str)
                print("-" * 80)
            elif path.endswith(".state"):
                f.seek(offset)
                state_hdr_bytes = f.read(16)
                if len(state_hdr_bytes) == 16:
                    s_magic, s_version, s_padded_n, s_total_axons = struct.unpack("<4sIII", state_hdr_bytes)
                    s_magic_str = s_magic.decode("ascii", errors="replace")
                    print(f"\n--- Binary File Structure: '{path}' ---")
                    print(f"  Header Magic:  {s_magic} ({s_magic_str})")
                    print(f"  Version:       {s_version}")
                    print(f"  Padded N:      {s_padded_n} (Neurons aligned to warp)")
                    print(f"  Total Axons:   {s_total_axons}")
                    print(f"  Total Size:    {size} bytes")
            elif path.endswith(".axons"):
                f.seek(offset)
                axons_hdr_bytes = f.read(16)
                if len(axons_hdr_bytes) == 16:
                    a_magic, a_version, a_total_axons, _pad = struct.unpack("<4sIII", axons_hdr_bytes)
                    a_magic_str = a_magic.decode("ascii", errors="replace")
                    print(f"\n--- Binary File Structure: '{path}' ---")
                    print(f"  Header Magic:  {a_magic} ({a_magic_str})")
                    print(f"  Version:       {a_version}")
                    print(f"  Total Axons:   {a_total_axons}")
                    print(f"  Total Size:    {size} bytes")
            elif path.endswith(".paths"):
                f.seek(offset)
                paths_hdr_bytes = f.read(16)
                if len(paths_hdr_bytes) == 16:
                    p_magic, p_version, p_total_axons, p_max_segments = struct.unpack("<IIII", paths_hdr_bytes)
                    print(f"\n--- Binary File Structure: '{path}' ---")
                    print(f"  Header Magic:  0x{p_magic:08X} ('HTAP' / 'PATH')")
                    print(f"  Version:       {p_version}")
                    print(f"  Total Axons:   {p_total_axons}")
                    print(f"  Max Segments:  {p_max_segments}")
                    print(f"  Total Size:    {size} bytes")

if __name__ == "__main__":
    archive_path = "cartpole.axic"
    if len(sys.argv) > 1:
        archive_path = sys.argv[1]
    
    # If file doesn't exist locally, look in parent dirs or workspace root
    if not os.path.exists(archive_path) and os.path.exists(os.path.join("../../", archive_path)):
        archive_path = os.path.join("../../", archive_path)
    elif not os.path.exists(archive_path) and os.path.exists(os.path.join("w:\\Workspace", archive_path)):
        archive_path = os.path.join("w:\\Workspace", archive_path)
        
    parse_axic(archive_path)
