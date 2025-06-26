fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto/trading.proto"); // Rerun if .proto file changes
    tonic_build::configure()
        .build_server(true) // Generate server code
        .build_client(true) // Generate client code (optional, but can be useful for tests or if engine itself calls other gRPC services)
        // .out_dir("src/services/generated") // Output directory for generated Rust code - Let tonic_build use default OUT_DIR
        .compile(
            &["proto/trading.proto"], // Path to .proto files relative to engine crate root
            &["proto"], // Include path for .proto files
        )?;
    Ok(())
}
