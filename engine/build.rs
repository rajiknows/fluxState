use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    tonic_prost_build::configure()
        .compile_protos(&["../proto/flux.proto"], &["../proto"])
        .unwrap();
    Ok(())
}
