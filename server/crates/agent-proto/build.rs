fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = "../../../proto";

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &[
                "../../../proto/common.proto",
                "../../../proto/auth.proto",
                "../../../proto/chat.proto",
                "../../../proto/session.proto",
            ],
            &[proto_root],
        )?;

    Ok(())
}
