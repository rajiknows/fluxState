use std::time::Duration;

use crate::{
    build_local_perf,
    server::{ClusterMap, send_perf},
};

pub async fn start_gossip_loop(cluster: ClusterMap, node_id: String) {
    let peers: Vec<String> = vec![];

    loop {
        let perf = build_local_perf(node_id.clone());

        {
            let mut map = cluster.write().await;
            map.insert(perf.node_id.clone(), perf.clone());
        }

        for peer in &peers {
            let _ = send_perf(peer, perf.clone()).await;
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
