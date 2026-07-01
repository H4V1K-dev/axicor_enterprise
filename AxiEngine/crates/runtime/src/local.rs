//! Implement local day loop runtime coordinator.

use crate::dto::{
    LocalRuntimeConfig, RuntimeBatchInput, RuntimeBatchReport, RuntimeState, RuntimeStats,
};
use crate::error::RuntimeError;

/// Local orchestrator of a day cycle loop execution for a single shard.
pub struct LocalRuntime {
    engine: compute::ShardEngine,
    config: LocalRuntimeConfig,
    state: RuntimeState,
    stats: RuntimeStats,
    current_tick: u64,
    cached_output_spikes: Vec<u32>,
    cached_output_spike_counts: Vec<u32>,
}

impl LocalRuntime {
    /// Constructs a new runtime instance, checking that the engine is already Running.
    pub fn new(
        engine: compute::ShardEngine,
        config: LocalRuntimeConfig,
    ) -> Result<Self, RuntimeError> {
        let engine_state = engine.state();
        if engine_state != compute::LifecycleState::Running {
            return Err(RuntimeError::InvalidEngineLifecycle {
                actual: engine_state,
            });
        }

        Ok(Self {
            engine,
            config,
            state: RuntimeState::Running,
            stats: RuntimeStats::default(),
            current_tick: 0,
            cached_output_spikes: Vec::new(),
            cached_output_spike_counts: Vec::new(),
        })
    }

    /// Coordinates synchronous execution of a day batch.
    pub fn run_batch(
        &mut self,
        input: RuntimeBatchInput<'_>,
    ) -> Result<RuntimeBatchReport, RuntimeError> {
        if self.state != RuntimeState::Running {
            return Err(RuntimeError::InvalidState {
                from: self.state,
                expected: "Running",
            });
        }

        // Check for biological tick overflow
        let sync_ticks = self.config.sync_batch_ticks;
        let tick_base = self.current_tick;
        if self.current_tick.checked_add(sync_ticks as u64).is_none() {
            return Err(RuntimeError::TickOverflow {
                current: self.current_tick,
                sync: sync_ticks,
            });
        }

        // Validate incoming_spike_counts size
        let counts_len = input.incoming_spike_counts.len();
        if counts_len != sync_ticks as usize {
            return Err(RuntimeError::InvalidInputDimensions {
                field: "incoming_spike_counts",
                expected: sync_ticks as usize,
                actual: counts_len,
            });
        }

        // Validate elements inside incoming_spike_counts
        let max_spikes = self.config.max_spikes_per_tick;
        for &c in input.incoming_spike_counts {
            if c > max_spikes {
                return Err(RuntimeError::InvalidInputDimensions {
                    field: "incoming_spike_counts value",
                    expected: max_spikes as usize,
                    actual: c as usize,
                });
            }
        }

        // Validate incoming_spikes buffer requirements
        match input.incoming_spikes {
            Some(spikes) => {
                let required_min = (sync_ticks as usize)
                    .checked_mul(max_spikes as usize)
                    .ok_or(RuntimeError::CapacityExceeded {
                        reason: "incoming spikes capacity overflow",
                    })?;
                if spikes.len() < required_min {
                    return Err(RuntimeError::InvalidInputDimensions {
                        field: "incoming_spikes",
                        expected: required_min,
                        actual: spikes.len(),
                    });
                }
            }
            None => {
                // If incoming_spikes is None, all counts must be 0
                for &c in input.incoming_spike_counts {
                    if c != 0 {
                        return Err(RuntimeError::InvalidInputDimensions {
                            field: "incoming_spike_counts (when incoming_spikes is None)",
                            expected: 0,
                            actual: c as usize,
                        });
                    }
                }
            }
        }

        // Validate input_bitmask requirements
        if let Some(mask) = input.input_bitmask {
            let expected_mask_len = (sync_ticks as usize)
                .checked_mul(self.config.input_words_per_tick as usize)
                .ok_or(RuntimeError::CapacityExceeded {
                    reason: "input bitmask capacity overflow",
                })?;
            if mask.len() != expected_mask_len {
                return Err(RuntimeError::InvalidInputDimensions {
                    field: "input_bitmask",
                    expected: expected_mask_len,
                    actual: mask.len(),
                });
            }
        }

        // Set up output buffers
        let total_output_capacity = (sync_ticks as usize)
            .checked_mul(max_spikes as usize)
            .ok_or(RuntimeError::CapacityExceeded {
                reason: "output spikes capacity overflow",
            })?;
        self.cached_output_spikes.resize(total_output_capacity, 0);
        self.cached_output_spikes.fill(0);
        self.cached_output_spike_counts
            .resize(sync_ticks as usize, 0);
        self.cached_output_spike_counts.fill(0);

        // Build command payload
        let cmd = compute_api::DayBatchCmd {
            tick_base,
            sync_batch_ticks: sync_ticks,
            v_seg: self.config.v_seg,
            dopamine: self.config.dopamine,
            input_words_per_tick: self.config.input_words_per_tick,
            max_spikes_per_tick: max_spikes,
            num_outputs: self.config.mapped_soma_ids.len() as u32,
            virtual_offset: self.config.virtual_offset,
            num_virtual_axons: self.config.num_virtual_axons,
            input_bitmask: input.input_bitmask,
            incoming_spikes: input.incoming_spikes,
            incoming_spike_counts: input.incoming_spike_counts,
            mapped_soma_ids: &self.config.mapped_soma_ids,
            output_spikes: &mut self.cached_output_spikes,
            output_spike_counts: &mut self.cached_output_spike_counts,
        };

        match self.engine.run_day_batch(cmd) {
            Ok(result) => {
                let ticks_executed = result.ticks_executed;
                self.current_tick = self.current_tick.saturating_add(ticks_executed as u64);

                // Update cumulative stats
                self.stats.current_tick = self.current_tick;
                self.stats.batches_executed = self.stats.batches_executed.saturating_add(1);
                self.stats.ticks_executed = self
                    .stats
                    .ticks_executed
                    .saturating_add(ticks_executed as u64);
                self.stats.generated_spikes = self
                    .stats
                    .generated_spikes
                    .saturating_add(result.generated_spikes_count as u64);
                self.stats.output_spikes_written = self
                    .stats
                    .output_spikes_written
                    .saturating_add(result.output_spikes_written as u64);
                self.stats.dropped_spikes = self
                    .stats
                    .dropped_spikes
                    .saturating_add(result.dropped_spikes_count as u64);

                Ok(RuntimeBatchReport {
                    batch_result: result,
                    output_spikes: self.cached_output_spikes.clone(),
                    output_spike_counts: self.cached_output_spike_counts.clone(),
                    tick_base,
                    ticks_executed,
                })
            }
            Err(err) => {
                self.stats.compute_errors = self.stats.compute_errors.saturating_add(1);
                self.state = RuntimeState::Faulted;
                Err(RuntimeError::Compute(err))
            }
        }
    }

