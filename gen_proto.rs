fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate product proto
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .out_dir("services/product-service/src/interface")
        .compile(&["proto/product.proto"], &["proto"])?;

    // Generate order proto
    tonic_build