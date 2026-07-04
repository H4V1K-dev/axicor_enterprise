const path = require('path');
const fs = require('fs');
const { execFileSync } = require('child_process');

const IS_WINDOWS = process.platform === 'win32';
const BAKER_BIN = path.resolve(__dirname, `../../target/debug/axi-baker${IS_WINDOWS ? '.exe' : ''}`);
const NODE_BIN = path.resolve(__dirname, `../../target/debug/axi-node${IS_WINDOWS ? '.exe' : ''}`);

if (!fs.existsSync(BAKER_BIN)) {
    console.error(`Error: axi-baker binary not found at ${BAKER_BIN}`);
    process.exit(1);
}
if (!fs.existsSync(NODE_BIN)) {
    console.error(`Error: axi-node binary not found at ${NODE_BIN}`);
    process.exit(1);
}

const thresholds = [10, 25, 50, 100];
const synapseWeights = [250, 500, 1000, 2000];
const spontaneousFiringPeriods = [0, 2, 5, 10];
const maxSpikesPerTicks = [100, 500, 1000];

const csvPath = path.resolve(__dirname, '../../artifacts/sweep_summary.csv');
const csvDir = path.dirname(csvPath);
if (!fs.existsSync(csvDir)) {
    fs.mkdirSync(csvDir, { recursive: true });
}

// Initialize CSV file
fs.writeFileSync(csvPath, 'threshold,initial_synapse_weight,spontaneous_firing_period_ticks,max_spikes_per_tick,generated,written,dropped,dropped_ratio,saturation,nonzero_ticks,wall_time,status\n');

function generateToml(threshold, weight, period) {
    return `[dimensions]
w = 20
d = 20
h = 20

[settings]
ghost_capacity = 1024
prune_threshold = 0
max_sprouts = 8
night_interval_ticks = 100
save_checkpoints_interval_ticks = 1000

[[neuron_types]]
name = "TypeA"
[neuron_types.membrane]
threshold = ${threshold}
rest_potential = -70
leak_shift = 1
ahp_amplitude = 5
[neuron_types.timing]
refractory_period = 2
fatigue_capacity = 255
[neuron_types.signal]
signal_propagation_length = 10
[neuron_types.homeostasis]
homeostasis_penalty = 0
homeostasis_decay = 10
[neuron_types.adaptive_leak]
adaptive_leak_min_shift = 0
adaptive_leak_gain = 0
adaptive_mode = 0
[neuron_types.dopamine]
d1_affinity = 0
d2_affinity = 0
[neuron_types.gsop]
gsop_potentiation = 1
gsop_depression = 1
initial_synapse_weight = ${weight}
is_inhibitory = false
inertia_curve = [1, 1, 1, 1, 1, 1, 1, 1]
[neuron_types.growth]
steering_fov_deg = 45.0
steering_radius_um = 10.0
steering_weight_inertia = 0.5
steering_weight_sensor = 0.5
steering_weight_jitter = 0.1
dendrite_radius_um = 5.0
growth_vertical_bias = 0.0
type_affinity = 1.0
dendrite_whitelist = []
sprouting_weight_distance = 1.0
sprouting_weight_power = 1.0
sprouting_weight_explore = 1.0
sprouting_weight_type = 1.0
[neuron_types.spontaneous]
spontaneous_firing_period_ticks = ${period}

[[layers]]
name = "L1"
height_pct = 1.0
density = 0.2
[[layers.composition]]
type_name = "TypeA"
share = 1.0
`;
}

console.log('Starting Parameter Sweep Grid...');
console.log(`Total runs scheduled: ${thresholds.length * synapseWeights.length * spontaneousFiringPeriods.length * maxSpikesPerTicks.length}`);

let count = 0;
const tempToml = path.resolve(__dirname, 'temp_sweep.toml');
const tempAxic = path.resolve(__dirname, 'temp_sweep.axic');

try {
    for (const threshold of thresholds) {
        for (const weight of synapseWeights) {
            for (const period of spontaneousFiringPeriods) {
                for (const maxSpikes of maxSpikesPerTicks) {
                    count++;
                    const toml = generateToml(threshold, weight, period);
                    fs.writeFileSync(tempToml, toml);

                    // Bake
                    execFileSync(BAKER_BIN, [
                        'bake-local',
                        '--shard', tempToml,
                        '--out', tempAxic,
                        '--seed', '42',
                        '--voxel-size-um', '1.0',
                        '--force',
                        '--json'
                    ], { stdio: 'ignore' });

                    // Run
                    const stdout = execFileSync(NODE_BIN, [
                        'run-local',
                        '--archive', tempAxic,
                        '--ticks', '35',
                        '--batch-ticks', '10',
                        '--max-spikes-per-tick', String(maxSpikes),
                        '--backend', 'cpu',
                        '--json'
                    ], { stdio: 'pipe', encoding: 'utf8' });

                    // Parse
                    const summary = JSON.parse(stdout.trim());
                    const gen = summary.total_generated_spikes || 0;
                    const writ = summary.total_output_spikes_written || 0;
                    const drop = summary.total_dropped_spikes || 0;
                    const wall = summary.wall_time_us || 0;
                    const nzTicks = summary.nonzero_output_ticks || 0;

                    const dropRatio = gen > 0 ? (drop / gen) : 0;
                    const sat = gen > 0 ? (writ / gen) : 0;

                    let status = 'healthy-ish';
                    if (gen === 0) {
                        status = 'silent';
                    } else if (dropRatio > 0.5) {
                        status = 'overheated';
                    } else if (sat < 0.2) {
                        status = 'bottleneck';
                    }

                    // Append row
                    fs.appendFileSync(
                        csvPath,
                        `${threshold},${weight},${period},${maxSpikes},${gen},${writ},${drop},${dropRatio},${sat},${nzTicks},${wall},${status}\n`
                    );

                    if (count % 20 === 0 || count === 192) {
                        console.log(`[Progress] Compiled ${count}/192 configurations.`);
                    }
                }
            }
        }
    }
    console.log(`Sweep completed successfully! Output saved to: ${csvPath}`);
} catch (err) {
    console.error('Sweep execution failed:', err);
} finally {
    // Cleanup temporary files
    if (fs.existsSync(tempToml)) fs.unlinkSync(tempToml);
    if (fs.existsSync(tempAxic)) fs.unlinkSync(tempAxic);
}
