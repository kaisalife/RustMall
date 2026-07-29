fn main() -> Result<(), Box<dyn std::error::Error>> {
    // proto 文件位于 crate 目录之外，需显式声明变更监听
    println!("cargo:rerun-if-changed=../../proto");

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &[
                "../../proto/v1/auth.proto",
                "../../proto/v1/email.proto",
                "../../proto/v1/product.proto",
                "../../proto/v1/order.proto",
                "../../proto/v1/inventory.proto",
                "../../proto/v1/payment.proto",
                "../../proto/v2/auth.proto",
            ],
            &["../../proto"],
        )?;
    Ok(())
}
