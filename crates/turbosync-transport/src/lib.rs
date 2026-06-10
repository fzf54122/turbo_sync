use std::{net::SocketAddr, path::Path, path::PathBuf, sync::Arc};

use anyhow::{bail, Context, Result};
use quinn::{Connection, Endpoint, RecvStream, SendStream, TransportConfig};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

/// Handler for file-index requests: receives task_id, returns JSON-encoded bytes.
pub type FileIndexHandler = Arc<dyn Fn(String) -> Result<Vec<u8>> + Send + Sync>;

/// Handler for pull-file requests: receives (task_id, relative_path), returns file bytes.
pub type PullFileHandler = Arc<dyn Fn(String, String) -> Result<Vec<u8>> + Send + Sync>;

/// Handler for incoming file operations: receives (target_root, relative_path, op_kind, bytes).
pub type IncomingFileOpHandler =
    Arc<dyn Fn(String, String, String, u64) -> Result<()> + Send + Sync>;

// ── certificate helpers ───────────────────────────────────────────────

fn make_server_config(
    cert: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
) -> Result<quinn::ServerConfig> {
    let mut config = quinn::ServerConfig::with_single_cert(cert, key)?;
    config.transport_config(Arc::new(bidi_transport()));
    Ok(config)
}

fn make_client_config(expected_fingerprint: Option<&str>) -> Result<quinn::ClientConfig> {
    let rustls_config =
        rustls::ClientConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(FingerprintVerifier {
                expected: expected_fingerprint.map(str::to_owned),
            }))
            .with_no_client_auth();
    let quic_config = quinn::crypto::rustls::QuicClientConfig::try_from(Arc::new(rustls_config))
        .map_err(|_| anyhow::anyhow!("TLS 1.3 not available"))?;
    let mut config = quinn::ClientConfig::new(Arc::new(quic_config));
    config.transport_config(Arc::new(bidi_transport()));
    Ok(config)
}

fn bidi_transport() -> TransportConfig {
    let mut transport = TransportConfig::default();
    transport.max_concurrent_bidi_streams(256_u32.into());
    transport
}

#[derive(Debug)]
struct FingerprintVerifier {
    expected: Option<String>,
}

impl rustls::client::danger::ServerCertVerifier for FingerprintVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> std::result::Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        if let Some(ref expected) = self.expected {
            let actual = compute_fingerprint(end_entity.as_ref());
            if actual != *expected {
                return Err(rustls::Error::General(format!(
                    "cert fingerprint mismatch: expected {expected}"
                )));
            }
        }
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

