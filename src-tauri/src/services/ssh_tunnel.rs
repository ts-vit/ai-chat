use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use russh::client::{self, Handle};
use russh::keys::key::PublicKey;
use russh::{Channel, ChannelMsg, Disconnect};
use russh_keys::PublicKeyBase64;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{watch, Mutex};
use tokio::task::JoinHandle;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshTunnelStatus {
    pub connected: bool,
    pub local_port: Option<u16>,
    pub remote_host: Option<String>,
}

#[derive(Clone)]
struct ConnectionParams {
    host: String,
    port: u16,
    username: String,
    auth_type: String,
    password: Option<String>,
    private_key: Option<String>,
}

struct SshTunnelState {
    local_port: u16,
    remote_host: String,
    shutdown_tx: watch::Sender<bool>,
    listener_handle: JoinHandle<()>,
    keepalive_handle: JoinHandle<()>,
    ssh_handle: Arc<Handle<SshHandler>>,
}

pub struct SshTunnelManager {
    state: Mutex<Option<SshTunnelState>>,
    connection_params: Mutex<Option<ConnectionParams>>,
    manually_disconnected: Mutex<bool>,
}

impl SshTunnelManager {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(None),
            connection_params: Mutex::new(None),
            manually_disconnected: Mutex::new(false),
        }
    }

    pub async fn connect(
        self: &Arc<Self>,
        app: AppHandle,
        host: String,
        port: u16,
        username: String,
        auth_type: String,
        password: Option<String>,
        private_key: Option<String>,
    ) -> Result<u16, String> {
        let params = ConnectionParams {
            host,
            port,
            username,
            auth_type,
            password,
            private_key,
        };
        self.disconnect_inner(None).await;
        let result = Self::connect_fresh(self.clone(), app, params.clone()).await?;
        *self.connection_params.lock().await = Some(params);
        *self.manually_disconnected.lock().await = false;
        Ok(result)
    }

    /// Core connection logic. Does NOT call disconnect_inner (caller must handle cleanup).
    fn connect_fresh(
        this: Arc<Self>,
        app: AppHandle,
        params: ConnectionParams,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<u16, String>> + Send>> {
        Box::pin(async move {
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("app_data_dir: {}", e))?;

        let config = client::Config {
            keepalive_interval: Some(std::time::Duration::from_secs(30)),
            keepalive_max: 3,
            ..Default::default()
        };

        let handler = SshHandler {
            app: app.clone(),
            host: params.host.clone(),
            port: params.port,
            app_data_dir,
        };

        let addr = format!("{}:{}", params.host, params.port);
        let mut session = client::connect(Arc::new(config), &addr, handler)
            .await
            .map_err(|e| format!("SSH connect failed: {}", e))?;

        // Authenticate
        let auth_ok = match params.auth_type.as_str() {
            "key" => {
                let key_path_or_data = params
                    .private_key
                    .clone()
                    .ok_or("Private key path is required")?;
                let key_data = if key_path_or_data.contains("-----") {
                    key_path_or_data
                } else {
                    std::fs::read_to_string(&key_path_or_data).map_err(|e| {
                        format!(
                            "Failed to read SSH key file '{}': {}",
                            key_path_or_data, e
                        )
                    })?
                };
                let key_pair = russh_keys::decode_secret_key(&key_data, None)
                    .map_err(|e| format!("Failed to parse SSH key: {}", e))?;
                session
                    .authenticate_publickey(&params.username, Arc::new(key_pair))
                    .await
                    .map_err(|e| format!("SSH key auth failed: {}", e))?
            }
            _ => {
                let pwd = params.password.clone().unwrap_or_default();
                session
                    .authenticate_password(&params.username, &pwd)
                    .await
                    .map_err(|e| format!("SSH password auth failed: {}", e))?
            }
        };

        if !auth_ok {
            return Err("SSH authentication rejected".to_string());
        }

        let ssh_handle = Arc::new(session);

        // Bind SOCKS5 listener on random port
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| format!("Failed to bind SOCKS5 listener: {}", e))?;
        let local_port = listener
            .local_addr()
            .map_err(|e| format!("Failed to get local addr: {}", e))?
            .port();

        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Spawn SOCKS5 acceptor loop
        let ssh_for_listener = ssh_handle.clone();
        let shutdown_rx_listener = shutdown_rx.clone();
        let listener_handle = tokio::spawn(async move {
            loop {
                let mut shutdown_check = shutdown_rx_listener.clone();
                tokio::select! {
                    result = listener.accept() => {
                        match result {
                            Ok((stream, _addr)) => {
                                let ssh = ssh_for_listener.clone();
                                let shutdown = shutdown_rx_listener.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = handle_socks5_connection(stream, ssh, shutdown).await {
                                        log::debug!("[ssh-tunnel] SOCKS5 connection error: {}", e);
                                    }
                                });
                            }
                            Err(e) => {
                                log::error!("[ssh-tunnel] Accept error: {}", e);
                                break;
                            }
                        }
                    }
                    _ = shutdown_check.changed() => {
                        break;
                    }
                }
            }
        });

        // Spawn keepalive monitor with auto-reconnect
        let ssh_for_keepalive = ssh_handle.clone();
        let app_for_keepalive = app.clone();
        let shutdown_rx_keepalive = shutdown_rx.clone();
        let manager_for_keepalive = this.clone();
        let keepalive_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            let mut shutdown = shutdown_rx_keepalive;
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(_e) = ssh_for_keepalive.channel_open_session().await {
                            log::warn!("[ssh-tunnel] Keepalive failed, connection lost");

                            // Check if manually disconnected
                            if *manager_for_keepalive.manually_disconnected.lock().await {
                                let _ = app_for_keepalive.emit("ssh-tunnel-disconnected", serde_json::json!({}));
                                break;
                            }

                            // Clean up old state without aborting ourselves
                            {
                                let mut state = manager_for_keepalive.state.lock().await;
                                if let Some(old) = state.take() {
                                    let _ = old.shutdown_tx.send(true);
                                    old.listener_handle.abort();
                                    // Do NOT abort keepalive_handle — we are inside it
                                    let _ = old.ssh_handle
                                        .disconnect(Disconnect::ByApplication, "", "en")
                                        .await;
                                }
                            }

                            let _ = app_for_keepalive.emit("ssh-tunnel-reconnecting", serde_json::json!({}));

                            let params = manager_for_keepalive.connection_params.lock().await.clone();
                            if let Some(params) = params {
                                let mut delay_ms = 2000u64;
                                let mut reconnected = false;

                                for attempt in 1..=5u32 {
                                    if *manager_for_keepalive.manually_disconnected.lock().await {
                                        break;
                                    }

                                    let _ = app_for_keepalive.emit(
                                        "ssh-tunnel-reconnect-attempt",
                                        serde_json::json!({
                                            "attempt": attempt,
                                            "maxAttempts": 5,
                                        }),
                                    );

                                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;

                                    match SshTunnelManager::connect_fresh(
                                        manager_for_keepalive.clone(),
                                        app_for_keepalive.clone(),
                                        params.clone(),
                                    )
                                        .await
                                    {
                                        Ok(new_port) => {
                                            log::info!(
                                                "[ssh-tunnel] Reconnected on attempt {}, port {}",
                                                attempt,
                                                new_port
                                            );
                                            let _ = app_for_keepalive.emit(
                                                "ssh-tunnel-reconnected",
                                                serde_json::json!({ "port": new_port }),
                                            );
                                            let _ = app_for_keepalive.emit("proxy-settings-changed", ());
                                            reconnected = true;
                                            break;
                                        }
                                        Err(e) => {
                                            log::warn!(
                                                "[ssh-tunnel] Reconnect attempt {}/5 failed: {}",
                                                attempt,
                                                e
                                            );
                                            delay_ms = (delay_ms * 2).min(32000);
                                        }
                                    }
                                }

                                if !reconnected {
                                    let _ = app_for_keepalive.emit(
                                        "ssh-tunnel-reconnect-failed",
                                        serde_json::json!({}),
                                    );
                                    let _ = app_for_keepalive.emit(
                                        "ssh-tunnel-disconnected",
                                        serde_json::json!({}),
                                    );
                                }
                            } else {
                                let _ = app_for_keepalive.emit(
                                    "ssh-tunnel-disconnected",
                                    serde_json::json!({}),
                                );
                            }

                            break; // Exit this keepalive loop (reconnect spawns a new one)
                        }
                    }
                    _ = shutdown.changed() => {
                        break;
                    }
                }
            }
        });

        let host_for_event = params.host.clone();
        let _ = app.emit(
            "ssh-tunnel-connected",
            serde_json::json!({ "host": host_for_event, "port": local_port }),
        );
        let _ = app.emit("proxy-settings-changed", ());

        let mut state = this.state.lock().await;
        *state = Some(SshTunnelState {
            local_port,
            remote_host: params.host,
            shutdown_tx,
            listener_handle,
            keepalive_handle,
            ssh_handle,
        });

        log::info!(
            "[ssh-tunnel] Connected, SOCKS5 proxy on 127.0.0.1:{}",
            local_port
        );

        Ok(local_port)
        }) // Box::pin
    }

    pub async fn disconnect(&self, app: Option<&AppHandle>) -> Result<(), String> {
        *self.manually_disconnected.lock().await = true;
        self.disconnect_inner(app).await;
        Ok(())
    }

    async fn disconnect_inner(&self, app: Option<&AppHandle>) {
        let mut state = self.state.lock().await;
        if let Some(tunnel) = state.take() {
            let _ = tunnel.shutdown_tx.send(true);
            tunnel.listener_handle.abort();
            tunnel.keepalive_handle.abort();
            let _ = tunnel
                .ssh_handle
                .disconnect(Disconnect::ByApplication, "user disconnect", "en")
                .await;
            if let Some(app) = app {
                let _ = app.emit("ssh-tunnel-disconnected", serde_json::json!({}));
                let _ = app.emit("proxy-settings-changed", ());
            }
            log::info!("[ssh-tunnel] Disconnected");
        }
    }

    pub async fn is_connected(&self) -> bool {
        self.state.lock().await.is_some()
    }

    pub async fn get_proxy_url(&self) -> Option<String> {
        let state = self.state.lock().await;
        state
            .as_ref()
            .map(|s| format!("socks5://127.0.0.1:{}", s.local_port))
    }

    pub async fn get_status(&self) -> SshTunnelStatus {
        let state = self.state.lock().await;
        match state.as_ref() {
            Some(s) => SshTunnelStatus {
                connected: true,
                local_port: Some(s.local_port),
                remote_host: Some(s.remote_host.clone()),
            },
            None => SshTunnelStatus {
                connected: false,
                local_port: None,
                remote_host: None,
            },
        }
    }
}

