import os
import glob
import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.colors as colors

def main():
    # Dynamically find paths relative to this script
    script_dir = os.path.dirname(os.path.abspath(__file__))
    workspace_root = os.path.dirname(script_dir)
    
    # Search for CSV files in workspace root, script dir, and AxiEngine dir
    csv_search_paths = [
        os.path.join(workspace_root, "deltas_tick_*.csv"),
        os.path.join(workspace_root, "AxiEngine", "deltas_tick_*.csv"),
        os.path.join(script_dir, "deltas_tick_*.csv")
    ]
    
    csv_files = []
    for pattern in csv_search_paths:
        csv_files.extend(glob.glob(pattern))
    
    # Keep unique paths and sort them
    csv_files = sorted(list(set(csv_files)))
    
    if not csv_files:
        print("Error: No deltas_tick_*.csv files found!")
        print("Checked patterns:")
        for pattern in csv_search_paths:
            print(f"  - {pattern}")
        return
        
    output_dir = os.path.join(workspace_root, "docs", "engine", "research", "archive", "_active", "mvp_cpu_replay_v1", "images")
    os.makedirs(output_dir, exist_ok=True)
    
    for csv_path in csv_files:
        tick_str = os.path.basename(csv_path).replace("deltas_tick_", "").replace(".csv", "")
        print(f"Processing {csv_path} (Tick {tick_str})...")
        
        df = pd.read_csv(csv_path)
        
        # 1024x1024 matrix initialization with NaN
        matrix = np.full((1024, 1024), np.nan)
        
        # Fill the matrix: row = source_tid (pre), col = target_tid (post)
        matrix[df['source_tid'].values, df['target_tid'].values] = df['delta_weight'].values
        
        plt.figure(figsize=(10, 8), dpi=150)
        
        # Custom colormap: 'bwr' (Blue-White-Red)
        # We set background (NaN/unconnected cells) to #111111 (almost black)
        cmap = plt.colormaps['bwr'].copy()
        cmap.set_bad(color='#111111')
        
        # Find absolute max for symmetric scaling
        max_val = np.nanmax(np.abs(matrix))
        if max_val == 0 or np.isnan(max_val):
            max_val = 1.0
            
        im = plt.imshow(matrix, cmap=cmap, norm=colors.Normalize(vmin=-max_val, vmax=max_val), interpolation='nearest')
        
        plt.colorbar(im, label="Weight Delta (current_weight - 50000)")
        plt.title(f"Synaptic Plasticity Matrix (All-to-All STDP) at Tick {tick_str}\n(Black = No Connection, Blue = LTD, Red = LTP)", fontsize=11, pad=15)
        plt.xlabel("Target Neuron ID (Post-Synaptic)", fontsize=10)
        plt.ylabel("Source Neuron ID (Pre-Synaptic)", fontsize=10)
        
        plt.tight_layout()
        output_png = os.path.join(output_dir, f"heatmap_tick_{tick_str}.png")
        plt.savefig(output_png, facecolor='#1e1e1e', edgecolor='none')
        plt.close()
        print(f"Saved heatmap plot to: {output_png}")

if __name__ == "__main__":
    main()