fn compute_fingerprint(der: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(der);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

// ── op kind wire encoding ─────────────────────────────────────────────

fn op_kind_to_u8(kind: &str) -> u8 {
    match kind {
        "create_file" => 0,
        "update_file" => 1,
        "delete_file" => 2,
        "create_dir" => 3,
        "delete_dir" => 4,
        _ => 0,
    }
}

fn op_kind_from_u8(v: u8) -> Result<String> {
    match v {
        0 => Ok("create_file".to_owned()),
        1 => Ok("update_file".to_owned()),
        2 => Ok("delete_file".to_owned()),
        3 => Ok("create_dir".to_owned()),
        4 => Ok("delete_dir".to_owned()),
        other => bail!("unknown operation kind 0x{other:02x}"),
    }
}

// ── server ────────────────────────────────────────────────────────────

struct ConnectionHandlers {
    file_index: Option<FileIndexHandler>,
    pull_file: Option<PullFileHandler>,
    incoming_file_op: Option<IncomingFileOpHandler>,
}

pub struct TransportServer {
    endpoint: Endpoint,
    handlers: ConnectionHandlers,
}

impl TransportServer {
    pub async fn bind(
        addr: &str,
        cert: CertificateDer<'static>,
        key: PrivateKeyDer<'static>,
    ) -> Result<Self> {
        let addr: SocketAddr = addr
            .parse()
            .with_context(|| format!("invalid transport address {addr}"))?;
        let endpoint = Endpoint::server(make_server_config(vec![cert], key)?, addr)
            .with_context(|| format!("failed to bind transport server on {addr}"))?;
        tracing::info!(%addr, "transport server listening");
        Ok(Self {
            endpoint,
            handlers: ConnectionHandlers {
                file_index: None,
                pull_file: None,
                incoming_file_op: None,
            },
        })
    }

    /// Register a handler for `0x10` file-index requests.
    #[must_use]
    pub fn with_file_index_handler(mut self, handler: FileIndexHandler) -> Self {
        self.handlers.file_index = Some(handler);
        self
    }

    /// Register a handler for `0x20` pull-file requests.
    #[must_use]
    pub fn with_pull_file_handler(mut self, handler: PullFileHandler) -> Self {
        self.handlers.pull_file = Some(handler);
        self
    }

    /// Register a handler for incoming `0x00` file operations.
    #[must_use]
    pub fn with_incoming_file_op_handler(mut self, handler: IncomingFileOpHandler) -> Self {
        self.handlers.incoming_file_op = Some(handler);
        self
    }

    /// Run the server forever, handling incoming connections.
    pub async fn run(self) -> Result<()> {
        let handlers = Arc::new(self.handlers);
        loop {
            let incoming = self
                .endpoint
                .accept()
                .await
                .ok_or_else(|| anyhow::anyhow!("transport server endpoint closed"))?;

            let handlers = Arc::clone(&handlers);
            tokio::spawn(async move {
                if let Err(error) = handle_connection(incoming, handlers).await {
                    tracing::warn!(%error, "transport connection failed");
                }
            });
        }
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.endpoint.local_addr().context("no local address")
    }
}

async fn handle_connection(
    incoming: quinn::Incoming,
    handlers: Arc<ConnectionHandlers>,
) -> Result<()> {
    let connection = incoming.await?;
    tracing::info!(
        remote = %connection.remote_address(),
        "transport connection accepted"
    );

    while let Ok((send, recv)) = connection.accept_bi().await {
        let handlers = Arc::clone(&handlers);
        tokio::spawn(async move {
            if let Err(error) = handle_stream(send, recv, handlers).await {
                tracing::warn!(%error, "transport stream failed");
            }
        });
    }

    Ok(())
}

async fn handle_stream(
    mut send: SendStream,
    mut recv: RecvStream,
    handlers: Arc<ConnectionHandlers>,
) -> Result<()> {
    let command = read_u8(&mut recv, "command").await?;

    match command {
        0x00 => handle_file_op(&mut send, &mut recv, &handlers).await,
        0x10 => {
            let task_id = read_string(&mut recv, "task_id").await?;
            match &handlers.file_index {
                Some(handler) => {
                    match handler(task_id) {
                        Ok(json_bytes) => {
                            write_u8(&mut send, 0x11).await?;
                            write_u8(&mut send, 0).await?; // status ok
                            write_u32(&mut send, json_bytes.len() as u32).await?;
                            send.write_all(&json_bytes).await?;
                        }
                        Err(e) => {
                            write_u8(&mut send, 0x11).await?;
                            write_u8(&mut send, 1).await?; // status error
                            write_string(&mut send, &e.to_string()).await?;
                        }
                    }
                }
                None => {
                    write_u8(&mut send, 0x11).await?;
                    write_u8(&mut send, 1).await?;
                    write_string(&mut send, "file index handler not configured").await?;
                }
            }
            send.finish()?;
            Ok(())
        }
        0x20 => {
            let task_id = read_string(&mut recv, "task_id").await?;
            let relative_path = read_string(&mut recv, "relative_path").await?;
            match &handlers.pull_file {
                Some(handler) => match handler(task_id, relative_path) {
                    Ok(data) => {
                        send.write_all(&[0]).await?;
                        write_u64(&mut send, data.len() as u64).await?;
                        send.write_all(&data).await?;
                    }
                    Err(e) => {
                        send.write_all(&[1]).await?;
                        write_string(&mut send, &e.to_string()).await?;
                    }
                },
                None => {
                    send.write_all(&[1]).await?;
                    write_string(&mut send, "pull file handler not configured").await?;
                }
            }
            send.finish()?;
            Ok(())
        }
        other => bail!("unknown command 0x{other:02x}"),
    }
}

async fn handle_file_op(
    send: &mut SendStream,
    recv: &mut RecvStream,
    handlers: &ConnectionHandlers,
) -> Result<()> {
    let target_root = read_string(recv, "target_root").await?;
    let target_root = PathBuf::from(target_root);
    let relative_path = read_string(recv, "relative_path").await?;
    let op_kind = read_u8(recv, "op_kind").await?;
    let op_kind = op_kind_from_u8(op_kind)?;
    let file_size = read_u64(recv, "file_size").await?;

    let result =
        apply_remote_operation(&target_root, &relative_path, &op_kind, file_size, recv).await;

    match result {
        Ok(bytes) => {
            send.write_all(&[0]).await?;
            tracing::info!(%relative_path, %op_kind, bytes, "remote operation ok");

            if let Some(handler) = &handlers.incoming_file_op {
                if let Err(e) = handler(
                    target_root.to_string_lossy().to_string(),
                    relative_path.clone(),
                    op_kind.clone(),
                    bytes,
                ) {
                    tracing::warn!(%relative_path, error = %e, "incoming file op handler failed");
                }
            }
        }
        Err(error) => {
            let msg = error.to_string();
            let len = u32::to_le_bytes(msg.len() as u32);
            send.write_all(&[1]).await?;
            send.write_all(&len).await?;
            send.write_all(msg.as_bytes()).await?;
            tracing::warn!(%relative_path, %error, "remote operation failed");
        }
    }

    send.finish()?;
    Ok(())
}

async fn apply_remote_operation(
    target_root: &Path,
    relative_path: &str,
    op_kind: &str,
    file_size: u64,
    recv: &mut RecvStream,
) -> Result<u64> {
    let path = safe_join(target_root, relative_path)?;

    match op_kind {
        "create_dir" => {
            std::fs::create_dir_all(&path)
                .with_context(|| format!("failed to create dir {}", path.display()))?;
            Ok(0)
        }
        "delete_dir" => {
            if path.exists() {
                std::fs::remove_dir(&path)
                    .with_context(|| format!("failed to remove dir {}", path.display()))?;
            }
            Ok(0)
        }
        "delete_file" => {
            if path.exists() {
                std::fs::remove_file(&path)
                    .with_context(|| format!("failed to delete file {}", path.display()))?;
            }
            Ok(0)
        }
        "create_file" | "update_file" => {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create dir {}", parent.display()))?;
            }
            let mut file = tokio::fs::File::create(&path)
                .await
                .with_context(|| format!("failed to create file {}", path.display()))?;
            let mut remaining = file_size;
            let buf = vec![0u8; 65536];
            while remaining > 0 {
                let limit = remaining.min(buf.len() as u64) as usize;
                let chunk = recv
                    .read_chunk(limit, true)
                    .await
                    .context("truncated stream reading file body")?
                    .ok_or_else(|| {
                        anyhow::anyhow!("unexpected end of stream for {}", relative_path)
                    })?;
                file.write_all(&chunk.bytes).await?;
                remaining -= chunk.bytes.len() as u64;
            }
            Ok(file_size)
        }
        _ => bail!("unsupported operation kind: {op_kind}"),
    }
}

