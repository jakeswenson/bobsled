use bobsled::protos::replication::replication_api_client::ReplicationApiClient;

async fn send() -> Result<(), Box<dyn std::error::Error>> {
  let mut _client = ReplicationApiClient::connect("http://[::1]:50051").await?;

  Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  for _i in 0..5 {
    send().await?;
  }

  Ok(())
}
