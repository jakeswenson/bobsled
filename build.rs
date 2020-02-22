fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("protos/storage.proto")?;
    tonic_build::compile_protos("protos/api.proto")?;
    Ok(())
}
