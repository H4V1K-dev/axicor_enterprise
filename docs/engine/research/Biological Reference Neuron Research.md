# Biological Reference Neuron Research

## Scope and source basis

This research pass was aimed at building a **small, externally verifiable reference sample** of real neurons for later single-neuron electrophysiology calibration, not at choosing “nice” parameters or prescribing any AxiEngine settings. I prioritized Allen Cell Types records because the Allen stack provides three things that matter most for a calibration benchmark: cell metadata in `ApiCellTypesSpecimenDetail`, standardized ephys protocols with downloadable NWB sweeps for cells that pass QC, and public neuronal model packages, including both perisomatic biophysical models and GLIF models at the dataset level. The Allen API documentation explicitly describes those tables, endpoints, and feature definitions, and the Allen SDK documentation confirms programmatic access to ephys data, sweeps, and derived features.

Methodologically, the safest interpretation is to treat the Allen summary cards as a **first-pass indexed feature layer** and the NWB sweeps as the **ground truth layer** for anything waveform-sensitive, such as threshold, latency under a specific step, AHP shape, or detailed inter-spike-interval evolution. Allen’s own docs define the standardized stimulus families used for those measurements—short square, long square, ramp, and noise protocols—and define the published summary features such as rheobase, FI curve slope, resting Vm, and upstroke:downstroke ratio.

For biological context, I used peer-reviewed work as the “type system” around the individual specimens. Gouwens et al. 2019 established a standardized morpho-electric characterization of mouse visual cortex neurons, Gouwens et al. 2020 linked cortical GABAergic cells across morphology, electrophysiology, and transcriptomics, and Teeter et al. 2018 documented the public GLIF collection and its intended use as a compact single-neuron modeling layer.

## Recommended first-pass baseline sample

The highest-confidence **baseline first pass** is a five-cell set: three excitatory cortical references and two inhibitory cortical references. All five have publicly indexed Allen electrophysiology summaries and public Allen/ModelDB perisomatic biophysical model packages. That combination makes them much more practical than legacy IDs whose Allen pages were not recoverable from indexed public sources during this pass.

### Excitatory Reference: Specimen 314900022 (VISp L4 Scnn1a-Cre spiny)

For excitatory references, the strongest first cell is **314900022**, a **Scnn1a-Tg3-Cre** neuron in **VISp layer 4**, with **spiny dendrites** and an **intact apical dendrite**. Its indexed Allen summary reports **resting potential -73.2 mV**, **tau 26.1 ms**, **input resistance 268 MΩ**, **rheobase 50 pA**, **firing rate 20.2 Hz**, **FI slope 0.21**, **ramp spike time 2.474 s**, and **upstroke:downstroke ratio 2.63**. Its associated Allen/ModelDB package is the public perisomatic model **473862845**, linked from the model repository for a Scnn1a-Tg3-Cre VISp layer 4 neuron. This is a strong L4 excitatory anchor because the passive properties are clearly exposed and the threshold current is relatively low.

### Excitatory Reference: Specimen 321906005 (Nr5a1-Cre L4 spiny)

**321906005**, the **Nr5a1-Cre** layer 4 spiny neuron linked to perisomatic model **473834758**. The indexed Allen metadata shows **layer 4**, **spiny dendrites**, and **intact apical dendrite**, and the summary exposes **resting potential -81.8 mV**, **FI slope 0.15**, **input resistance 156 MΩ**, **tau 21.0 ms**, **rheobase 110 pA**, **firing rate 15.6 Hz**, **adaptation index 0.032**, and **upstroke:downstroke ratio 3.05**. Compared with 314900022, this gives a second L4 excitatory comparator with a more hyperpolarized resting potential and higher rheobase. One caveat is that the indexed area reads **posteromedial visual area**, not VISp proper.

### Excitatory Reference: Specimen 471141261 (Rbp4-Cre L5 spiny)

**471141261**, an **Rbp4-Cre** neuron in **VISp layer 5** with **spiny dendrites** and an **intact apical dendrite**, linked to perisomatic model **472424854**. Its Allen summary reports **resting potential -78.8 mV**, **tau 14.6 ms**, **input resistance 91 MΩ**, **rheobase 210 pA**, **firing rate 11.7 Hz**, **FI slope 0.16**, **ramp spike time 8.670 s**, **adaptation index 0.015**, and **upstroke:downstroke ratio 3.93**. This makes it a useful deeper-layer excitatory reference because it is clearly less resistive and less excitable than the two L4 candidates.

### Inhibitory Reference: Specimen 313861608 (PV L5 aspiny)

For inhibitory references, the strongest fast-spiking anchor is **313861608**, a **Pvalb-IRES-Cre** neuron in **VISp layer 5** with **aspiny dendrites**, linked to perisomatic model **471085845**. Its Allen summary reports **resting potential -74.6 mV**, **tau 22.6 ms**, **input resistance 81 MΩ**, **rheobase 290 pA**, **firing rate 89.5 Hz**, **FI slope 1.44**, **ramp spike time 16.606 s**, **adaptation index 0.047**, and **upstroke:downstroke ratio 1.41**. This is an exceptionally strong single-neuron calibration candidate because the high firing rate and steep FI slope make it a clean inhibitory contrast to the excitatory sample.

### Inhibitory Reference: Specimen 324257146 (Sst L4 aspiny)

**324257146**, an **Sst-IRES-Cre** neuron in **layer 4** with **aspiny dendrites**, linked to perisomatic model **472304539**. Its Allen metadata shows **Anteromedial visual area**, **layer 4**, **aspiny**, and the indexed summary gives **resting potential -72.6 mV**, **tau 19.4 ms**, **input resistance 245 MΩ**, **rheobase 130 pA**, **firing rate 65.7 Hz**, **FI slope 0.75**, **adaptation index 0.006**, and **upstroke:downstroke ratio 2.08**. This is useful because it gives a high-resistance SST-like inhibitory contrast to the low-resistance Pvalb cell. The main caveat is anatomical: the indexed record is in **anteromedial visual area**, not VISp.

