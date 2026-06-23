fn main() {
    // Skip building CUDA files if mock-gpu feature is active
    if std::env::var("CARGO_FEATURE_MOCK_GPU").is_ok() {
        return;
    }

    // Check if nvcc is available in PATH.
    // If not, panic to avoid silent build errors.
    let nvcc_found = std::process::Command::new("nvcc")
        .arg("--version")
        .output()
        .is_ok();

    if !nvcc_found {
        panic!("FATAL: nvcc compiler not found, and 'mock-gpu' feature is NOT enabled. Either install CUDA Toolkit or build with '--features mock-gpu'.");
    }

    cc::Build::new()
        .cuda(true)
        .file("src/cuda/physics.cu")
        .compile("axicor_cuda");
    println!("cargo:rerun-if-changed=src/cuda/");
}