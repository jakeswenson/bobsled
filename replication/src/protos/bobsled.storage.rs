#[derive(Clone, PartialEq, ::prost::Message)]
pub struct OwnerId {
    #[prost(string, tag="1")]
    pub value: std::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RecordId {
    #[prost(string, tag="1")]
    pub value: std::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Version {
    #[prost(int32, tag="1")]
    pub value: i32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ChangeId {
    #[prost(bytes, tag="1")]
    pub value: std::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StoredRecord {
    #[prost(message, optional, tag="1")]
    pub version: ::std::option::Option<Version>,
    #[prost(message, optional, tag="2")]
    pub payload: ::std::option::Option<::prost_types::Value>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Change {
    #[prost(message, optional, tag="1")]
    pub owner_id: ::std::option::Option<OwnerId>,
    #[prost(message, optional, tag="2")]
    pub record_id: ::std::option::Option<RecordId>,
    #[prost(message, optional, tag="3")]
    pub version: ::std::option::Option<Version>,
    #[prost(message, optional, tag="4")]
    pub payload: ::std::option::Option<::prost_types::Value>,
}
