use std::{
    collections::{HashMap, hash_map},
    time::Instant,
};

use libp2p::{kad::store::MemoryStore, swarm::NetworkBehaviour};

use crate::utils::generate_node_id;

pub type NodeId = u64;
pub type RamCapacity = usize;

pub struct DHT {
    pub inner: HashMap<NodeId, NodePerf>,
}

struct NodePerf {
    pub node_id: String,
    pub ram_tokens: usize,
    pub layer_latency: HashMap<LayerId, f32>,
    pub rtt: HashMap<NodeId, f32>,
    pub last_updated: Instant,
}

pub struct PerfMap {
    pub inner: HashMap<String, NodePerf>,
}

pub type LayerId = u32;

#[derive(NetworkBehaviour)]
pub struct Behaviour {
    pub kad: Kademlia<MemoryStore>,
    pub ping: Ping,
    pub identify: Identify,
}

fn publish_perf(kad: &mut Kademlia<MemoryStore>, perf: &NodePerf) {
    let key = format!("perf/{}", perf.node_id);
    let value = serde_json::to_vec(perf).unwrap();

    let record = Record::new(key.into_bytes(), value);
    kad.put_record(record, Quorum::One).unwrap();
}

fn fetch_node(kad: &mut Kademlia<MemoryStore>, node_id: &str) {
    let key = format!("perf/{}", node_id);
    kad.get_record(key.into_bytes(), Quorum::One);
}
