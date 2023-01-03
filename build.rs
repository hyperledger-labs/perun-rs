use std::io::Result;

fn main() -> Result<()> {
    prost_build::compile_protos(
        &["go-perun/wire/protobuf/wire.proto"],
        &([] as [&'static str; 0]),
    )?;
    Ok(())
}
