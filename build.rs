use std::io::Result;

fn main() -> Result<()> {
    prost_build::compile_protos(
        &["wire.proto", "perun-remote.proto"],
        &(["go-perun/wire/protobuf/", "src/wire/"]),
    )?;
    Ok(())
}