// ── client ────────────────────────────────────────────────────────────

pub struct TransferSummary {
    pub succeeded: usize,
    pub failed: usize,
    pub bytes_sent: u64,
    pub errors: Vec<String>,
}

pub struct PullSummary {
    pub succeeded: usize,
    pub failed: usize,
    pub bytes_received: u64,
    pub errors: Vec<String>,
}

/// Pull files from the remote agent.
///
/// Sends `0x20` pull requests using `task_id`; the remote reads from the
/// task's source path and sends files back.  Received files are written to
/// `local_target_root`.
pub async fn pull_files(
    endpoint: &str,
    task_id: &str,
    operations: &[turbosync_sync::PlannedOperation],
    local_target_root: &Path,
    fingerprint: Option<&str>,
) -> Result<PullSummary> {
    let remote: SocketAddr = endpoint
        .parse()
        .with_context(|| format!("invalid remote address {endpoint}"))?;

    let mut client_endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
    client_endpoint.set_default_client_config(make_client_config(fingerprint)?);

    let connection = connect_with_retry(&client_endpoint, remote).await?;

    let mut summary = PullSummary {
        succeeded: 0,
        failed: 0,
        bytes_received: 0,
        errors: Vec::new(),
    };

    for operation in operations {
        match pull_single_file(
            &connection,
            task_id,
            &operation.relative_path,
            local_target_root,
        )
        .await
        {
            Ok(bytes) => {
                summary.succeeded += 1;
                summary.bytes_received += bytes;
            }
            Err(error) => {
                summary.failed += 1;
                summary
                    .errors
                    .push(format!("{}: {}", operation.relative_path, error));
            }
        }
    }

    connection.close(0u32.into(), b"done");
    Ok(summary)
}

