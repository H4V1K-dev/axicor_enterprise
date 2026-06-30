//! Differential comparison helpers for result and snapshot verification.

use crate::outcome::HarnessErrorKind;
use compute_api::BatchResult;

/// Compares deterministic fields of two `BatchResult` structures.
pub fn compare_results(
    fixture_name: &str,
    tick: u64,
    expected: &BatchResult,
    actual: &BatchResult,
) -> Result<(), HarnessErrorKind> {
    if expected.ticks_executed != actual.ticks_executed {
        return Err(HarnessErrorKind::ResultMismatch {
            fixture_name: String::from(fixture_name),
            tick,
            field: "ticks_executed",
            expected: format!("{}", expected.ticks_executed),
            actual: format!("{}", actual.ticks_executed),
        });
    }
    if expected.generated_spikes_count != actual.generated_spikes_count {
        return Err(HarnessErrorKind::ResultMismatch {
            fixture_name: String::from(fixture_name),
            tick,
            field: "generated_spikes_count",
            expected: format!("{}", expected.generated_spikes_count),
            actual: format!("{}", actual.generated_spikes_count),
        });
    }
    if expected.output_spikes_written != actual.output_spikes_written {
        return Err(HarnessErrorKind::ResultMismatch {
            fixture_name: String::from(fixture_name),
            tick,
            field: "output_spikes_written",
            expected: format!("{}", expected.output_spikes_written),
            actual: format!("{}", actual.output_spikes_written),
        });
    }
    if expected.dropped_spikes_count != actual.dropped_spikes_count {
        return Err(HarnessErrorKind::ResultMismatch {
            fixture_name: String::from(fixture_name),
            tick,
            field: "dropped_spikes_count",
            expected: format!("{}", expected.dropped_spikes_count),
            actual: format!("{}", actual.dropped_spikes_count),
        });
    }
    Ok(())
}

/// Compares output spikes and counts per-tick.
#[allow(clippy::too_many_arguments)]
pub fn compare_output_spikes(
    fixture_name: &str,
    tick_base: u64,
    ticks: u32,
    max_spikes: u32,
    expected_spikes: &[u32],
    expected_counts: &[u32],
    actual_spikes: &[u32],
    actual_counts: &[u32],
) -> Result<(), HarnessErrorKind> {
    for t in 0..ticks as usize {
        let tick = tick_base + t as u64;
        if expected_counts[t] != actual_counts[t] {
            return Err(HarnessErrorKind::ResultMismatch {
                fixture_name: String::from(fixture_name),
                tick,
                field: "output_spike_counts",
                expected: format!("{}", expected_counts[t]),
                actual: format!("{}", actual_counts[t]),
            });
        }
        let count = expected_counts[t] as usize;
        let start = t * max_spikes as usize;
        for s in 0..count {
            let offset = start + s;
            if expected_spikes[offset] != actual_spikes[offset] {
                return Err(HarnessErrorKind::ResultMismatch {
                    fixture_name: String::from(fixture_name),
                    tick,
                    field: "output_spikes",
                    expected: format!("{}", expected_spikes[offset]),
                    actual: format!("{}", actual_spikes[offset]),
                });
            }
        }
    }
    Ok(())
}

/// Compares `.state` and `.axons` binary snapshots byte-by-byte.
pub fn compare_snapshots(
    fixture_name: &str,
    tick: u64,
    expected_state: &[u8],
    expected_axons: &[u8],
    actual_state: &[u8],
    actual_axons: &[u8],
) -> Result<(), HarnessErrorKind> {
    if expected_state.len() != actual_state.len() {
        return Err(HarnessErrorKind::FacadeError(format!(
            "Expected state snapshot size {} does not match actual {}",
            expected_state.len(),
            actual_state.len()
        )));
    }
    if expected_axons.len() != actual_axons.len() {
        return Err(HarnessErrorKind::FacadeError(format!(
            "Expected axons snapshot size {} does not match actual {}",
            expected_axons.len(),
            actual_axons.len()
        )));
    }

    for (offset, (&exp, &act)) in expected_state.iter().zip(actual_state.iter()).enumerate() {
        if exp != act {
            return Err(HarnessErrorKind::SnapshotMismatch {
                fixture_name: String::from(fixture_name),
                tick,
                plane: "state_blob",
                offset,
                expected: exp,
                actual: act,
            });
        }
    }

    for (offset, (&exp, &act)) in expected_axons.iter().zip(actual_axons.iter()).enumerate() {
        if exp != act {
            return Err(HarnessErrorKind::SnapshotMismatch {
                fixture_name: String::from(fixture_name),
                tick,
                plane: "axons_blob",
                offset,
                expected: exp,
                actual: act,
            });
        }
    }

    Ok(())
}
