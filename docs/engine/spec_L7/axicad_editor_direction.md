# Design Direction: axi-viewer

> Spec Version: 1.1  
> Date: 2026-07-01  
> Status: Draft (Evaluation & Design Direction)

---

## Section 1. Overview & Scope

The goal of the **axi-viewer** is to provide a simple diagnostic "microscope" for viewing and analyzing simulation artifacts. It is a read-only tool designed to quickly load simulation folders, render telemetry, inspect output spikes, and compare different runs.

At this stage, the viewer does not load `.axic` binary archives directly and does not write or modify any configurations. It works entirely with exported JSON and CSV telemetry files.

---

## Section 2. Primary Modes of Operation

1. **Artifacts Directory Inspector**:
   - Open and parse simulation folders containing any `*_summary.json` (such as `local_engine_*_summary.json` or `node_summary.json`) or `node_summary.csv`, as well as `node_batches.csv`, `node_outputs.csv`, and `node_output_spikes.csv`.
2. **Soma Placement Visualizer (Optional)**:
   - 2D/3D visualization of soma placement positions (when placement data is exported).
3. **Connectivity Visualizer (Optional)**:
   - Visualizing axon paths and connections (when topology data is exported).
4. **Weight Heatmap Visualizer (Optional)**:
   - Rendering synapse weight distributions (when weight matrix data is exported).
5. **Runtime Telemetry Inspector**:
   - Loading and graphing batch execution speed, generated spikes, and dropped spikes over time.
6. **Differential Comparison Mode**:
   - Loading two runs side-by-side or overlaid (e.g. comparing a baseline run against an active/stimulated run).

---

## Section 3. MVP Requirements (Stage A Viewer: Read-Only Dev Viewer)

The minimum viable product must support the following:

- **Input Ingestion**: Load a local directory (such as `artifacts/local_engine_active_e2e`) containing simulation CSV/JSON outputs.
- **Summary Cards**: Quick stats display (total ticks, execution wall time, total generated spikes, total output spikes written, total dropped spikes, resolved backend hardware).
- **Spike Raster Plot**: An interactive scatter/dot plot displaying individual soma spikes (`node_output_spikes.csv` containing `batch_idx,tick_index,slot,soma_id`) over time.
- **Charts Over Time**: Graphing batch execution times, generated/output/dropped spikes over batches.
- **Weight Heatmap (Optional Panel)**: A 2D slice representing synaptic weights if the weights CSV file is present.
- **Soma Placement Plot (Optional Panel)**: 2D/3D scatter plot of soma coordinate positions if the placement CSV file is present.
- **Comparison Engine**: Select two directories (e.g., baseline and active) to overlay or compare their charts side-by-side.
- **Diagnostic Export**: Support exporting charts and views to PNG images.
- **Headless Screen Capture**: Compatible with Playwright scripts for automated screenshot generation.

---

## Section 4. Technology Stack Alternatives

### 1. Browser-First Web Application (WebGL / Canvas + React/Svelte)
- **Frameworks**: Svelte or React, TailwindCSS, charting libraries (Plotly/Chart.js/uPlot), and WebGL/WebGPU or Three.js for 3D/2D scatter plots.
- **Pros**:
  - **Highest prototyping speed**: Rich ecosystem of chart packages, tables, and layouts available instantly.
  - **Easy automation**: Simple to write Playwright/Puppeteer scripts to spin up a headless browser, load the viewer, and capture PNGs/videos.
  - **Zero backend dependencies**: Reads JSON and CSV files using browser file APIs (or a simple static server).
- **Cons**:
  - Browser memory limits when loading massive CSV files (e.g., millions of lines of raw spikes).
- **Risks**: Parsing giant CSV files directly in JS might block the main thread (requires Web Workers).
- **Prototyping Speed**: Fast.
- **100k+ Somas scaling**: Good (requires binary/columnar data formats or downsampling, as raw CSV parsing on millions of rows will bottleneck performance).
- **Rust crates integration**: None (reads raw CSV/JSON files, avoiding binary mmap parsing).

### 2. Hybrid Shell (Tauri Wrapper)
- **Frameworks**: Tauri backend (Rust) + browser-first web app (Svelte/React).
- **Pros**:
  - Easy transition from the browser-first app (web code is compiled into Tauri).
  - Can call native Rust crates (like `vfs` or binary parsing) if we decide to read `.axic` files directly in future stages.
- **Cons**:
  - Adds build and packaging complexity during early development.
- **Risks**: High IPC serialization overhead if passing large datasets from Rust backend to webview.
- **Prototyping Speed**: Fast.
- **100k+ Somas scaling**: Excellent.
- **Rust crates integration**: Trivial.

### 3. Full Rust Native Desktop (egui / Bevy)
- **Libraries**: `egui`/`eframe`, Bevy Engine, or native `wgpu` wrappers.
- **Pros**:
  - Low memory overhead, native execution, direct binary integration.
- **Cons**:
  - Low prototyping speed for complex layouts, grids, and interactive charts compared to the web ecosystem.
  - Harder to automate headless screenshot capture.
- **Risks**: Immediate-mode GUI (`egui`) might lag when rendering highly interactive plots with thousands of nodes.
- **Prototyping Speed**: Medium.
- **100k+ Somas scaling**: Excellent.
- **Rust crates integration**: Trivial.

---

## Section 5. Stack Matrix Evaluation

| Criterion | Browser-First Web App | Hybrid (Tauri Shell) | Full Rust (egui/Bevy) |
|---|---|---|---|
| **Prototyping Speed** | **Fastest** | Fast | Medium |
| **100k+ Somas Rendering** | Good (with downsampling)| **Excellent** | **Excellent** |
| **Rust Crates Integration**| None | **Trivial** | **Trivial** |
| **Headless Screen Capture**| **Trivial** (Playwright) | Complex | Complex |
| **UI Library Maturity** | **High** | **High** | Low |

---

## Section 6. Proposed Routes

### Route A: "Fast Dev Viewer" (Browser-First web app)
A lightweight local web page located in a standalone folder (e.g., `tools/axi-viewer/` or `apps/axi-viewer/`) that consumes CSV/JSON files from an output folder. It provides a simple GUI with plotly charts, spike rasters, and summary card overlays, along with a Playwright capture script. Tauri is kept as an optional future wrapper if direct binary or file system access is needed.
- *Time to MVP*: 1-2 weeks.
- *Target Audience*: Developers looking to visually analyze simulation telemetry and compare baseline/active runs.

### Route B: "Desktop IDE" (Bevy/wgpu or Tauri + Rust Core)
A native workstation app that compiles configs (`axi-baker`), runs simulation processes (`axi-node`), and visualizes spatial axon growth directly using `.axic` binary buffers.
- *Time to MVP*: 8+ weeks.
- *Target Audience*: Scientists configuring large structural networks.

---

## Section 7. Recommendation

### Selected First Step: **Route A: "Fast Dev Viewer" (Browser-First Web App)**

**Rationale**:
1. **Microscope Focus**: We need to see and compare simulation results quickly. Consuming CSV/JSON files is simple and robust.
2. **Prototyping Velocity**: Utilizing ready-made HTML5 chart and grid libraries saves weeks of layout code.
3. **Automated Capture**: Running a Playwright script to capture PNGs and output videos fits naturally with a browser-first application.
4. **No Lock-in**: The viewer remains a clean static frontend. If we need to expand it into a full file editor or packager later, it can be wrapped in Tauri or a Rust backend shell without rewriting the UI logic.
