fn main() -> Result<(), Box<dyn std::error::Error>> {
    // proto 文件位于 crate 目录之外，需显式声明变更监听
    println!("cargo:rerun-if-changed=../../proto");

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &[
                "../../proto/auth.proto",
                "../../proto/email.proto",
                "../../proto/product.proto",
                "../../proto/order.proto",
                "../../proto/inventory.proto",
                "../../proto/payment.proto",
            ],
            &["../../proto"],
        )?;
    Ok(())
}