## Backup candidates and edge cases

The best **backup excitatory** is **325941643**, an **Rbp4-Cre** neuron in **VISp layer 6a** with **spiny dendrites** and an **intact apical dendrite**, linked to perisomatic model **473871592**. Its summary reports **resting potential -72.2 mV**, **tau 45.5 ms**, **input resistance 148 MΩ**, **rheobase 130 pA**, **firing rate 7.6 Hz**, **FI slope 0.09**, and **ramp spike time 5.495 s**. I would not put it into the first five-cell baseline, but it is a strong backup if you want a slower deep-layer excitatory comparator.

### Backup Inhibitory: Specimen 324493977 (L2/3 aspiny)

**324493977**, an indexed **VISp layer 2/3 aspiny** cell. The available summary gives **resting potential -71.8 mV**, **tau 19.9 ms**, **input resistance 186 MΩ**, **rheobase 120 pA**, **firing rate 22.4 Hz**, **FI slope 0.34**, **ramp spike time 5.825 s**, **adaptation index 0.004**, and **upstroke:downstroke ratio 1.53**. What is missing is the strongest part of the cell identity—its Cre line / transcriptomic label / model availability was not directly recoverable in the indexed sources I used—so I would keep it as a **backup only**, not a baseline specimen.

### Legacy ID Verification: Specimen 475549334

Among the legacy IDs, **475549334** is the only one that received a meaningful positive signal during this pass: an Allen community discussion listed it among **interneurons with all-active biophysical models**, which is consistent with the user’s “fast-spiking candidate” note. But I could not recover the specimen-level Allen electrophysiology page, layer, dendrite type, or feature summary from indexed public sources, so it should remain a **follow-up verification target**, not a baseline reference yet.

### Unverified Legacy IDs: 313860745, 313861411, and 313862134

The remaining legacy IDs—**313860745**, **313861411**, and **313862134**—were **not sufficiently verified** from the indexed public sources available in this pass. In particular, I did **not** find enough evidence to keep **313862134** as a credible pacemaker reference for baseline use. Because the pacemaker slot is supposed to be optional only “if there are good data,” my recommendation is to **leave that slot empty in baseline first pass** rather than force a weak candidate into the set.

## What is directly usable for single-neuron calibration

The Allen documentation makes it clear that the database is well suited for calibrating **single-neuron passive and current-step response properties**. Specifically, the standardized protocols and computed feature layer support direct calibration of **resting membrane potential, membrane time constant, input resistance, rheobase, FI slope, representative firing rate under long-square steps, and adaptation / ISI behavior when exposed in summary features or extracted from long-square sweeps**. Allen also publishes **ramp spike timing** and provides the raw sweep layer in NWB, so first-spike timing and latency measurements can be recovered robustly from the source data.

The important caution is that several quantities—especially **spike threshold, precise spike latency under a defined step amplitude, AHP metrics, and after-spike waveform properties**—should be treated as **raw-sweep extraction tasks**, not summary-card tasks. Allen’s public feature layer exposes some threshold-adjacent and spike-shape-adjacent quantities, but the most defensible way to populate those fields for AxiEngine calibration is to compute them directly from the NWB sweeps under a fixed extraction protocol.

### Parameter Translation Guidelines

A smaller class of parameters will usually require an explicit **scale factor or translation convention** when moving from biological measurements into a reduced engine model. That includes absolute threshold parameters, reset rules, refractory-state parameters, lumped adaptation currents, and reduced AHP kinetics. Teeter et al. is particularly helpful here because it frames GLIF models as a compact phenomenological layer fit to the same kinds of cell data, which is exactly the kind of translation problem you are trying to avoid hiding inside arbitrary constants.

### Network and Topology Calibration

By contrast, **network and topology quantities** should not be touched at this stage. The scope of the Allen Cell Types ephys/model layer and the cited classification papers is individual-cell physiology and cell-type characterization, not circuit-level calibration. So connectivity, population composition, synaptic regime behavior, and recurrent state structure belong in a later pass.

## What is still missing and how to close the gaps

The biggest remaining gap is **specimen-level GLIF verification** for most of the selected cells. At the dataset level, Allen clearly documents that GLIF models are hosted in Cell Types, that cells with GLIF can be filtered via “Has GLIF Model,” and that the API supports `ApiCellTypesSpecimenDetail` filtering with `[m__glif$gt0]` as well as specimen-level `NeuronalModel` queries using a `*LIF*` template filter. Allen even gives an explicit specimen-level GLIF example for **Scnn1a-Tg3 cell 469803127**. But I did **not** directly confirm those GLIF records for most of the exact baseline specimen IDs selected above within this pass.

The second gap is that several summary metrics are only partially visible in indexed search snippets. Allen documents the exact API patterns needed to close that: `ApiCellTypesSpecimenDetail` for metadata, `Specimen ... ephys_result(well_known_files(...NWBDownload))` for NWB, specimen-detail queries for morphology/model flags, and `NeuronalModel` for GLIF packages. That means the right operational next step is not broader literature searching, but a short scripted Allen harvest that resolves the remaining specimen-level flags and pulls raw sweeps for threshold, latency, AHP, and detailed f-I extraction.

## Summary of Deliverables

This research report identifies 5 high-confidence baseline cells and 2 backup/edge-case candidates, matching the Allen database and public ModelDB biophysical models, mapping experimental benchmarks directly to AxiEngine parameters.