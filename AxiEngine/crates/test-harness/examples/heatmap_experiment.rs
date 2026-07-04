use layout::{BurstHeads8, VariantParameters, VARIANT_LUT_LEN};
use physics::{
    active_tail_hit, heartbeat_spike, homeostasis_decay, initial_axon_head, is_glif_spike,
    update_glif_voltage, weight_to_charge,
};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::Instant;
use test_harness::{
    cpu_apply_gsop, cpu_inject_inputs, cpu_propagate_axons, MvpAxonBuffer, MvpStateBuffer,
};
use types::AXON_SENTINEL;

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.state >> 32) as u32
    }

    fn next_range(&mut self, min: u32, max: u32) -> u32 {
        assert!(max > min);
        let range = max - min;
        min + (self.next_u32() % range)
    }
}

fn create_experiment_variants() -> [VariantParameters; VARIANT_LUT_LEN] {
    [VariantParameters {
        threshold: 1000,
        rest_potential: -70000,
        leak_shift: 6,
        homeostasis_penalty: 50,
        spontaneous_firing_period_ticks: 0,
        initial_synapse_weight: 100,
        gsop_potentiation: 128,
        gsop_depression: 64,
        homeostasis_decay: 1,
        refractory_period: 2,
        synapse_refractory_period: 5,
        signal_propagation_length: 5,
        is_inhibitory: 0,
        inertia_curve: [128; 8],
        ahp_amplitude: 0,
        _pad1: [0; 6],
        adaptive_leak_min_shift: 0,
        adaptive_leak_gain: 0,
        adaptive_mode: 0,
        _leak_pad: [0; 3],
        d1_affinity: 64,
        d2_affinity: 64,
        heartbeat_m: 1000, // 1000 DDS phase step for spontaneous background activity
    }; VARIANT_LUT_LEN]
}

pub fn research_update_neurons(
    state_buf: &mut MvpStateBuffer,
    axon_buf: &mut MvpAxonBuffer,
    variants: &[VariantParameters; VARIANT_LUT_LEN],
    current_tick: u64,
    v_seg: u32,
) {
    let padded_n = state_buf.padded_n();
    let total_axons = state_buf.total_axons();

    for i in 0..padded_n {
        let flags_val = state_buf.read_soma_flags(i);
        let var_id = ((flags_val >> 4) & 0x0F) as usize;
        let variant = &variants[var_id];

        // 1. Homeostasis decay
        let old_thresh_offset = state_buf.read_threshold_offset(i);
        let new_thresh_offset =
            homeostasis_decay(old_thresh_offset, variant.homeostasis_decay as i32);
        state_buf.write_threshold_offset(i, new_thresh_offset);

        // 2. Check neuron refractory timer
        let t = state_buf.read_timer(i);
        let mut is_glif = false;
        let mut flags = flags_val;

        if t > 0 {
            state_buf.write_timer(i, t - 1);
            let v_reset = variant
                .rest_potential
                .wrapping_sub(variant.ahp_amplitude as i32);
            state_buf.write_soma_voltage(i, v_reset);
            flags &= !0x01;
            state_buf.write_soma_flags(i, flags);
        } else {
            // Compute i_in
            let mut i_in = 0i32;
            for d in 0..128 {
                let target_packed = state_buf.read_dendrite_target(d, i);
                if target_packed == 0 {
                    break; // Sentinel
                }
                let raw_id = target_packed & 0x00FFFFFF;
                if raw_id == 0 {
                    break; // Trap
                }
                let axon_id = (raw_id - 1) as usize;
                let seg_idx = target_packed >> 24;

                if axon_id < total_axons {
                    let h = axon_buf.read_head(axon_id);
                    let heads = [h.h0, h.h1, h.h2, h.h3, h.h4, h.h5, h.h6, h.h7];
                    if active_tail_hit(&heads, seg_idx, variant.signal_propagation_length as u32) {
                        let w = state_buf.read_dendrite_weight(d, i);
                        i_in = i_in.wrapping_add(weight_to_charge(w));
                    }
                }
            }

            let v_old = state_buf.read_soma_voltage(i);
            let v_new = update_glif_voltage(
                v_old,
                i_in,
                variant.rest_potential,
                new_thresh_offset,
                variant.leak_shift as i32,
                variant.adaptive_leak_gain as i32,
                1,
                variant.adaptive_mode as i32,
            );

            is_glif = is_glif_spike(v_new, variant.threshold, new_thresh_offset);
            if is_glif {
                let v_reset = variant
                    .rest_potential
                    .wrapping_sub(variant.ahp_amplitude as i32);
                state_buf.write_soma_voltage(i, v_reset);
                state_buf.write_timer(i, variant.refractory_period);
                state_buf.write_threshold_offset(
                    i,
                    new_thresh_offset.wrapping_add(variant.homeostasis_penalty),
                );
            } else {
                state_buf.write_soma_voltage(i, v_new);
            }
        }

        // DDS Heartbeat
        let is_heartbeat = heartbeat_spike(current_tick, variant.heartbeat_m, i as u32);
        let final_spike = is_glif || is_heartbeat;

        if final_spike {
            flags |= 0x01;
            let old_burst = (flags >> 1) & 0x07;
            let new_burst = old_burst.saturating_add(1).min(7);
            flags = (flags & !0x0E) | (new_burst << 1);

            let axon_id = state_buf.read_soma_to_axon(i) as usize;
            if axon_id < total_axons {
                let mut h = axon_buf.read_head(axon_id);
                h.h7 = h.h6;
                h.h6 = h.h5;
                h.h5 = h.h4;
                h.h4 = h.h3;
                h.h3 = h.h2;
                h.h2 = h.h1;
                h.h1 = h.h0;
                h.h0 = initial_axon_head(v_seg);
                axon_buf.write_head(axon_id, h);
            }
        } else {
            flags &= !0x01;
        }
        state_buf.write_soma_flags(i, flags);
    }
}

