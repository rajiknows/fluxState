//! This example demonstrates an HTTP server that serves files from a directory.
//!
//! Checkout the `README.md` for guidance.

use std::{
    ascii, fs, io,
    net::SocketAddr,
    path::{self, Path, PathBuf},
    str,
    sync::Arc,
};

use anyhow::{Context, anyhow, bail};
use quinn::{Endpoint, ServerConfig};
use rustls::{
    ServerConfig as TlsServerConfig,
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, pem::PemObject},
};

struct CertChain {
    cert_chain: Vec<rustls::pki_types::CertificateDer<'static>>,
    private_key: PrivateKeyDer<'static>,
}

fn generate_self_signed_certificates() -> Result<CertChain, anyhow::Error> {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;

    let cert_der = cert.cert.der().clone();

    let key_der = PrivateKeyDer::Pkcs8(cert.signing_key.serialize_der().into());

    Ok(CertChain {
        cert_chain: vec![cert_der],
        private_key: key_der,
    })
}

#[tokio::main]
async fn main() -> std::result::Result<(), anyhow::Error> {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .unwrap();
    let cert = generate_self_signed_certificates()?;
    let mut tlsconfig = TlsServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert.cert_chain, cert.private_key)
        .unwrap();
    tlsconfig.alpn_protocols = vec![b"h3".to_vec()];
    let server_config = ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(tlsconfig)?,
    ));
    let endpoint = Endpoint::server(server_config, "127.0.0.1:4433".parse()?)?;
    while let Some(conn) = endpoint.accept().await {
        let _ = conn.await?;
    }
    Ok(())
}
//  code extracted from quinn-rs example
// #[tokio::main]
// async fn run(options: Opt) -> Result<()> {
//     let (certs, key) = if let (Some(key_path), Some(cert_path)) = (&options.key, &options.cert) {
//         let key = if key_path.extension().is_some_and(|x| x == "der") {
//             PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
//                 fs::read(key_path).context("failed to read private key file")?,
//             ))
//         } else {
//             PrivateKeyDer::from_pem_file(key_path)
//                 .context("failed to read PEM from private key file")?
//         };
//
//         let cert_chain = if cert_path.extension().is_some_and(|x| x == "der") {
//             vec![CertificateDer::from(
//                 fs::read(cert_path).context("failed to read certificate chain file")?,
//             )]
//         } else {
//             CertificateDer::pem_file_iter(cert_path)
//                 .context("failed to read PEM from certificate chain file")?
//                 .collect::<Result<_, _>>()
//                 .context("invalid PEM-encoded certificate")?
//         };
//
//         (cert_chain, key)
//     } else {
//         let dirs = directories_next::ProjectDirs::from("org", "quinn", "quinn-examples").unwrap();
//         let path = dirs.data_local_dir();
//         let cert_path = path.join("cert.der");
//         let key_path = path.join("key.der");
//         let (cert, key) = match fs::read(&cert_path).and_then(|x| Ok((x, fs::read(&key_path)?))) {
//             Ok((cert, key)) => (
//                 CertificateDer::from(cert),
//                 PrivateKeyDer::try_from(key).map_err(anyhow::Error::msg)?,
//             ),
//             Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
//                 info!("generating self-signed certificate");
//                 let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
//                 let key = PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der());
//                 let cert = cert.cert.into();
//                 fs::create_dir_all(path).context("failed to create certificate directory")?;
//                 fs::write(&cert_path, &cert).context("failed to write certificate")?;
//                 fs::write(&key_path, key.secret_pkcs8_der())
//                     .context("failed to write private key")?;
//                 (cert, key.into())
//             }
//             Err(e) => {
//                 bail!("failed to read certificate: {}", e);
//             }
//         };
//
//         (vec![cert], key)
//     };
//
//     let mut server_crypto = rustls::ServerConfig::builder()
//         .with_no_client_auth()
//         .with_single_cert(certs, key)?;
//     server_crypto.alpn_protocols = common::ALPN_QUIC_HTTP.iter().map(|&x| x.into()).collect();
//     if options.keylog {
//         server_crypto.key_log = Arc::new(rustls::KeyLogFile::new());
//     }
//
//     let mut server_config =
//         quinn::ServerConfig::with_crypto(Arc::new(QuicServerConfig::try_from(server_crypto)?));
//     let transport_config = Arc::get_mut(&mut server_config.transport).unwrap();
//     transport_config.max_concurrent_uni_streams(0_u8.into());
//
//     let root = Arc::<Path>::from(options.root.clone());
//     if !root.exists() {
//         bail!("root path does not exist");
//     }
//
//     let endpoint = quinn::Endpoint::server(server_config, options.listen)?;
//     eprintln!("listening on {}", endpoint.local_addr()?);
//
//     while let Some(conn) = endpoint.accept().await {
//         if options
//             .connection_limit
//             .is_some_and(|n| endpoint.open_connections() >= n)
//         {
//             info!("refusing due to open connection limit");
//             conn.refuse();
//         } else if Some(conn.remote_address()) == options.block {
//             info!("refusing blocked client IP address");
//             conn.refuse();
//         } else if options.stateless_retry && !conn.remote_address_validated() {
//             info!("requiring connection to validate its address");
//             conn.retry().unwrap();
//         } else {
//             info!("accepting connection");
//             let fut = handle_connection(root.clone(), conn);
//             tokio::spawn(async move {
//                 if let Err(e) = fut.await {
//                     error!("connection failed: {reason}", reason = e.to_string())
//                 }
//             });
//         }
//     }
//
//     Ok(())
// }
//
// async fn handle_connection(root: Arc<Path>, conn: quinn::Incoming) -> Result<()> {
//     let connection = conn.await?;
//     let span = info_span!(
//         "connection",
//         remote = %connection.remote_address(),
//         protocol = %connection
//             .handshake_data()
//             .unwrap()
//             .downcast::<quinn::crypto::rustls::HandshakeData>().unwrap()
//             .protocol
//             .map_or_else(|| "<none>".into(), |x| String::from_utf8_lossy(&x).into_owned())
//     );
//     async {
//         info!("established");
//
//         // Each stream initiated by the client constitutes a new request.
//         loop {
//             let stream = connection.accept_bi().await;
//             let stream = match stream {
//                 Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
//                     info!("connection closed");
//                     return Ok(());
//                 }
//                 Err(e) => {
//                     return Err(e);
//                 }
//                 Ok(s) => s,
//             };
//             let fut = handle_request(root.clone(), stream);
//             tokio::spawn(
//                 async move {
//                     if let Err(e) = fut.await {
//                         error!("failed: {reason}", reason = e.to_string());
//                     }
//                 }
//                 .instrument(info_span!("request")),
//             );
//         }
//     }
//     .instrument(span)
//     .await?;
//     Ok(())
// }
//
// async fn handle_request(
//     root: Arc<Path>,
//     (mut send, mut recv): (quinn::SendStream, quinn::RecvStream),
// ) -> Result<()> {
//     let req = recv
//         .read_to_end(64 * 1024)
//         .await
//         .map_err(|e| anyhow!("failed reading request: {}", e))?;
//     let mut escaped = String::new();
//     for &x in &req[..] {
//         let part = ascii::escape_default(x).collect::<Vec<_>>();
//         escaped.push_str(str::from_utf8(&part).unwrap());
//     }
//     info!(content = %escaped);
//     // Execute the request
//     let resp = process_get(&root, &req).unwrap_or_else(|e| {
//         error!("failed: {}", e);
//         format!("failed to process request: {e}\n").into_bytes()
//     });
//     // Write the response
//     send.write_all(&resp)
//         .await
//         .map_err(|e| anyhow!("failed to send response: {}", e))?;
//     // Gracefully terminate the stream
//     send.finish().unwrap();
//     info!("complete");
//     Ok(())
// }
//
// fn process_get(root: &Path, x: &[u8]) -> Result<Vec<u8>> {
//     if x.len() < 4 || &x[0..4] != b"GET " {
//         bail!("missing GET");
//     }
//     if x[4..].len() < 2 || &x[x.len() - 2..] != b"\r\n" {
//         bail!("missing \\r\\n");
//     }
//     let x = &x[4..x.len() - 2];
//     let end = x.iter().position(|&c| c == b' ').unwrap_or(x.len());
//     let path = str::from_utf8(&x[..end]).context("path is malformed UTF-8")?;
//     let path = Path::new(&path);
//     let mut real_path = PathBuf::from(root);
//     let mut components = path.components();
//     match components.next() {
//         Some(path::Component::RootDir) => {}
//         _ => {
//             bail!("path must be absolute");
//         }
//     }
//     for c in components {
//         match c {
//             path::Component::Normal(x) => {
//                 real_path.push(x);
//             }
//             x => {
//                 bail!("illegal component in path: {:?}", x);
//             }
//         }
//     }
//     let data = fs::read(&real_path).context("failed reading file")?;
//     Ok(data)
// }
