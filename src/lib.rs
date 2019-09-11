//! # Kineasy
//!
//! Kineasy is a library that helps you to use AWS Kinesis service. 
//! It very opinionated and focused on performance.
//! With this library you can consume a stream with multiple shards without caring about
//! orchestrating them, you will get a stream of records from multiple shards.
//! 
//! You can also enable auto checkpointing so you can safely restart the service in needed,
//! this checkpoint feature writes checkpoints to disk but you can implement your own
//! writting feature.
//!
//! ## Example
//!
//! ```rust
//!# extern crate kineasy;
//!# extern crate dockers;
//!
//!# use dockers::{Container, Image, containers::{ContainerConfig, HostConfig, PortBinding}};
//! use kineasy::{Kineasy, Region, shard::ShardIterator, Record};
//! use futures_util::stream::StreamExt;
//! use futures::future;
//!# use std::{time, collections::HashMap, thread};
//!# use kineasy_test_utils::*;
//! use tokio;
//!
//! fn main () {
//!#     let localstack = "localstack/localstack".to_owned();
//!#     let img = Image::pull(localstack.clone(), None)
//!#         .expect("Cannot pull image");
//!# 
//!#     let mut published_ports = HashMap::new();
//!# 
//!#     published_ports.insert(
//!#         "4568/tcp".to_owned(),
//!#         vec![PortBinding {
//!#             HostPort: "4568".to_owned(),
//!#             HostIp: "0.0.0.0".to_owned()
//!#         }],
//!#     );
//!#     
//!#     let cont_conf = ContainerConfig {
//!#         Image: localstack.clone(),
//!#         HostConfig: HostConfig {
//!#             PortBindings: Some(published_ports),
//!#             ..Default::default()
//!#         },
//!#         ..Default::default()
//!#     };
//!#
//!#     let cont = Container::new(None, Some(localstack.clone()))
//!#         .create(Some("kineasy_test".to_owned()), Some(cont_conf))
//!#         .expect("Cannot create container");
//!# 
//!#     cont.start().unwrap();
//!# 
//!#     thread::sleep(time::Duration::from_millis(10000));
//! 
//!     let run = tokio::runtime::Runtime::new().unwrap();
//!     
//!#     create_test_stream();
//!# 
//!#     thread::sleep(time::Duration::from_millis(3000));
//! 
//!     run.block_on(async {
//! 
//!         let kns = Kineasy::new(Region::Custom {
//!             name: "custom-region".to_owned(),
//!             endpoint: "http://localhost:4568".to_owned()
//!         }, "kineasy_test_stream".to_owned(), ShardIterator::Latest);
//! 
//!         let stream = kns.stream().await;
//! 
//!#         send_test_record();
//! 
//!         stream
//!             .take(1)
//!             .map(|r: Record| {
//!                let r: TestExample = serde_json::from_str(&String::from_utf8(r.data.to_vec())
//!                    .expect("Cannot parse this."))
//!                    .expect("Cannot parse json");
//!                r
//!            }).for_each(|parsed| {
//!                assert_eq!(TestExample {
//!                    example: "example".to_owned()
//!                }, parsed);
//!
//!                future::ready(())
//!            }).await;
//!     });
//!
//!#     cont.remove();
//!#     img.remove();
//! }
//! ```

pub mod stream;
pub mod checkpoint;
pub mod shard;

pub use stream::Kineasy;
pub use rusoto_core::{Region};
pub use rusoto_kinesis::Record;