struct SshHandler {
    app: AppHandle,
    host: String,
    port: u16,
    app_data_dir: PathBuf,
}

#[async_trait]
impl client::Handler for SshHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        let known_hosts_path = self.app_data_dir.join("ssh_known_hosts");
        let host_entry = format!("{}:{}", self.host, self.port);
        let key_b64 = server_public_key.public_key_base64();

        // Read existing known hosts file
        let contents = std::fs::read_to_string(&known_hosts_path).unwrap_or_default();
        for line in contents.lines() {
            if let Some(stored_key) = line.strip_prefix(&format!("{} ", host_entry)) {
                if stored_key == key_b64 {
                    log::info!("[ssh-tunnel] Host key verified for {}", host_entry);
                    return Ok(true);
                } else {
                    // Key mismatch — possible MITM attack
                    log::warn!("[ssh-tunnel] HOST KEY CHANGED for {}", host_entry);
                    let _ = self.app.emit(
                        "ssh-host-key-changed",
                        serde_json::json!({
                            "host": self.host,
                            "port": self.port,
                        }),
                    );
                    return Ok(false);
                }
            }
        }

        // First connection — TOFU: save and trust
        let _ = std::fs::create_dir_all(&self.app_data_dir);
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&known_hosts_path)
            .map_err(russh::Error::IO)?;
        writeln!(f, "{} {}", host_entry, key_b64).map_err(russh::Error::IO)?;
        log::info!("[ssh-tunnel] TOFU: saved host key for {}", host_entry);
        Ok(true)
    }
}

