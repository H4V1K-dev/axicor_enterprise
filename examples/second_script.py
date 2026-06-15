from axipy import ModelBuilder, Shard, Department

def build_hivemind():

    model = ModelBuilder(
        project_name="HiveMindMegacity",
        output_dir="./hivemind"
    )

    neuron = model.gnm_lib("ai/generic/neuron")
    gate   = model.gnm_lib("ai/generic/gate")
    memory = model.gnm_lib("ai/generic/memory")

    ####################################################################
    # L1 — SENSOR GRID
    ####################################################################

    sensors = Department.L1("SensorGrid")

    cameras = []

    for i in range(32):

        shard = Shard(
            f"CameraCluster_{i}",
            x=128,
            y=128,
            z=8
        )

        shard.add_layer(
            "Capture",
            height_pct=1.0,
            density=0.9
        )

        shard.add_population(
            "Capture",
            neuron,
            fraction=1.0
        )

        shard.add_input_port(
            "video",
            width=64,
            height=64
        )

        shard.add_socket(
            "frame_out",
            width=16,
            height=16
        )

        shard.add_socket(
            "sync",
            width=4,
            height=4
        )

        sensors.add_shard(shard)
        cameras.append(shard)

    ####################################################################
    # L2 — REGIONAL PROCESSORS
    ####################################################################

    regional = Department.L2("RegionalProcessors")

    processors = []

    for i in range(16):

        shard = Shard(
            f"Region_{i}",
            x=256,
            y=256,
            z=32
        )

        shard.add_layer(
            "Analysis",
            height_pct=0.7,
            density=0.8
        )

        shard.add_population(
            "Analysis",
            neuron,
            fraction=1.0
        )

        shard.add_layer(
            "Memory",
            height_pct=0.3,
            density=0.6
        )

        shard.add_population(
            "Memory",
            memory,
            fraction=1.0
        )

        shard.add_socket(
            "visual_in",
            width=16,
            height=16
        )

        shard.add_socket(
            "regional_out",
            width=16,
            height=16
        )

        shard.add_socket(
            "crosslink",
            width=8,
            height=8
        )

        regional.add_shard(shard)
        processors.append(shard)

    ####################################################################
    # L3 — HYPERCORE
    ####################################################################

    hyper = Department.L3("HyperCore")

    cores = []

    for i in range(8):

        shard = Shard(
            f"HyperCore_{i}",
            x=512,
            y=512,
            z=64
        )

        shard.add_layer(
            "Reasoning",
            height_pct=0.5,
            density=0.95
        )

        shard.add_population(
            "Reasoning",
            neuron,
            fraction=1.0
        )

        shard.add_layer(
            "Gating",
            height_pct=0.2,
            density=0.8
        )

        shard.add_population(
            "Gating",
            gate,
            fraction=1.0
        )

        shard.add_layer(
            "LongTerm",
            height_pct=0.3,
            density=0.7
        )

        shard.add_population(
            "LongTerm",
            memory,
            fraction=1.0
        )

        shard.add_socket(
            "core_in",
            width=32,
            height=32
        )

        shard.add_socket(
            "core_out",
            width=32,
            height=32
        )

        shard.add_socket(
            "mesh",
            width=16,
            height=16
        )

        hyper.add_shard(shard)
        cores.append(shard)

    ####################################################################
    # FULL SENSOR → REGIONAL
    ####################################################################

    for cam in cameras:

        for region in processors:

            cam.sockets["frame_out"].connect_to(
                f"RegionalProcessors.{region.name}.visual_in"
            )

    ####################################################################
    # REGIONAL RING
    ####################################################################

    for i in range(len(processors)):

        a = processors[i]
        b = processors[(i + 1) % len(processors)]

        a.sockets["crosslink"].connect_to(
            f"RegionalProcessors.{b.name}.crosslink"
        )

    ####################################################################
    # FULL REGIONAL → CORE
    ####################################################################

    for region in processors:

        for core in cores:

            region.sockets["regional_out"].connect_to(
                f"HyperCore.{core.name}.core_in"
            )

    ####################################################################
    # CORE MESH
    ####################################################################

    for a in cores:

        for b in cores:

            if a == b:
                continue

            a.sockets["mesh"].connect_to(
                f"HyperCore.{b.name}.mesh"
            )

    ####################################################################
    # SELF LOOPS
    ####################################################################

    for core in cores:

        core.sockets["core_out"].connect_to(
            f"HyperCore.{core.name}.core_in"
        )

    model.add_department(
        sensors,
        regional,
        hyper
    )

    model.build(dry_run=True)