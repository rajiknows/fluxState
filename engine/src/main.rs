use clap::{Parser, Subcommand};
use std::{
    collections::HashMap,
    env,
    sync::{Arc, RwLock},
    time::SystemTime,
};

use crate::{
    dht::NodePerf,
    gossip::start_gossip_loop,
    server::{ClusterMap, request_sync, start_server},
};

mod client;
mod dht;
mod gossip;
mod gpu;
mod model;
mod scheduling;
mod server;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Start {
        #[arg(long)]
        addr: String,
    },
    Join {
        #[arg(long)]
        addr: String,
        #[arg(long)]
        peer: String,
    },
}

struct ModelMetadata {
    name: String,
    model_layers: usize,
}

fn build_local_perf(node_id: String) -> NodePerf {
    NodePerf {
        node_id,
        ram_tokens: 1024,
        layer_latency: HashMap::new(),
        rtt: HashMap::new(),
        timestamp_ms: now_ms(),
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let node_id = env::var("NODE_ID").unwrap_or_else(|_| "node-1".into());

    let cluster: ClusterMap = Arc::new(RwLock::new(HashMap::new()));

    match cli.command {
        Commands::Start { addr } => {
            let cluster_clone = cluster.clone();

            tokio::spawn(async move {
                start_server(&addr, cluster_clone).await.unwrap();
            });

            start_gossip_loop(cluster, node_id).await;
        }

        Commands::Join { addr, peer } => {
            let cluster_clone = cluster.clone();

            tokio::spawn(async move {
                start_server(&addr, cluster_clone).await.unwrap();
            });

            // sync from existing node
            request_sync(&peer, cluster.clone()).await?;

            start_gossip_loop(cluster, node_id).await;
        }
    }

    Ok(())
}

struct SystemInfo {
    ram: usize,
    gpu_vram: usize,
}
