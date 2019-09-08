use rusoto_kinesis::{KinesisClient, Kinesis, GetShardIteratorInput, GetRecordsOutput, GetRecordsInput};
use rusoto_core::Region;
use futures::task::Context;
use futures::{Poll, Stream, FutureExt};
use past::future::Future as Fut;
use std::{pin::Pin, sync::{Arc, RwLock}};
use std::time::Duration;
use std::ops::Add;


#[derive(Clone)]
pub enum ShardIterator {
    TrimHorizon,
    Latest,
    AtSequenceNumber,
    AfterSequenceNumber,
    AtTimestamp
}

impl ShardIterator {
    fn to_owned(&self) -> String {
        match self {
            ShardIterator::TrimHorizon => "TRIM_HORIZON".to_owned(),
            ShardIterator::Latest => "LATEST".to_owned(),
            ShardIterator::AtSequenceNumber => "AT_SEQUENCE_NUMBER".to_owned(),
            ShardIterator::AfterSequenceNumber => "AFTER_SEQUENCE_NUMBER".to_owned(),
            ShardIterator::AtTimestamp => "AT_TIMESTAMP".to_owned(),
        }
    }
}

#[derive(Clone)]
pub struct KinesisShard {
    stream: String,
    pub(crate) shard_id: String,
    it: Option<String>,
    client: KinesisClient,
    timestamp: Option<String>,
    running: Arc<RwLock<bool>>,
    seq_number: Option<String>,
    iterator_type: ShardIterator
}

impl KinesisShard {
    pub fn new(region: Region, stream: String, shard_id: String, seq_number: Option<String>, timestamp: Option<String>, iterator_type: ShardIterator, running: Arc<RwLock<bool>>) -> Self {
        KinesisShard {
            stream,
            running,
            it: None,
            timestamp: timestamp,
            shard_id,
            seq_number,
            iterator_type,
            client: KinesisClient::new(region),
        }
    }

    pub fn from_timestamp(region: Region, stream: String, shard_id: String, timestamp: String, running: Arc<RwLock<bool>>) -> Self {
        KinesisShard::new(region, stream, shard_id, None, Some(timestamp), ShardIterator::AtTimestamp, running)
    }

    pub fn from_sequence(region: Region, stream: String, shard_id: String, seq_number: String, running: Arc<RwLock<bool>>) -> Self {
        KinesisShard::new(region, stream, shard_id, Some(seq_number), None, ShardIterator::AtSequenceNumber, running)
    }

    pub fn from_trim_horizon(region: Region, stream: String, shard_id: String, running: Arc<RwLock<bool>>) -> Self {
        KinesisShard::new(region, stream, shard_id, None, None, ShardIterator::TrimHorizon, running)
    }

    pub fn from_latest(region: Region, stream: String, shard_id: String, running: Arc<RwLock<bool>>) -> Self {
        KinesisShard::new(region, stream, shard_id, None, None, ShardIterator::Latest, running)
    }

    fn get_shard_iterator(&mut self, cx: &mut Context<'_>) {
        let _ = self.client
            .get_shard_iterator(GetShardIteratorInput {
                shard_id: self.shard_id.clone(),
                shard_iterator_type: self.iterator_type.to_owned(),
                starting_sequence_number: self.seq_number.clone(),
                stream_name: self.stream.clone(),
                timestamp: None,
            }).and_then(|it| {
                self.it = it.shard_iterator;
                cx.waker().wake_by_ref();
                past::future::ok(())
            }).map_err(|e| {
                panic!("Do something here: {}", e);
            }).wait();
    }
}

impl Stream for KinesisShard {
    type Item = GetRecordsOutput;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if *self.running.read().unwrap() == false {
            return Poll::Ready(None)
        }

        if let Some(it) = &self.it {
            let result: Result<GetRecordsOutput, _> = self.client.get_records(GetRecordsInput {
                limit: None,
                shard_iterator: it.clone(),
            }).sync();

            if let Ok(records) = result {
                self.it = records.next_shard_iterator.clone();
                if records.records.len() > 0 {
                    Poll::Ready(Some(records))
                }
                else {
                    let wa = cx.waker().clone();
                    let delay = std::time::Instant::now().add(Duration::from_millis(500));
                    let f = tokio::timer::delay(delay)
                        .map(move |v| {
                            wa.wake_by_ref();
                            v
                        });

                    tokio::spawn(f); // TODO this makes the runtime to fail when stopping the stream

                    Poll::Pending
                }
            }
            else {
                let wa = cx.waker().clone();
                let delay = std::time::Instant::now().add(Duration::from_millis(500));
                let f = tokio::timer::delay(delay)
                    .map(move |v| {
                        wa.wake_by_ref();
                        v
                    });
                
                tokio::spawn(f); // TODO this makes the runtime to fail when stopping the stream

                Poll::Pending
            }
        }
        else {
            self.get_shard_iterator(cx);
            Poll::Pending
        }
    }
}