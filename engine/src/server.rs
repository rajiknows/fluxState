//! This example demonstrates an HTTP server that serves files from a directory.
//!
//! Checkout the `README.md` for guidance.
use anyhow::Result;
use quinn::{ClientConfig, Endpoint, RecvStream, SendStream, ServerConfig};
use rustls::{
    ClientConfig as TlsClientConfig, RootCertStore, ServerConfig as TlsServerConfig,
    pki_types::{CertificateDer, PrivateKeyDer},
};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::dht::{GossipMsg, NodePerf};

struct CertChain {
    cert_chain: Vec<CertificateDer<'static>>,
    private_key: PrivateKeyDer<'static>,
}

fn generate_self_signed_certificates() -> Result<CertChain> {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;

    let cert_der = cert.cert.der().clone();
    let key_der = PrivateKeyDer::Pkcs8(cert.signing_key.serialize_der().into());

    Ok(CertChain {
        cert_chain: vec![cert_der],
        private_key: key_der,
    })
}

fn make_client_config(server_cert: CertificateDer<'static>) -> Result<ClientConfig> {
    let mut roots = RootCertStore::empty();
    roots.add(server_cert)?;

    let tls = TlsClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();

    Ok(ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(tls)?,
    )))
}

pub type ClusterMap = Arc<RwLock<HashMap<String, NodePerf>>>;

pub async fn start_server(addr: &str, cluster: ClusterMap) -> Result<()> {
    let cert = generate_self_signed_certificates()?;

    let tls = TlsServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert.cert_chain.clone(), cert.private_key)?;

    let server_config = ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(tls)?,
    ));

    let endpoint = Endpoint::server(server_config, addr.parse()?)?;

    info!("server listening on {addr}");

    while let Some(connecting) = endpoint.accept().await {
        let cluster = cluster.clone();

        tokio::spawn(async move {
            let conn = match connecting.await {
                Ok(c) => c,
                Err(e) => {
                    error!("connection failed: {e}");
                    return;
                }
            };

            while let Ok((send, recv)) = conn.accept_bi().await {
                let cluster = cluster.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_stream(send, recv, cluster).await {
                        error!("stream error: {e}");
                    }
                });
            }
        });
    }

    Ok(())
}

async fn handle_stream(
    mut send: SendStream,
    mut recv: RecvStream,
    cluster: ClusterMap,
) -> Result<()> {
    let data = recv.read_to_end(1024 * 1024).await?;
    let msg: GossipMsg = serde_json::from_slice(&data)?;

    match msg {
        GossipMsg::Perf(perf) => {
            merge_perf(cluster, perf).await;
        }

        GossipMsg::SyncRequest => {
            let snapshot = {
                let map = cluster.read().await;
                map.values().cloned().collect::<Vec<_>>()
            };

            let resp = GossipMsg::SyncResponse(snapshot);
            let bytes = serde_json::to_vec(&resp)?;
            send.write_all(&bytes).await?;
        }

        GossipMsg::SyncResponse(perfs) => {
            for p in perfs {
                merge_perf(cluster.clone(), p).await;
            }
        }
    }

    send.finish()?;
    Ok(())
}

async fn merge_perf(cluster: ClusterMap, incoming: NodePerf) {
    let mut map = cluster.write().await;

    match map.get(&incoming.node_id) {
        Some(old) if old.timestamp_ms >= incoming.timestamp_ms => {}
        _ => {
            map.insert(incoming.node_id.clone(), incoming);
        }
    }
}
pub async fn send_perf(
    addr: &str,
    perf: NodePerf,
    server_cert: CertificateDer<'static>,
) -> Result<()> {
    let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
    let client_cfg = make_client_config(server_cert)?;
    endpoint.set_default_client_config(client_cfg);

    let conn = endpoint.connect(addr.parse()?, "localhost")?.await?;

    let (mut send, _) = conn.open_bi().await?;

    let msg = GossipMsg::Perf(perf);
    let bytes = serde_json::to_vec(&msg)?;

    send.write_all(&bytes).await?;
    send.finish()?;

    Ok(())
}

pub async fn request_sync(
    addr: &str,
    server_cert: CertificateDer<'static>,
    cluster: ClusterMap,
) -> Result<()> {
    let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
    let client_cfg = make_client_config(server_cert)?;
    endpoint.set_default_client_config(client_cfg);

    let conn = endpoint.connect(addr.parse()?, "localhost")?.await?;

    let (mut send, mut recv) = conn.open_bi().await?;

    let msg = GossipMsg::SyncRequest;
    let bytes = serde_json::to_vec(&msg)?;

    send.write_all(&bytes).await?;
    send.finish()?;

    let resp = recv.read_to_end(1024 * 1024).await?;
    let msg: GossipMsg = serde_json::from_slice(&resp)?;

    if let GossipMsg::SyncResponse(perfs) = msg {
        for p in perfs {
            merge_perf(cluster.clone(), p).await;
        }
    }

    Ok(())
}
