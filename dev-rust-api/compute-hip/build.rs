fn main() {
    // Skip building HIP files if mock-gpu feature is active
    if std::env::var("CARGO_FEATURE_MOCK_GPU").is_ok() {
        return;
    }

    // Check if hipcc is available in PATH.
    // If not, panic to avoid silent build errors.
    let hipcc_found = std::process::Command::new("hipcc")
        .arg("--version")
        .output()
        .is_ok();

    if !hipcc_found {
        panic!("FATAL: hipcc compiler not found, and 'mock-gpu' feature is NOT enabled. Either install ROCm/HIP Toolkit or build with '--features mock-gpu'.");
    }

    cc::Build::new()
        .compiler("hipcc")
        .file("src/hip/physics.hip")
        .compile("axicor_hip");
    println!("cargo:rerun-if-changed=src/hip/");
}
