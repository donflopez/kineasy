#![feature(async_await, async_closure)]
use rusoto_kinesis::GetRecordsOutput;
use rusoto_core::Region;
use rusoto_kinesis::{
    Kinesis, KinesisClient,
    ListShardsInput, Record, Shard
};
use std::{sync::{Arc, RwLock}, time::Duration, collections::HashMap};
use futures::{future::{join_all}};
use tokio::{prelude::*, future, sync::mpsc};

use crate::checkpoint::Checkpointing;
use crate::shard::{ShardIterator, KinesisShard};

pub struct Kineasy
{
    running: Arc<RwLock<bool>>,
    region: Region,
    stream: String,
    cp: Checkpointing,
    client: Arc<KinesisClient>,
    iterator_type: ShardIterator,
    sender: mpsc::Sender<Record>,
    receiver: mpsc::Receiver<Record>,
    shards: HashMap<String, KinesisShard>,
}

impl Kineasy
{
    pub fn new (region: Region, stream: String, iterator_type: ShardIterator) -> Self {
        let (sender, receiver) = mpsc::channel::<Record>(100);

        Kineasy {
            client: Arc::new(KinesisClient::new(region.clone())),
            cp: Checkpointing::default(),
            shards: HashMap::new(),
            running: Arc::new(RwLock::new(false)),
            iterator_type,
            receiver,
            sender,
            region,
            stream,
        }
    }

    pub fn enable_checkpoint(&mut self, interval: Duration, file: Option<String>) {
        // TODO: move this logic to Checkpointing module
        self.cp.file = file;
        self.cp.active = true;
        self.cp.interval = interval;
    }

    fn list_shards(&mut self) -> Vec<Shard> {
        self.client.list_shards(
            ListShardsInput {
                stream_name: Some(self.stream.clone()),
                ..ListShardsInput::default()
            })
            .sync().expect("List shards failed.").shards.expect("No shards available.")
    }

    fn create_shards(&mut self) {
        for shard in self.list_shards().into_iter() {
            let s = KinesisShard::from_latest(self.region.clone(), self.stream.clone(), shard.shard_id.clone(), self.running.clone());
            self.shards.insert(shard.shard_id, s);
        }
    }

    pub async fn stream(mut self) -> mpsc::Receiver<Record> {
        *self.running.write().unwrap() = true;

        for shard in self.list_shards().into_iter() {
            let s = KinesisShard::from_latest(self.region.clone(), self.stream.clone(), shard.shard_id.clone(), self.running.clone());
            self.shards.insert(shard.shard_id, s);
        }

        let shards = self.shards.clone();
        let sender = self.sender.clone();

        let running = self.running.clone();
        tokio::spawn(async move {
            join_all(shards.values().map(|shard| {
                let s = shard.clone();

                let sender = sender.clone();
                s.take_while(|_| future::ready(*running.read().unwrap())).for_each(move |l: GetRecordsOutput| {
                    let mut sender = sender.clone();

                    for r in l.records {
                        let _ = sender.try_send(r);
                    }

                    future::ready(())
                })
            }).collect::<Vec<_>>()).await;
        });

        self.receiver
    }
}