fn dump_deltas(state_buf: &MvpStateBuffer, tick: u64) {
    let filename = format!("deltas_tick_{:04}.csv", tick);
    let file = File::create(&filename).expect("Unable to create file");
    let mut writer = BufWriter::new(file);

    writeln!(writer, "source_tid,target_tid,delta_weight").unwrap();

    let padded_n = state_buf.padded_n();
    for tid in 0..padded_n {
        for slot in 0..128 {
            let target_packed = state_buf.read_dendrite_target(slot, tid);
            if target_packed == 0 {
                break;
            }
            let raw_id = target_packed & 0x00FFFFFF;
            if raw_id == 0 {
                break;
            }
            let source_tid = raw_id - 1;
            let w = state_buf.read_dendrite_weight(slot, tid);
            let delta = w - 50000;
            writeln!(writer, "{},{},{}", source_tid, tid, delta).unwrap();
        }
    }
    println!("Dumped deltas to {}", filename);
}

#[allow(clippy::needless_range_loop)]
fn main() {
    println!("Starting heatmap experiment (1K neurons, 128 synapses)...");

    let padded_n = 1024;
    let total_axons = 1024;
    let mut state_buf = MvpStateBuffer::new(padded_n, total_axons);
    let mut axon_buf = MvpAxonBuffer::new(total_axons);
    let variants = create_experiment_variants();

    // 1. Topology & Weights Setup
    let mut rng = SimpleRng::new(42);
    for tid in 0..padded_n {
        state_buf.write_soma_to_axon(tid, tid as u32);
        state_buf.write_soma_voltage(tid, -70000);
        state_buf.write_threshold_offset(tid, 0);
        state_buf.write_timer(tid, 0);
        state_buf.write_soma_flags(tid, 0);

        let mut targets = Vec::new();
        while targets.len() < 128 {
            let t = rng.next_range(0, 1024) as usize;
            if t != tid && !targets.contains(&t) {
                targets.push(t);
            }
        }
        for (slot, &target_tid) in targets.iter().enumerate() {
            let seg_idx = rng.next_range(0, 5); // prop window = 5
            let target_packed = (seg_idx << 24) | ((target_tid + 1) as u32);
            state_buf.write_dendrite_target(slot, tid, target_packed);
            state_buf.write_dendrite_weight(slot, tid, 50000);
            state_buf.write_dendrite_timer(slot, tid, 0);
        }
    }

    // 2. Input Stimulation Bitmask Setup
    // Selected 10 fixed noise/input neurons: 100, 200, 300, 400, 500, 600, 700, 800, 900, 1000
    let noise_neurons = [100, 200, 300, 400, 500, 600, 700, 800, 900, 1000];
    let mut input_bitmask = [0u32; 32];
    for &id in &noise_neurons {
        let word_idx = id / 32;
        let bit_idx = id % 32;
        input_bitmask[word_idx] |= 1u32 << bit_idx;
    }

    // 3. Execution loop
    let mut heads = vec![BurstHeads8::empty(AXON_SENTINEL); 1024];
    let start_time = Instant::now();
    for t in 1..=10000 {
        // A. Propagate axonal spikes
        cpu_propagate_axons(&mut heads, 1);

        // B. Inject input stimulation every 20 ticks
        if t % 20 == 0 {
            cpu_inject_inputs(&mut heads, &input_bitmask, 0, 1024, 1);
        }

        // Synchronize heads to axon_buf before running plasticity
        for i in 0..1024 {
            axon_buf.write_head(i, heads[i]);
        }

        // C. Apply GSOP plasticity (STDP) for spiking somas
        cpu_apply_gsop(&mut state_buf, &axon_buf, &variants, 0);

        // D. Update GLIF voltage and generate spikes (mutates axon_buf on spike)
        research_update_neurons(&mut state_buf, &mut axon_buf, &variants, t, 1);

        // Synchronize heads back from axon_buf after updating neurons
        for i in 0..1024 {
            heads[i] = axon_buf.read_head(i);
        }

        // E. Save state every 2 000 ticks
        if t % 2000 == 0 {
            dump_deltas(&state_buf, t);
        }
    }
    let duration = start_time.elapsed();
    println!("Experiment completed in {:.2?}", duration);
}
