import os
import tomllib

def migrate_file(legacy_path, output_dir):
    with open(legacy_path, "rb") as f:
        data = tomllib.load(f)
    
    # Extract the first neuron type from the list
    nt = data["neuron_type"][0]
    
    name = nt["name"]
    threshold = nt["threshold"]
    rest_potential = nt["rest_potential"]
    leak_shift = nt["leak_shift"]
    homeostasis_penalty = nt["homeostasis_penalty"]
    spontaneous_firing_period_ticks = nt["spontaneous_firing_period_ticks"]
    initial_synapse_weight = nt["initial_synapse_weight"]
    gsop_potentiation = nt["gsop_potentiation"]
    gsop_depression = nt["gsop_depression"]
    homeostasis_decay = nt["homeostasis_decay"]
    refractory_period = nt["refractory_period"]
    
    # Map capacity = synapse_refractory_period (old_refractory = 15 -> capacity = 15)
    synapse_ref_period = nt["synapse_refractory_period"]
    fatigue_capacity = synapse_ref_period
    
    signal_propagation_length = nt["signal_propagation_length"]
    is_inhibitory = nt["is_inhibitory"]
    inertia_curve = nt["inertia_curve"]
    ahp_amplitude = nt["ahp_amplitude"]
    adaptive_leak_min_shift = nt["adaptive_leak_min_shift"]
    adaptive_leak_gain = nt["adaptive_leak_gain"]
    adaptive_mode = nt["adaptive_mode"]
    d1_affinity = nt["d1_affinity"]
    d2_affinity = nt["d2_affinity"]
    
    steering_fov_deg = nt["steering_fov_deg"]
    steering_radius_um = nt["steering_radius_um"]
    growth_vertical_bias = nt["growth_vertical_bias"]
    dendrite_radius_um = nt["dendrite_radius_um"]
    type_affinity = nt["type_affinity"]
    sprouting_weight_distance = nt["sprouting_weight_distance"]
    sprouting_weight_power = nt["sprouting_weight_power"]
    sprouting_weight_explore = nt["sprouting_weight_explore"]
    sprouting_weight_type = nt["sprouting_weight_type"]
    steering_weight_inertia = nt["steering_weight_inertia"]
    steering_weight_sensor = nt["steering_weight_sensor"]
    steering_weight_jitter = nt["steering_weight_jitter"]
    
    # Format according to the new config::NeuronType ABI format
    modernized_toml = f"""name = "{name}"

[membrane]
threshold = {threshold}
rest_potential = {rest_potential}
leak_shift = {leak_shift}
ahp_amplitude = {ahp_amplitude}

[timing]
refractory_period = {refractory_period}
fatigue_capacity = {fatigue_capacity}

[signal]
signal_propagation_length = {signal_propagation_length}

[homeostasis]
homeostasis_penalty = {homeostasis_penalty}
homeostasis_decay = {homeostasis_decay}

[adaptive_leak]
adaptive_leak_min_shift = {adaptive_leak_min_shift}
adaptive_leak_gain = {adaptive_leak_gain}
adaptive_mode = {adaptive_mode}

[dopamine]
d1_affinity = {d1_affinity}
d2_affinity = {d2_affinity}

[gsop]
gsop_potentiation = {gsop_potentiation}
gsop_depression = {gsop_depression}
initial_synapse_weight = {initial_synapse_weight}
is_inhibitory = {str(is_inhibitory).lower()}
inertia_curve = {list(inertia_curve)}

[growth]
steering_fov_deg = {steering_fov_deg}
steering_radius_um = {steering_radius_um}
steering_weight_inertia = {steering_weight_inertia}
steering_weight_sensor = {steering_weight_sensor}
steering_weight_jitter = {steering_weight_jitter}
dendrite_radius_um = {dendrite_radius_um}
growth_vertical_bias = {growth_vertical_bias}
type_affinity = {type_affinity}
dendrite_whitelist = []
sprouting_weight_distance = {sprouting_weight_distance}
sprouting_weight_power = {sprouting_weight_power}
sprouting_weight_explore = {sprouting_weight_explore}
sprouting_weight_type = {sprouting_weight_type}

[spontaneous]
spontaneous_firing_period_ticks = {spontaneous_firing_period_ticks}
"""

    output_path = os.path.join(output_dir, f"{name}.toml")
    with open(output_path, "w") as f_out:
        f_out.write(modernized_toml)
    print(f"Migrated {legacy_path} -> {output_path} (fatigue_capacity={fatigue_capacity})")

def main():
    lib_dir = "/home/alex/AI_Home/workflow/Axicor_Neuron-Lib"
    output_dir = os.path.join(lib_dir, "modernized")
    os.makedirs(output_dir, exist_ok=True)
    
    files = ["4.toml", "7.toml", "218.toml"]
    for file_name in files:
        path = os.path.join(lib_dir, file_name)
        if os.path.exists(path):
            migrate_file(path, output_dir)
        else:
            print(f"File not found: {path}")

if __name__ == "__main__":
    main()