    /// Coordinates a batch run without any input signals.
    pub fn run_empty_batch(&mut self) -> Result<RuntimeBatchReport, RuntimeError> {
        let zeroed_counts = vec![0; self.config.sync_batch_ticks as usize];
        let input = RuntimeBatchInput {
            input_bitmask: None,
            incoming_spikes: None,
            incoming_spike_counts: &zeroed_counts,
        };
        self.run_batch(input)
    }

    /// Shutdown the runtime orchestrator and clean up engine resources.
    pub fn shutdown(&mut self) -> Result<(), RuntimeError> {
        if self.state == RuntimeState::Stopped {
            return Ok(());
        }

        match self.engine.teardown() {
            Ok(()) => {
                self.state = RuntimeState::Stopped;
                Ok(())
            }
            Err(err) => {
                self.stats.compute_errors = self.stats.compute_errors.saturating_add(1);
                self.state = RuntimeState::Faulted;
                Err(RuntimeError::Compute(err))
            }
        }
    }

    /// Returns a snapshot of accumulated statistics.
    pub fn stats(&self) -> RuntimeStats {
        self.stats.clone()
    }

    /// Returns the current runtime lifecycle state.
    pub fn state(&self) -> RuntimeState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{remove_file, File};
    use std::io::Write;

    fn create_test_engine_and_path() -> (compute::ShardEngine, std::path::PathBuf) {
        use baker::{bake_local_shard, pack_local_shard_artifacts, LocalShardBakeInput};
        use boot::{bootstrap_local_shard_engine, LocalShardComputeInput};
        use types::MasterSeed;

        let neuron_types = vec![config::NeuronType {
            name: "TypeA".to_string(),
            membrane: config::MembraneParams {
                threshold: 1000,
                rest_potential: -70,
                leak_shift: 1,
                ahp_amplitude: 5,
            },
            timing: config::TimingParams {
                refractory_period: 2,
                synapse_refractory_period: 2,
            },
            signal: config::SignalParams {
                signal_propagation_length: 10,
            },
            homeostasis: config::HomeostasisParams {
                homeostasis_penalty: 0,
                homeostasis_decay: 10,
            },
            adaptive_leak: config::AdaptiveLeakParams {
                adaptive_leak_min_shift: 0,
                adaptive_leak_gain: 0,
                adaptive_mode: 0,
            },
            dopamine: config::DopamineParams {
                d1_affinity: 0,
                d2_affinity: 0,
            },
            gsop: config::GsopParams {
                gsop_potentiation: 1,
                gsop_depression: 1,
                initial_synapse_weight: 100,
                is_inhibitory: false,
                inertia_curve: vec![1, 1, 1, 1, 1, 1, 1, 1],
            },
            growth: config::GrowthParams {
                steering_fov_deg: 45.0,
                steering_radius_um: 10.0,
                steering_weight_inertia: 0.5,
                steering_weight_sensor: 0.5,
                steering_weight_jitter: 0.1,
                dendrite_radius_um: 5.0,
                growth_vertical_bias: 0.0,
                type_affinity: 1.0,
                dendrite_whitelist: vec![],
                sprouting_weight_distance: 1.0,
                sprouting_weight_power: 1.0,
                sprouting_weight_explore: 1.0,
                sprouting_weight_type: 1.0,
            },
            spontaneous: config::SpontaneousParams {
                spontaneous_firing_period_ticks: 0,
            },
        }];
        let layers = vec![config::LayerConfig {
            name: "L1".to_string(),
            height_pct: 1.0,
            density: 0.2,
            composition: vec![config::NeuronTypeDistribution {
                type_name: "TypeA".to_string(),
                share: 1.0,
            }],
        }];
        let shard_config = config::ShardConfig {
            meta: None,
            dimensions: config::ShardDimensions {
                w: 20,
                d: 20,
                h: 20,
            },
            settings: config::ShardSettings {
                ghost_capacity: 1024,
                prune_threshold: 0,
                max_sprouts: 8,
                night_interval_ticks: 100,
                save_checkpoints_interval_ticks: 1000,
            },
            layers,
            neuron_types,
            sockets: None,
            ports: None,
        };
        let input = LocalShardBakeInput {
            shard_config: &shard_config,
            master_seed: MasterSeed(42),
            voxel_size_um: 1.0,
        };
        let (artifacts, _) = bake_local_shard(&input).unwrap();
        let packed = pack_local_shard_artifacts(&artifacts).unwrap();

        let mut temp = std::env::temp_dir();
        let rand = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        temp.push(format!("local_unit_{}.axic", rand));
        {
            let mut f = File::create(&temp).unwrap();
            f.write_all(&packed).unwrap();
        }

        let compute_input = LocalShardComputeInput {
            archive_path: temp.clone(),
            backend_preference: compute::BackendPreference::Cpu,
            virtual_offset: 0,
            total_ghosts: 0,
        };
        let (engine, _) = bootstrap_local_shard_engine(&compute_input).unwrap();
        (engine, temp)
    }

    #[test]
    fn test_local_runtime_tick_overflow_unit() {
        let (engine, path) = create_test_engine_and_path();
        let config = LocalRuntimeConfig {
            sync_batch_ticks: 2,
            v_seg: 1,
            dopamine: 0,
            max_spikes_per_tick: 4,
            virtual_offset: 0,
            num_virtual_axons: 0,
            input_words_per_tick: 1,
            mapped_soma_ids: vec![0, 1],
        };
        let mut runtime = LocalRuntime::new(engine, config).unwrap();

        // Mutate private fields directly
        runtime.current_tick = u64::MAX - 1;

        let res = runtime.run_empty_batch();
        assert!(matches!(
            res,
            Err(RuntimeError::TickOverflow {
                current: _,
                sync: 2
            })
        ));

        let _ = remove_file(path);
    }
}
