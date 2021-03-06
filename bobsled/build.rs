use std::io;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  compile_protos("protos/storage.proto")?;
  compile_protos("protos/api.proto")?;
  compile_protos("protos/replication.proto")?;
  Ok(())
}

/// Copied from [`compile_protos`](tonic_build::compile_protos)
pub fn compile_protos(proto_path: impl AsRef<Path>) -> io::Result<()> {
  let proto_path: &Path = proto_path.as_ref();

  // directory the main .proto file resides in
  let proto_dir = proto_path
    .parent()
    .expect("proto file should reside in a directory");

  tonic_build::configure()
    .out_dir("src/protos/")
    .format(false)
    .compile(&[proto_path], &[proto_dir])?;

  Ok(())
}
