use bobsled::protos::journal_api_client::JournalApiClient;
use bobsled::protos::{CreateRequest, OwnerId};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    async fn send() -> Result<(), Box<dyn std::error::Error>> {
        let mut client = JournalApiClient::connect("http://[::1]:50051").await?;
        let value: String = promptly::prompt_opt("name")
            .unwrap()
            .unwrap_or_else(String::default);

        let request = tonic::Request::new(CreateRequest {
            owner_id: Some(OwnerId { value }),
            payload: None,
        });

        let response = client.create(request).await?;
        println!("RESPONSE={:?}", response);
        Ok(())
    }

    for i in 1..5 {
        send().await?;
    }

    Ok(())
}
