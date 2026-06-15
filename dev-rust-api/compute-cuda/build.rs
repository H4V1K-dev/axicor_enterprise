fn main() {
    cc::Build::new()
        .cuda(true)
        .file("src/cuda/physics.cu")
        .compile("axicor_cuda");
    println!("cargo:rerun-if-changed=src/cuda/");
}