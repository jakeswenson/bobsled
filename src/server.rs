use std::sync::Mutex;

use futures::channel::oneshot::Sender;
use sled::Db;
use sled::Transactional;
use tonic::codegen::Arc;
use tonic::{transport::Server, Request, Response, Status};

use bobsled::protos::journal_api_server::{JournalApi, JournalApiServer};
use bobsled::protos::{CreateRequest, CreateResponse, RecordId};
use prost::Message;

#[derive(Debug)]
pub struct MyGreeter {
    db: Db,
}

#[tonic::async_trait]
impl JournalApi for MyGreeter {
    async fn create(
        &self,
        request: Request<CreateRequest>, // Accept request of type HelloRequest
    ) -> Result<Response<CreateResponse>, Status> {
        // Return an instance of type HelloReply
        println!("Got a request: {:?}", request);
        let request = request.into_inner();
        let owner_id: String = request.owner_id.unwrap().value;
        let resource_id = uuid::Uuid::new_v4().to_string();

        let journal = self.db.open_tree(b"journal").unwrap();
        let resources = self.db.open_tree(b"resources").unwrap();

        let payload: Option<prost_types::Struct> = request.payload;

        let payload_bytes = payload.map_or(Vec::default(), |payload| {
            let mut vec: Vec<u8> = Vec::with_capacity(payload.encoded_len());
            payload.encode(&mut vec).unwrap();
            vec
        });

        // Use write-only transactions as a write_batch:
        (&resources, &journal)
            .transaction(|(resources, journal)| {
                let key: Vec<u8> = owner_id
                    .as_bytes()
                    .iter()
                    .chain(b"/")
                    .chain(resource_id.as_bytes())
                    .cloned()
                    .collect();

                resources.insert(key, payload_bytes.clone())?;

                let id = self.db.generate_id()?;
                journal.insert(&id.to_be_bytes(), b"dogs")?;
                Ok(())
            })
            .unwrap();

        let reply = CreateResponse {
            record_id: Some(RecordId {
                value: resource_id.to_string(),
            }),
        };

        Ok(Response::new(reply)) // Send back our formatted greeting
    }
}

fn signal(tx: Arc<Mutex<Option<Sender<()>>>>) {
    if let Ok(mut tx_option) = tx.lock() {
        tx_option.take().unwrap().send(()).unwrap()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;

    let config = sled::Config::default()
        .path("dbs/data.sled".to_owned())
        .temporary(true)
        .cache_capacity(10_000_000_000)
        .flush_every_ms(Some(1000))
        .snapshot_after_ops(100_000);

    let db = config.open()?;
    let greeter = MyGreeter { db };

    use futures::channel::oneshot::{Receiver, Sender};
    use futures::future::FutureExt;
    let (tx, rx): (Sender<()>, Receiver<()>) = futures::channel::oneshot::channel::<()>();

    let tx = Arc::new(Mutex::new(Some(tx)));

    ctrlc::set_handler(move || {
        let tx = tx.clone();
        signal(tx)
    })
    .unwrap();

    println!("Running server...");
    Server::builder()
        .add_service(JournalApiServer::new(greeter))
        .serve_with_shutdown(addr, rx.map(|result| result.unwrap()))
        .await
        .unwrap();

    println!("Shutting down!");

    Ok(())
}
