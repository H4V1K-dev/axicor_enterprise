from axipy import ModelBuilder, Shard, Department

def build_cube():

    model = ModelBuilder(
        project_name="QuantumCube",
        output_dir="./cube"
    )

    neuron = model.gnm_lib("generic/neuron")

    ####################################################################
    # L1
    ####################################################################

    l1 = Department.L1("Foundation")

    base_a = Shard("Base_A", x=48, y=48, z=8)
    base_a.add_layer("Core", height_pct=1.0, density=0.8)
    base_a.add_population("Core", neuron, fraction=1.0)

    base_a.add_socket(
        "upstream",
        width=12,
        height=12,
        placement="top"
    )

    base_a.add_socket(
        "mesh",
        width=8,
        height=8,
        placement="top"
    )

    base_b = Shard("Base_B", x=48, y=48, z=8)
    base_b.add_layer("Core", height_pct=1.0, density=0.8)
    base_b.add_population("Core", neuron, fraction=1.0)

    base_b.add_socket(
        "upstream",
        width=12,
        height=12,
        placement="top"
    )

    base_b.add_socket(
        "mesh",
        width=8,
        height=8,
        placement="top"
    )

    l1.add_shard(base_a, base_b)

    ####################################################################
    # L2
    ####################################################################

    l2 = Department.L2("Aggregation")

    agg_a = Shard("Agg_A", x=56, y=56, z=12)
    agg_a.add_layer("Fusion", height_pct=1.0, density=0.7)
    agg_a.add_population("Fusion", neuron, fraction=1.0)

    agg_a.add_socket(
        "downstream",
        width=12,
        height=12,
        placement="bottom"
    )

    agg_a.add_socket(
        "upstream",
        width=14,
        height=14,
        placement="top"
    )

    agg_a.add_socket(
        "cross",
        width=10,
        height=10,
        placement="top"
    )

    agg_b = Shard("Agg_B", x=56, y=56, z=12)
    agg_b.add_layer("Fusion", height_pct=1.0, density=0.7)
    agg_b.add_population("Fusion", neuron, fraction=1.0)

    agg_b.add_socket(
        "downstream",
        width=12,
        height=12,
        placement="bottom"
    )

    agg_b.add_socket(
        "upstream",
        width=14,
        height=14,
        placement="top"
    )

    agg_b.add_socket(
        "cross",
        width=10,
        height=10,
        placement="top"
    )

    l2.add_shard(agg_a, agg_b)

    ####################################################################
    # L3
    ####################################################################

    l3 = Department.L3("Routing")

    router = Shard("Router", x=64, y=64, z=16)

    router.add_layer(
        "Routing",
        height_pct=1.0,
        density=0.9
    )

    router.add_population(
        "Routing",
        neuron,
        fraction=1.0
    )

    router.add_socket(
        "south",
        width=16,
        height=16,
        placement="bottom"
    )

    router.add_socket(
        "north",
        width=16,
        height=16,
        placement="top"
    )

    router.add_socket(
        "east",
        width=10,
        height=10,
        placement="top"
    )

    router.add_socket(
        "west",
        width=10,
        height=10,
        placement="top"
    )

    l3.add_shard(router)

    ####################################################################
    # L4
    ####################################################################

    l4 = Department.L4("Compute")

    compute = []

    for i in range(8):

        shard = Shard(
            f"Compute_{i}",
            x=48,
            y=48,
            z=20
        )

        shard.add_layer(
            "Inference",
            height_pct=0.7,
            density=0.85
        )

        shard.add_population(
            "Inference",
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
            neuron,
            fraction=1.0
        )

        shard.add_socket(
            "input",
            width=12,
            height=12,
            placement="bottom"
        )

        shard.add_socket(
            "output",
            width=12,
            height=12,
            placement="top"
        )

        shard.add_socket(
            "mesh",
            width=8,
            height=8,
            placement="top"
        )

        compute.append(shard)

    l4.add_shard(*compute)

    ####################################################################
    # L5
    ####################################################################

    l5 = Department.L5("Consensus")

    consensus = Shard(
        "Consensus",
        x=64,
        y=64,
        z=24
    )

    consensus.add_layer(
        "Voting",
        height_pct=1.0,
        density=0.95
    )

    consensus.add_population(
        "Voting",
        neuron,
        fraction=1.0
    )

    consensus.add_socket(
        "collect",
        width=16,
        height=16,
        placement="bottom"
    )

    consensus.add_socket(
        "decision",
        width=16,
        height=16,
        placement="top"
    )

    l5.add_shard(consensus)

    ####################################################################
    # L6
    ####################################################################

    l6 = Department.L6("Executive")

    executive = Shard(
        "Executive",
        x=56,
        y=56,
        z=28
    )

    executive.add_layer(
        "Planning",
        height_pct=1.0,
        density=0.9
    )

    executive.add_population(
        "Planning",
        neuron,
        fraction=1.0
    )

    executive.add_socket(
        "inbox",
        width=14,
        height=14,
        placement="bottom"
    )

    executive.add_socket(
        "orders",
        width=14,
        height=14,
        placement="top"
    )

    executive.add_socket(
        "feedback",
        width=10,
        height=10,
        placement="bottom"
    )

    l6.add_shard(executive)

    ####################################################################
    # L7
    ####################################################################

    l7 = Department.L7("MetaControl")

    meta = Shard(
        "Meta",
        x=40,
        y=40,
        z=32
    )

    meta.add_layer(
        "Governance",
        height_pct=1.0,
        density=1.0
    )

    meta.add_population(
        "Governance",
        neuron,
        fraction=1.0
    )

    meta.add_socket(
        "meta_in",
        width=10,
        height=10,
        placement="bottom"
    )

    meta.add_socket(
        "meta_out",
        width=10,
        height=10,
        placement="top"
    )

    l7.add_shard(meta)

    ####################################################################
    # L8
    ####################################################################

    l8 = Department.L8("Crown")

    crown = Shard(
        "Crown",
        x=32,
        y=32,
        z=40
    )

    crown.add_layer(
        "Finality",
        height_pct=1.0,
        density=1.0
    )

    crown.add_population(
        "Finality",
        neuron,
        fraction=1.0
    )

    crown.add_socket(
        "root_in",
        width=8,
        height=8,
        placement="bottom"
    )

    l8.add_shard(crown)

    ####################################################################
    # VERTICAL LINKS
    ####################################################################

    base_a.sockets["upstream"].connect_to(
        "Aggregation.Agg_A.downstream"
    )

    base_b.sockets["upstream"].connect_to(
        "Aggregation.Agg_B.downstream"
    )

    agg_a.sockets["upstream"].connect_to(
        "Routing.Router.south"
    )

    agg_b.sockets["upstream"].connect_to(
        "Routing.Router.south"
    )

    ####################################################################
    # ROUTER -> COMPUTE
    ####################################################################

    for node in compute:

        router.sockets["north"].connect_to(
            f"Compute.{node.name}.input"
        )

    ####################################################################
    # FULL COMPUTE MESH
    ####################################################################

    for a in compute:
        for b in compute:

            if a == b:
                continue

            a.sockets["mesh"].connect_to(
                f"Compute.{b.name}.mesh"
            )

    ####################################################################
    # COMPUTE -> CONSENSUS
    ####################################################################

    for node in compute:

        node.sockets["output"].connect_to(
            "Consensus.Consensus.collect"
        )

    ####################################################################
    # UPPER STACK
    ####################################################################

    consensus.sockets["decision"].connect_to(
        "Executive.Executive.inbox"
    )

    executive.sockets["orders"].connect_to(
        "MetaControl.Meta.meta_in"
    )

    meta.sockets["meta_out"].connect_to(
        "Crown.Crown.root_in"
    )

    ####################################################################
    # FEEDBACK
    ####################################################################

    executive.sockets["feedback"].connect_to(
        "Routing.Router.west"
    )

    model.add_department(
        l1,
        l2,
        l3,
        l4,
        l5,
        l6,
        l7,
        l8
    )

    model.build(dry_run=True)