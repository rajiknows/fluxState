use std::{
    collections::{HashMap, hash_map},
    time::Instant,
};

use serde::{Deserialize, Serialize};

pub type NodeId = u64;
pub type RamCapacity = usize;

pub struct DHT {
    pub inner: HashMap<NodeId, NodePerf>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodePerf {
    pub node_id: String,
    pub ram_tokens: usize,
    pub layer_latency: HashMap<LayerId, f32>,
    pub rtt: HashMap<NodeId, f32>,
    // pub last_updated: Instant,
}

pub struct PerfMap {
    pub inner: HashMap<NodeId, NodePerf>,
}

pub type LayerId = u32;

#[derive(Serialize, Deserialize)]
pub enum GossipMsg {
    Perf(NodePerf),
    SyncRequest,
    SyncResponse(Vec<NodePerf>),
}