async fn pull_single_file(
    connection: &Connection,
    task_id: &str,
    relative_path: &str,
    local_target_root: &Path,
) -> Result<u64> {
    let (mut send, mut recv) = connection
        .open_bi()
        .await
        .context("failed to open stream for pull")?;

    write_u8(&mut send, 0x20).await?;
    write_string(&mut send, task_id).await?;
    write_string(&mut send, relative_path).await?;

    let status = read_u8(&mut recv, "pull_status").await?;
    if status != 0 {
        let error_msg = read_string(&mut recv, "error_message")
            .await
            .unwrap_or_default();
        bail!("pull failed: {error_msg}");
    }

    let file_size = read_u64(&mut recv, "file_size").await?;
    let mut buf = vec![0u8; file_size as usize];
    recv.read_exact(&mut buf)
        .await
        .context("failed to read file data")?;

    let dest = safe_join(local_target_root, relative_path)?;
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("failed to create dir {}", parent.display()))?;
    }
    tokio::fs::write(&dest, &buf)
        .await
        .with_context(|| format!("failed to write {}", dest.display()))?;

    send.finish()?;
    Ok(file_size)
}

/// Connect to the remote agent and send the planned operations.
///
/// `fingerprint` is the expected SHA256 hex fingerprint of the remote's TLS
/// cert.  When `None`, cert verification is skipped (MVP/LAN mode).
/// `endpoint` is e.g. "192.168.1.20:38746".
pub async fn transfer_files(
    endpoint: &str,
    target_root: &Path,
    operations: &[turbosync_sync::PlannedOperation],
    source_root: &Path,
    fingerprint: Option<&str>,
) -> Result<TransferSummary> {
    let remote: SocketAddr = endpoint
        .parse()
        .with_context(|| format!("invalid remote address {endpoint}"))?;

    let mut client_endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
    client_endpoint.set_default_client_config(make_client_config(fingerprint)?);

    let connection = connect_with_retry(&client_endpoint, remote).await?;

    let target_root = target_root.to_string_lossy();
    let mut summary = TransferSummary {
        succeeded: 0,
        failed: 0,
        bytes_sent: 0,
        errors: Vec::new(),
    };

    for operation in operations {
        match send_operation(&connection, &target_root, operation, source_root).await {
            Ok(bytes) => {
                summary.succeeded += 1;
                summary.bytes_sent += bytes;
            }
            Err(error) => {
                summary.failed += 1;
                summary
                    .errors
                    .push(format!("{}: {}", operation.relative_path, error));
            }
        }
    }

    connection.close(0u32.into(), b"done");
    Ok(summary)
}

async fn connect_with_retry(client_endpoint: &Endpoint, remote: SocketAddr) -> Result<Connection> {
    let mut last_err = None;
    for delay in [1u64, 2, 4] {
        let connecting = client_endpoint
            .connect(remote, "turbosync")
            .context("failed to connect")?;

        match tokio::time::timeout(std::time::Duration::from_secs(3), connecting).await {
            Ok(Ok(conn)) => return Ok(conn),
            Ok(Err(e)) => {
                last_err = Some(anyhow::Error::new(e));
                tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
            }
            Err(_) => {
                last_err = Some(anyhow::anyhow!(
                    "transport connection to {remote} timed out"
                ));
                tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
            }
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("transport connection failed")))
        .context("transport connection failed after 3 retries")
}

