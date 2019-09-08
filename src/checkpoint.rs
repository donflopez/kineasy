use std::{time::Duration, collections::HashMap};
use serde_derive::{Serialize, Deserialize};


#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Checkpointing {
	pub(crate) active: bool,
    pub(crate) custom: bool,
    checkpoints: Checkpoints,
    pub(crate) file: Option<String>,
	pub(crate) interval: Duration,
}

#[derive(Clone, Serialize, Deserialize, Default)]

struct Checkpoint {
	stream: String,
	shard_id: String,
	seq_number: String,
}

type Checkpoints = HashMap<String, Checkpoint>;