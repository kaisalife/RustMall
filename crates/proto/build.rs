fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(
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
