#![cfg(all(feature = "cpu", feature = "mvp-cpu-replay", feature = "baker-probe"))]

use std::fs::File;
use std::path::PathBuf;

#[test]
fn run_growth_v2_mvp_extraction_inventory() {
    println!("=== Starting Growth v2 MVP Extraction Inventory ===");

    let mut artifacts_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    artifacts_dir.pop(); // to crates
    artifacts_dir.pop(); // to AxiEngine
    artifacts_dir.pop(); // to workflow
    artifacts_dir.push("artifacts");
    std::fs::create_dir_all(&artifacts_dir).unwrap();

    let inventory_path = artifacts_dir.join("growth_v2_mvp_extraction_inventory.json");
    let inventory = serde_json::json!({
        "status": "completed_source_audit",
        "description": "Growth v2 MVP Extraction Inventory",
        "legacy_files_audited": [
            "axicor-master/axicor-baker/src/bake/cone_tracing.rs",
            "axicor-master/axicor-baker/src/bake/axon_growth.rs",
            "axicor-master/axicor-baker/src/bake/spatial_grid.rs",
            "axicor-master/axicor-baker/src/bake/dendrite_connect.rs",
            "axicor-master/axicor-baker/src/bake/sprouting.rs"
        ],
        "inventory": {
            "attraction_mechanics": "cone_tracing with FOV and type_affinity",
            "steering_weights": "blend of v_global, v_attract, and v_noise",
            "state_machine": "GrowthEvent: Advanced, TargetReached, Stagnated, OutOfBounds",
            "ghost_packets": "inter-shard axon continuation",
            "uniqueness": "single synapse per target/source pair",
            "radius_gate": "legacy dendrite connect uses cell-neighborhood scan without final exact Euclidean radius check",
            "whitelist_policy": "legacy external axons with soma_idx == usize::MAX bypass whitelists; Growth v2 must not port this blindly"
        }
    });

    let file = File::create(&inventory_path).unwrap();
    serde_json::to_writer_pretty(file, &inventory).unwrap();
    println!("Wrote {}", inventory_path.display());
    println!("=== Growth v2 MVP Extraction Inventory Complete ===");
}
