# Axicor HFT: Ant-v4 Connectome

> [!WARNING]
> This example is currently non-functional due to ongoing maintenance and updates of the neuron library.

An integration example for an Embodied AI agent within the Gymnasium Ant-v4 environment. This demonstration showcases real-time Spiking Neural Network (SNN) execution using a 3-zone topology (Sensory, Thoracic, Motor) and Reinforcement-based STDP (dopaminergic modulation) without backpropagation.

## Architectural Principles (Zero-Garbage)
The agent is engineered following strict Data-Oriented Design (DOD). Inside the hot loop (`while episodes < EPISODES`), **all memory allocations are prohibited**. State vectors are normalized in-place, and the `PopulationEncoder` writes spikes directly into zero-copy `client.payload_views` buffers pre-allocated during UDP client initialization.

## Quick Start

### 0. Environment Setup
Ensure your Python virtual environment is activated and dependencies are installed:

```bash
# Windows (PowerShell)
.venv\Scripts\Activate.ps1
# Linux/macOS
source .venv/bin/activate

pip install gymnasium[mujoco] numpy
```

### 1. Topology Compilation (Baking Phase)
Generate the brain (layer mapping, axonal Cone Tracing) and compile binary `.state` and `.axons` dumps. The script automatically invokes the `axicor-baker` Rust compiler.

```bash
# For NVIDIA / AMD:
python examples/ant_exp/build_brain.py

# For CPU Fallback (No GPU):
python examples/ant_exp/build_brain.py --cpu
```
The build artifact will be generated at `Axicor-Models/AntConnectome.axic`.

### 2. Launching the Orchestrator (Day/Night Phase)
Start the HFT engine in a dedicated terminal. Select the command appropriate for your hardware.

**NVIDIA (CUDA):**
```bash
cargo run --release -p axicor-node -- Axicor-Models/AntConnectome.axic --log
```

**AMD (ROCm / HIP):**
```bash
cargo run --release -p axicor-node --features amd -- Axicor-Models/AntConnectome.axic --log
```

**CPU Fallback (No GPU):**
```bash
cargo run --release -p axicor-node --features axicor-compute/mock-gpu -- Axicor-Models/AntConnectome.axic --cpu --log
```

Wait for the `[Node] Bootstrap Successful. Hands-off to NodeRuntime.` message.
This typically takes 2–5 seconds, concluding with `Warmup complete for 0x273FD103. Voltage stabilized.`

### 3. Running the RL-Agent (Hot Loop)
In a new terminal, launch the UDP client. The ant will begin erratic movements (Motor Babbling), and the R-STDP modulator will begin reinforcing patterns that stabilize its posture.

*Note: We utilize zero-copy buffers via `client.payload_views`. This guarantees microsecond latency, as all memory allocations are completely eliminated within the hot loop.*

```bash
# Windows
.venv\Scripts\Activate.ps1
# Linux
source .venv/bin/activate

python examples/ant_exp/ant_agent.py
```

## Troubleshooting
- **ConnectionResetError (WinError 10054):** Ensure `axicor-node` has successfully finished the `Bootstrap` phase and logged `Hands-off to NodeRuntime` *before* launching the RL-Agent.
- **Port In Use:** Ensure port `9000` (Fast Path) is available; otherwise, the server will fail to initialize the network stack.
- **Archive Not Found:** Verify that `build_brain.py` completed successfully and that `Axicor-Models/AntConnectome.axic` exists at the specified path.
- **Virtual Machine Rendering Error (GLFWError 65542 / WGL):** Standard VM display drivers (like Hyper-V) do not support the OpenGL hardware acceleration required by Mujoco. To run the agent without the 3D window (Headless mode), open `examples/ant_exp/ant_agent.py` and change `env = gym.make("Ant-v4", render_mode="human")` to `render_mode=None`.
