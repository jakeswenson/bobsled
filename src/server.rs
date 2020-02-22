use std::sync::Mutex;

use futures::channel::oneshot::Sender;
use prost::{EncodeError, Message};
use sled::Transactional;
use sled::{abort, Db, TransactionalTree, Tree};
use tonic::codegen::Arc;
use tonic::{transport::Server, Request, Response, Status};

use bobsled::protos::api::{
    journal_api_server::{JournalApi, JournalApiServer},
    ChangesReply, CreateRequest, CreateResponse, GetReply, GetRequest, ReadChangesRequest,
    UpdateReply, UpdateRequest,
};
use bobsled::protos::storage::{Change, ChangeId, OwnerId, RecordId, StoredRecord, Version};

#[derive(Debug)]
pub struct BobsledJournalApi {
    db: Db,
}

impl BobsledJournalApi {
    fn make_storage_key(owner_id: &OwnerId, record_id: &RecordId) -> Vec<u8> {
        owner_id
            .value
            .as_bytes()
            .iter()
            .chain(b"/")
            .chain(record_id.value.as_bytes())
            .cloned()
            .collect()
    }

    fn encode_proto<T: Message>(proto: T) -> Result<Vec<u8>, EncodeError> {
        let mut storage_bytes: Vec<u8> = Vec::with_capacity(proto.encoded_len());
        proto.encode(&mut storage_bytes).map(|_| storage_bytes)
    }
}

#[tonic::async_trait]
impl JournalApi for BobsledJournalApi {
    async fn create(
        &self,
        request: Request<CreateRequest>,
    ) -> Result<Response<CreateResponse>, Status> {
        println!("Got a request: {:?}", request);
        let request = request.into_inner();
        let owner_id: OwnerId = request.owner_id.expect("owner_id is required");
        let record_id = RecordId {
            value: uuid::Uuid::new_v4().to_simple().to_string(),
        };

        let version = Version::default();

        let storage_key = BobsledJournalApi::make_storage_key(&owner_id, &record_id);

        let payload: Option<prost_types::Value> = request.payload;

        let storage_record = StoredRecord {
            version: Some(version.clone()),
            payload: payload.clone(),
        };

        let storage_bytes =
            BobsledJournalApi::encode_proto(storage_record).expect("Encode proto failure");

        let change = Change {
            owner_id: Some(owner_id),
            record_id: Some(record_id.clone()),
            version: Some(version),
            payload,
        };

        let change_bytes: Vec<u8> =
            BobsledJournalApi::encode_proto(change).expect("Encode Changes proto failure");

        let journal = self
            .db
            .open_tree(b"journal")
            .expect("Unable to get journal tree");
        let resources = self
            .db
            .open_tree(b"resources")
            .expect("Unablt to get resources tree");

        // Use write-only transactions as a write_batch:
        (&resources, &journal)
            .transaction(|(resources, journal)| {
                resources.insert(storage_key.clone(), storage_bytes.clone())?;

                let id = self.db.generate_id()?;
                journal.insert(&id.to_be_bytes(), change_bytes.clone())?;
                Ok(id)
            })
            .expect("Unable to complete transaction");

        let reply = CreateResponse {
            record_id: Some(record_id),
        };

        Ok(Response::new(reply))
    }

    async fn get(&self, request: Request<GetRequest>) -> Result<Response<GetReply>, Status> {
        let get_request = request.into_inner();
        let owner_id = get_request.owner_id.unwrap();
        let record_id = get_request.record_id.unwrap();

        let storage_key = BobsledJournalApi::make_storage_key(&owner_id, &record_id);

        let resources = self.db.open_tree(b"resources").unwrap();

        let result = resources
            .get(storage_key)
            .unwrap()
            .map(|ivec| StoredRecord::decode(ivec.as_ref()).unwrap());

        let reply = GetReply {
            owner_id: Some(owner_id),
            record_id: Some(record_id),
            record: result,
        };

        Ok(Response::new(reply))
    }

    async fn update(
        &self,
        request: Request<UpdateRequest>,
    ) -> Result<Response<UpdateReply>, Status> {
        println!("Got a request: {:?}", request);

        let update_request = request.into_inner();
        let owner_id: OwnerId = update_request.owner_id.expect("owner_id is required");
        let record_id = update_request.record_id.expect("record_id is required");
        let version = update_request.version.expect("version is required");
        let payload: Option<prost_types::Value> = update_request.payload;

        let expected_current_version: i32 = version.value;
        let new_version = Version {
            value: expected_current_version + 1,
        };

        let storage_key = BobsledJournalApi::make_storage_key(&owner_id, &record_id);

        let storage_record = StoredRecord {
            version: Some(new_version.clone()),
            payload: payload.clone(),
        };

        let storage_bytes = BobsledJournalApi::encode_proto(storage_record)
            .expect("Encode Storage Bytes proto failure");

        let change = Change {
            owner_id: Some(owner_id.clone()),
            record_id: Some(record_id.clone()),
            version: Some(new_version.clone()),
            payload,
        };

        let change_bytes =
            BobsledJournalApi::encode_proto(change).expect("Encode Changes proto failure");

        let journal: Tree = self
            .db
            .open_tree(b"journal")
            .expect("Unable to get journal tree");

        let resources = self
            .db
            .open_tree(b"resources")
            .expect("Unable to get resources tree");

        // Use write-only transactions as a write_batch:
        let result: Option<u64> = Transactional::transaction(
            &(&resources, &journal),
            |(resources, journal): &(TransactionalTree, TransactionalTree)| {
                let current_record_bytes = resources.get(storage_key.clone())?;
                let current_version: i32 = current_record_bytes
                    .and_then(|vec| StoredRecord::decode(vec.as_ref()).ok())
                    .and_then(|record| record.version)
                    .map(|v| v.value)
                    .unwrap_or(-1);

                if expected_current_version != current_version {
                    resources.insert(storage_key.clone(), storage_bytes.clone())?;
                } else {
                    return abort(());
                }

                let id = self.db.generate_id()?;
                journal.insert(&id.to_be_bytes(), change_bytes.clone())?;
                Ok(id)
            },
        )
        .ok();

        result
            .map(|id| {
                println!("New Journal ID: {}", id);
                let reply = UpdateReply {
                    owner_id: Some(owner_id),
                    record_id: Some(record_id),
                    new_version: Some(new_version),
                };

                Ok(Response::new(reply))
            })
            .unwrap_or_else(|| Err(Status::aborted("Failed to update")))
    }

    async fn read_changes(
        &self,
        request: Request<ReadChangesRequest>,
    ) -> Result<Response<ChangesReply>, Status> {
        let read_changes_request = request.into_inner();
        let after_change_id: Option<ChangeId> = read_changes_request.after;
        let limit: i32 = read_changes_request.limit;

        let journal = self.db.open_tree(b"journal").unwrap();

        let changes: Vec<(ChangeId, Change)> = after_change_id
            .map(|id| journal.range(id.value..))
            .unwrap_or_else(|| journal.iter())
            .take(limit as usize)
            .map(|result| result.unwrap())
            .map(|(key, bytes)| {
                (
                    ChangeId {
                        value: key.to_vec(),
                    },
                    Change::decode(bytes.as_ref()).unwrap(),
                )
            })
            .collect();

        let reply = ChangesReply {
            last_change: changes.last().map(|(id, _)| id.clone()),
            changes: changes.iter().map(|(_, v)| v).cloned().collect(),
        };

        Ok(Response::new(reply))
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
    let greeter = BobsledJournalApi { db };

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