/// Handle a single SOCKS5 connection: parse SOCKS5 handshake, open SSH direct-tcpip channel,
/// and proxy data bidirectionally.
async fn handle_socks5_connection(
    mut stream: TcpStream,
    ssh: Arc<Handle<SshHandler>>,
    shutdown: watch::Receiver<bool>,
) -> Result<(), String> {
    // --- SOCKS5 handshake ---
    // Read client greeting
    let mut buf = [0u8; 2];
    stream
        .read_exact(&mut buf)
        .await
        .map_err(|e| format!("socks5 greeting read: {}", e))?;

    if buf[0] != 0x05 {
        return Err("Not SOCKS5".to_string());
    }

    let nmethods = buf[1] as usize;
    let mut methods = vec![0u8; nmethods];
    stream
        .read_exact(&mut methods)
        .await
        .map_err(|e| format!("socks5 methods read: {}", e))?;

    // Reply: no auth required
    stream
        .write_all(&[0x05, 0x00])
        .await
        .map_err(|e| format!("socks5 greeting reply: {}", e))?;

    // Read connect request
    let mut header = [0u8; 4];
    stream
        .read_exact(&mut header)
        .await
        .map_err(|e| format!("socks5 request read: {}", e))?;

    if header[0] != 0x05 || header[1] != 0x01 {
        // Only CONNECT supported
        let reply = [0x05, 0x07, 0x00, 0x01, 0, 0, 0, 0, 0, 0];
        let _ = stream.write_all(&reply).await;
        return Err("Unsupported SOCKS5 command".to_string());
    }

    let (dest_host, dest_port) = match header[3] {
        0x01 => {
            // IPv4
            let mut addr = [0u8; 4];
            stream
                .read_exact(&mut addr)
                .await
                .map_err(|e| format!("socks5 ipv4 read: {}", e))?;
            let host = format!("{}.{}.{}.{}", addr[0], addr[1], addr[2], addr[3]);
            let mut port_buf = [0u8; 2];
            stream
                .read_exact(&mut port_buf)
                .await
                .map_err(|e| format!("socks5 port read: {}", e))?;
            let port = u16::from_be_bytes(port_buf);
            (host, port)
        }
        0x03 => {
            // Domain
            let mut len_buf = [0u8; 1];
            stream
                .read_exact(&mut len_buf)
                .await
                .map_err(|e| format!("socks5 domain len: {}", e))?;
            let len = len_buf[0] as usize;
            let mut domain = vec![0u8; len];
            stream
                .read_exact(&mut domain)
                .await
                .map_err(|e| format!("socks5 domain read: {}", e))?;
            let host = String::from_utf8_lossy(&domain).to_string();
            let mut port_buf = [0u8; 2];
            stream
                .read_exact(&mut port_buf)
                .await
                .map_err(|e| format!("socks5 port read: {}", e))?;
            let port = u16::from_be_bytes(port_buf);
            (host, port)
        }
        0x04 => {
            // IPv6
            let mut addr = [0u8; 16];
            stream
                .read_exact(&mut addr)
                .await
                .map_err(|e| format!("socks5 ipv6 read: {}", e))?;
            let host = std::net::Ipv6Addr::from(addr).to_string();
            let mut port_buf = [0u8; 2];
            stream
                .read_exact(&mut port_buf)
                .await
                .map_err(|e| format!("socks5 port read: {}", e))?;
            let port = u16::from_be_bytes(port_buf);
            (host, port)
        }
        _ => {
            return Err("Unknown SOCKS5 address type".to_string());
        }
    };

    // Open SSH direct-tcpip channel
    let channel = ssh
        .channel_open_direct_tcpip(&dest_host, dest_port as u32, "127.0.0.1", 0)
        .await
        .map_err(|e| format!("SSH direct-tcpip failed for {}:{}: {}", dest_host, dest_port, e))?;

    // Send SOCKS5 success reply
    let reply = [0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0];
    stream
        .write_all(&reply)
        .await
        .map_err(|e| format!("socks5 reply write: {}", e))?;

    // Bidirectional proxy between TCP stream and SSH channel
    proxy_bidirectional(stream, channel, shutdown).await;

    Ok(())
}

