import json
import os
import sys
import numpy as np

# Force matplotlib to use Agg backend for headless PNG generation
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

def plot_ephys_probe():
    csv_path = "artifacts/full_neuron_replay_314900022_trace.csv"
    if not os.path.exists(csv_path):
        print(f"Error: {csv_path} not found!")
        return

    # Load data
    data = np.genfromtxt(csv_path, delimiter=',', names=True)
    tick = data['tick']
    v_pre = data['voltage_pre'] / 1000.0  # uV to mV
    eff_th = data['effective_threshold'] / 1000.0  # uV to mV
    spikes = data['final_spike']

    time_ms = tick * 0.1  # dt = 0.1ms

    # Setup figure
    fig, ax = plt.subplots(figsize=(12, 5))
    ax.plot(time_ms, v_pre, color="#d62728", label="Membrane Potential V(t)", linewidth=1.0)
    ax.plot(time_ms, eff_th, 'k--', label="Effective Threshold V_th(t)", alpha=0.8, linewidth=1.0)

    # Plot spike times as vertical markers
    spike_indices = np.where(spikes == 1)[0]
    for s_idx in spike_indices:
        ax.axvline(time_ms[s_idx], color='red', alpha=0.4, linestyle=':', ymin=0.5, ymax=1.0)

    ax.set_title("EPHYS_PROBE_01 GLIF Replay (RUST Runner, dt=0.1ms)", fontsize=14)
    ax.set_xlabel("Time (ms)", fontsize=12)
    ax.set_ylabel("Potential (mV)", fontsize=12)
    ax.legend(loc="upper right")
    ax.grid(True, linestyle=':', alpha=0.6)

    plt.tight_layout()
    out_dir = "docs/engine/research/archive/2026-07-04_full_neuron_replay_314900022/images"
    os.makedirs(out_dir, exist_ok=True)
    out_path = os.path.join(out_dir, "ephys_probe_01_replay_rust.png")
    plt.savefig(out_path, dpi=150)
    plt.close()
    print(f"Saved EPHYS_PROBE_01 plot to: {out_path}")


def plot_fi_curve():
    json_path = "artifacts/full_neuron_replay_314900022_summary.json"
    if not os.path.exists(json_path):
        print(f"Error: {json_path} not found!")
        return

    with open(json_path, 'r') as f:
        data = json.load(f)

    amps = [entry['stimulus_pa'] for entry in data]
    counts = [entry['spike_count'] for entry in data]

    # Allen bio data for 314900022 (Scnn1a_L4_excitatory)
    # Amplitudes: -10, 30, 40, 50, 70, 90, 110, 130, 150, 190
    # Bio spikes: 0.0, 0.0, 0.0, 3.5, 11.0, 20.0, 22.0, 26.0, 29.0, 36.0
    bio_amps = [-10, 30, 40, 50, 70, 90, 110, 130, 150, 190]
    bio_counts = [0.0, 0.0, 0.0, 3.5, 11.0, 20.0, 22.0, 26.0, 29.0, 36.0]

    fig, ax = plt.subplots(figsize=(8, 5))
    ax.plot(amps, counts, color="#1f77b4", marker='o', label="Simulated (Full Neuron Replay)", linewidth=2.0)
    ax.plot(bio_amps, bio_counts, color="#2ca02c", marker='s', linestyle='--', label="Biological (Allen Cell Types)", linewidth=1.5)

    ax.set_title("f-I Curve Comparison (Specimen 314900022)", fontsize=14)
    ax.set_xlabel("Stimulus Current (pA)", fontsize=12)
    ax.set_ylabel("Spike Count (1s window)", fontsize=12)
    ax.legend(loc="upper left")
    ax.grid(True, linestyle=':', alpha=0.6)

    plt.tight_layout()
    out_dir = "docs/engine/research/archive/2026-07-04_full_neuron_replay_314900022/images"
    out_path = os.path.join(out_dir, "full_neuron_fi_curve.png")
    plt.savefig(out_path, dpi=150)
    plt.close()
    print(f"Saved f-I Curve plot to: {out_path}")


def plot_sweep_190():
    csv_path = "artifacts/full_neuron_replay_314900022_sweep_190.csv"
    if not os.path.exists(csv_path):
        print(f"Error: {csv_path} not found!")
        return

    # Load data
    data = np.genfromtxt(csv_path, delimiter=',', names=True)
    tick = data['tick']
    v_pre = data['voltage_pre'] / 1000.0  # uV to mV
    eff_th = data['effective_threshold'] / 1000.0  # uV to mV
    spikes = data['final_spike']
    i_ext = data['i_ext'] / 1000.0  # scaled current unit (roughly mV equivalent)

    time_ms = tick  # dt = 1.0ms

    fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(12, 8), sharex=True, gridspec_kw={'height_ratios': [3, 1]})

    # Voltage / Threshold
    ax1.plot(time_ms, v_pre, color="#17becf", label="Membrane Potential V(t)", linewidth=1.0)
    ax1.plot(time_ms, eff_th, 'r--', label="Effective Threshold V_th(t)", alpha=0.8, linewidth=1.0)
    
    # Draw spikes
    spike_indices = np.where(spikes == 1)[0]
    for s_idx in spike_indices:
        ax1.axvline(time_ms[s_idx], color='red', alpha=0.3, linestyle=':', ymin=0.5, ymax=1.0)

    ax1.set_title("Sweep 190 pA Simulation (Specimen 314900022)", fontsize=14)
    ax1.set_ylabel("Potential (mV)", fontsize=12)
    ax1.legend(loc="upper right")
    ax1.grid(True, linestyle=':', alpha=0.6)

    # Input Current
    ax2.plot(time_ms, i_ext, color="#2ca02c", label="External Input Current", linewidth=1.5)
    ax2.set_xlabel("Time (ms)", fontsize=12)
    ax2.set_ylabel("I_ext (a.u.)", fontsize=12)
    ax2.legend(loc="upper right")
    ax2.grid(True, linestyle=':', alpha=0.6)

    plt.tight_layout()
    out_dir = "docs/engine/research/archive/2026-07-04_full_neuron_replay_314900022/images"
    out_path = os.path.join(out_dir, "sweep_190_replay_rust.png")
    plt.savefig(out_path, dpi=150)
    plt.close()
    print(f"Saved Sweep 190 plot to: {out_path}")


if __name__ == "__main__":
    plot_ephys_probe()
    plot_fi_curve()
    plot_sweep_190()
