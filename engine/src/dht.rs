use std::{collections::HashMap, sync::RwLock};

use serde::{Deserialize, Serialize};

pub type NodeId = u64;
pub type RamCapacity = usize;

pub struct DHT {
    pub inner: RwLock<HashMap<NodeId, NodePerf>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NodePerf {
    pub node_id: String,
    pub ram_tokens: usize,
    pub layer_latency: HashMap<LayerId, f32>,
    pub rtt: HashMap<NodeId, f32>,
    pub timestamp_ms: u64,
}

pub struct PerfMap {
    pub inner: RwLock<HashMap<NodeId, NodePerf>>,
}

pub type LayerId = u32;

#[derive(Serialize, Deserialize)]
pub enum GossipMsg {
    Perf(NodePerf),
    SyncRequest,
    SyncResponse(Vec<NodePerf>),
}