/// Proxy data between a TCP stream and an SSH channel.
async fn proxy_bidirectional(
    stream: TcpStream,
    channel: Channel<client::Msg>,
    mut shutdown: watch::Receiver<bool>,
) {
    let (mut tcp_read, mut tcp_write) = stream.into_split();
    // We need to split the channel into reader/writer
    // russh Channel doesn't support split directly, so we use the channel's methods
    let channel = Arc::new(Mutex::new(channel));
    let channel_for_read = channel.clone();

    // TCP → SSH
    let channel_for_tcp = channel.clone();
    let tcp_to_ssh = tokio::spawn(async move {
        let mut buf = [0u8; 8192];
        loop {
            match tcp_read.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    let ch = channel_for_tcp.lock().await;
                    if ch.data(&buf[..n]).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        let ch = channel_for_tcp.lock().await;
        let _ = ch.eof().await;
    });

    // SSH → TCP
    let ssh_to_tcp = tokio::spawn(async move {
        loop {
            let msg = {
                let mut ch = channel_for_read.lock().await;
                ch.wait().await
            };
            match msg {
                Some(ChannelMsg::Data { data }) => {
                    if tcp_write.write_all(&data).await.is_err() {
                        break;
                    }
                }
                Some(ChannelMsg::Eof) | None => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = tcp_to_ssh => {}
        _ = ssh_to_tcp => {}
        _ = shutdown.changed() => {}
    }
}