/// Check whether a remote TurboSync transport endpoint can be reached.
pub async fn check_connection(endpoint: &str, fingerprint: Option<&str>) -> Result<()> {
    let remote: SocketAddr = endpoint
        .parse()
        .with_context(|| format!("invalid remote address {endpoint}"))?;

    let mut client_endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
    client_endpoint.set_default_client_config(make_client_config(fingerprint)?);

    let connecting = client_endpoint
        .connect(remote, "turbosync")
        .context("failed to connect")?;
    let connection = tokio::time::timeout(std::time::Duration::from_secs(2), connecting)
        .await
        .with_context(|| format!("transport connection to {remote} timed out"))??;
    connection.close(0u32.into(), b"health");
    Ok(())
}

/// Request the remote file index for a given task.
///
/// Returns JSON-encoded bytes that can be deserialized into `Vec<FileIndexEntry>`.
pub async fn request_file_index(
    endpoint: &str,
    task_id: &str,
    fingerprint: Option<&str>,
) -> Result<Vec<u8>> {
    let remote: SocketAddr = endpoint
        .parse()
        .with_context(|| format!("invalid remote address {endpoint}"))?;

    let mut client_endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
    client_endpoint.set_default_client_config(make_client_config(fingerprint)?);

    let connection = connect_with_retry(&client_endpoint, remote).await?;
    let (mut send, mut recv) = connection
        .open_bi()
        .await
        .context("failed to open stream for file index request")?;

    write_u8(&mut send, 0x10).await?;
    write_string(&mut send, task_id).await?;

    let resp_cmd = read_u8(&mut recv, "response_command").await?;
    if resp_cmd != 0x11 {
        bail!("unexpected response command 0x{resp_cmd:02x}");
    }
    let status = read_u8(&mut recv, "response_status").await?;
    if status != 0 {
        let error_msg = read_string(&mut recv, "error_message")
            .await
            .unwrap_or_default();
        bail!("file index request failed: {error_msg}");
    }

    let len = read_u32(&mut recv, "json_len").await? as usize;
    if len > 16 * 1024 * 1024 {
        bail!("file index response too large: {len}");
    }
    let mut buf = vec![0u8; len];
    recv.read_exact(&mut buf)
        .await
        .context("failed to read file index response")?;

    send.finish()?;
    connection.close(0u32.into(), b"done");
    Ok(buf)
}

async fn send_operation(
    connection: &Connection,
    target_root: &str,
    operation: &turbosync_sync::PlannedOperation,
    source_root: &Path,
) -> Result<u64> {
    let (mut send, mut recv) = connection
        .open_bi()
        .await
        .context("failed to open bidirectional stream")?;

    write_u8(&mut send, 0x00).await?;
    write_string(&mut send, target_root).await?;
    write_string(&mut send, &operation.relative_path).await?;
    write_u8(&mut send, op_kind_to_u8(operation.kind.as_str())).await?;

    let source_path = safe_join(source_root, &operation.relative_path)?;
    let (file_size, file_data) = match operation.kind {
        turbosync_sync::PlannedOperationKind::CreateFile
        | turbosync_sync::PlannedOperationKind::UpdateFile => {
            let data = tokio::fs::read(&source_path)
                .await
                .with_context(|| format!("failed to read {}", source_path.display()))?;
            let len = data.len() as u64;
            (len, Some(data))
        }
        _ => (0, None),
    };

    write_u64(&mut send, file_size).await?;
    if let Some(data) = file_data {
        send.write_all(&data).await?;
    }
    send.finish()?;

    // Read server response.
    let status = read_u8(&mut recv, "response_status").await?;
    if status == 0 {
        Ok(file_size)
    } else {
        let error_msg = read_string(&mut recv, "error_message")
            .await
            .unwrap_or_default();
        bail!("remote error: {error_msg}")
    }
}

