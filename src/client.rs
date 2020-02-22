use prost_types::{value, Value};

use bobsled::protos::api::{
    journal_api_client::JournalApiClient, ChangesReply, CreateRequest, ReadChangesRequest,
};
use bobsled::protos::storage::{Change, OwnerId};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    async fn send() -> Result<(), Box<dyn std::error::Error>> {
        let mut client = JournalApiClient::connect("http://[::1]:50051").await?;
        let id: String = promptly::prompt_opt("name")
            .unwrap()
            .unwrap_or_else(String::default);

        let value: String = promptly::prompt_opt("value")
            .unwrap()
            .unwrap_or_else(String::default);

        let request = tonic::Request::new(CreateRequest {
            owner_id: Some(OwnerId { value: id }),
            payload: Some(Value {
                kind: Some(value::Kind::StringValue(value)),
            }),
        });

        let response = client.create(request).await?;
        println!("RESPONSE={:?}", response);

        let changes: tonic::Response<ChangesReply> = client
            .read_changes(tonic::Request::new(ReadChangesRequest {
                after: None,
                limit: 10,
            }))
            .await?;

        println!("RESPONSE={:?}", changes);

        let changes = changes.into_inner();

        let changes: Vec<Change> = changes.changes;

        println!("Changes({})", changes.len());

        Ok(())
    }

    for _i in 0..5 {
        send().await?;
    }

    Ok(())
}
