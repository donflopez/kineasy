#![macro_use]
use serde_derive::*;
use serde_json;
use chrono::Utc;

use rusoto_kinesis::{CreateStreamInput, ListShardsInput, KinesisClient, PutRecordInput, Kinesis};
use rusoto_core::Region;
use bytes::Bytes;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TestExample {
    pub example: String
}

fn client() -> KinesisClient {
    KinesisClient::new(Region::Custom {
        name:"custom-reg".to_owned(),
        endpoint: "http://localhost:4568".to_owned()
    })
}

fn example_record() -> TestExample {
    TestExample {example: "hahahaha".to_owned()}
}

pub fn send_test_record () {
    client().put_record(PutRecordInput {
        data: Bytes::from(serde_json::to_vec(&example_record()).expect("Cannot serialize example record")),
        partition_key: Utc::now().to_rfc3339(),
        ..Default::default()
    }).sync();
}

pub fn create_test_stream () {
    client().create_stream(CreateStreamInput {
        shard_count: 5,
        stream_name: "kineasy_test_stream".to_owned(),
    }).sync();
}