// ── wire format helpers ───────────────────────────────────────────────

async fn read_u8(stream: &mut RecvStream, label: &str) -> Result<u8> {
    let chunk = stream
        .read_chunk(1, true)
        .await
        .with_context(|| format!("failed to read {label}"))?
        .ok_or_else(|| anyhow::anyhow!("unexpected end of stream reading {label}"))?;
    Ok(chunk.bytes[0])
}

async fn read_u64(stream: &mut RecvStream, label: &str) -> Result<u64> {
    let chunk = stream
        .read_chunk(8, true)
        .await
        .with_context(|| format!("failed to read {label}"))?
        .ok_or_else(|| anyhow::anyhow!("unexpected end of stream reading {label}"))?;
    if chunk.bytes.len() < 8 {
        bail!("truncated {label}");
    }
    Ok(u64::from_le_bytes(chunk.bytes[..8].try_into().unwrap()))
}

async fn read_string(stream: &mut RecvStream, label: &str) -> Result<String> {
    let len = read_u32(stream, &format!("{label}_len")).await? as usize;
    if len > 16 * 1024 * 1024 {
        bail!("{label} too large: {len}");
    }
    let mut buf = vec![0u8; len];
    let mut offset = 0;
    while offset < len {
        let chunk = stream
            .read_chunk(len - offset, true)
            .await
            .with_context(|| format!("failed to read {label}"))?
            .ok_or_else(|| anyhow::anyhow!("unexpected end of stream reading {label}"))?;
        buf[offset..offset + chunk.bytes.len()].copy_from_slice(&chunk.bytes);
        offset += chunk.bytes.len();
    }
    String::from_utf8(buf).with_context(|| format!("invalid utf-8 in {label}"))
}

async fn read_u32(stream: &mut RecvStream, label: &str) -> Result<u32> {
    let chunk = stream
        .read_chunk(4, true)
        .await
        .with_context(|| format!("failed to read {label}"))?
        .ok_or_else(|| anyhow::anyhow!("unexpected end of stream reading {label}"))?;
    if chunk.bytes.len() < 4 {
        bail!("truncated {label}");
    }
    Ok(u32::from_le_bytes(chunk.bytes[..4].try_into().unwrap()))
}

async fn write_u8(stream: &mut SendStream, v: u8) -> Result<()> {
    stream.write_all(&[v]).await?;
    Ok(())
}

async fn write_u32(stream: &mut SendStream, v: u32) -> Result<()> {
    stream.write_all(&v.to_le_bytes()).await?;
    Ok(())
}

async fn write_u64(stream: &mut SendStream, v: u64) -> Result<()> {
    stream.write_all(&v.to_le_bytes()).await?;
    Ok(())
}

async fn write_string(stream: &mut SendStream, s: &str) -> Result<()> {
    let len = s.len() as u32;
    stream.write_all(&len.to_le_bytes()).await?;
    stream.write_all(s.as_bytes()).await?;
    Ok(())
}

// ── path utilities ────────────────────────────────────────────────────

fn safe_join(root: &Path, relative_path: &str) -> Result<PathBuf> {
    use std::path::Component;

    let path = std::path::Path::new(relative_path);
    if path.is_absolute() {
        bail!("relative path must not be absolute: {relative_path}");
    }

    let mut joined = root.to_path_buf();
    for component in path.components() {
        match component {
            Component::Normal(part) => joined.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                bail!("relative path escapes sync root: {relative_path}");
            }
        }
    }

    Ok(joined)
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_cert() -> (CertificateDer<'static>, PrivateKeyDer<'static>) {
        let key = rcgen::KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256).unwrap();
        let cert = rcgen::CertificateParams::new(vec!["turbosync".into()])
            .unwrap()
            .self_signed(&key)
            .unwrap();
        let key_bytes = key.serialized_der().to_vec();
        let key_der = PrivateKeyDer::from(rustls::pki_types::PrivatePkcs8KeyDer::from(key_bytes));
        (cert.der().clone(), key_der)
    }

    #[tokio::test]
    async fn transfer_roundtrip_creates_file_on_remote() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source = temp_dir.path().join("source");
        let target = temp_dir.path().join("target");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(source.join("hello.txt"), b"hello transport").unwrap();

        let (cert, key) = test_cert();
        let server = TransportServer::bind("127.0.0.1:0", cert.clone(), key)
            .await
            .unwrap();
        let addr = server.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = server.run().await;
        });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let content = b"hello transport";
        let ops = vec![turbosync_sync::PlannedOperation {
            relative_path: "hello.txt".to_owned(),
            kind: turbosync_sync::PlannedOperationKind::CreateFile,
            size_bytes: Some(content.len() as i64),
        }];

        let fingerprint = compute_fingerprint(cert.as_ref());
        let summary = transfer_files(
            &addr.to_string(),
            &target,
            &ops,
            &source,
            Some(&fingerprint),
        )
        .await
        .unwrap();

        assert_eq!(summary.succeeded, 1);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.bytes_sent, content.len() as u64);
        assert!(target.join("hello.txt").exists());
        assert_eq!(
            std::fs::read(target.join("hello.txt")).unwrap(),
            b"hello transport"
        );
    }

    #[tokio::test]
    async fn transfer_roundtrip_deletes_file_on_remote() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source = temp_dir.path().join("source");
        let target = temp_dir.path().join("target");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("old.txt"), b"to delete").unwrap();

        let (cert, key) = test_cert();
        let server = TransportServer::bind("127.0.0.1:0", cert.clone(), key)
            .await
            .unwrap();
        let addr = server.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = server.run().await;
        });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let ops = vec![turbosync_sync::PlannedOperation {
            relative_path: "old.txt".to_owned(),
            kind: turbosync_sync::PlannedOperationKind::DeleteFile,
            size_bytes: Some(9),
        }];

        let fingerprint = compute_fingerprint(cert.as_ref());
        let summary = transfer_files(
            &addr.to_string(),
            &target,
            &ops,
            &source,
            Some(&fingerprint),
        )
        .await
        .unwrap();

        assert_eq!(summary.succeeded, 1);
        assert!(!target.join("old.txt").exists());
    }

    #[tokio::test]
    async fn transfer_reports_failures() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source = temp_dir.path().join("source");
        let target = temp_dir.path().join("target");
        std::fs::create_dir_all(&source).unwrap();

        let (cert, key) = test_cert();
        let server = TransportServer::bind("127.0.0.1:0", cert.clone(), key)
            .await
            .unwrap();
        let addr = server.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = server.run().await;
        });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let ops = vec![turbosync_sync::PlannedOperation {
            relative_path: "missing.txt".to_owned(),
            kind: turbosync_sync::PlannedOperationKind::CreateFile,
            size_bytes: Some(5),
        }];

        let fingerprint = compute_fingerprint(cert.as_ref());
        let summary = transfer_files(
            &addr.to_string(),
            &target,
            &ops,
            &source,
            Some(&fingerprint),
        )
        .await
        .unwrap();

        assert_eq!(summary.succeeded, 0);
        assert_eq!(summary.failed, 1);
        assert!(!summary.errors.is_empty());
    }

    #[tokio::test]
    async fn fingerprint_mismatch_rejects_connection() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source = temp_dir.path().join("source");
        let target = temp_dir.path().join("target");
        std::fs::create_dir_all(&source).unwrap();

        let (cert, key) = test_cert();
        let server = TransportServer::bind("127.0.0.1:0", cert, key)
            .await
            .unwrap();
        let addr = server.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = server.run().await;
        });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let ops = vec![turbosync_sync::PlannedOperation {
            relative_path: "test.txt".to_owned(),
            kind: turbosync_sync::PlannedOperationKind::CreateFile,
            size_bytes: Some(0),
        }];

        let result = transfer_files(
            &addr.to_string(),
            &target,
            &ops,
            &source,
            Some("deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"),
        )
        .await;

        assert!(result.is_err());
    }
}
