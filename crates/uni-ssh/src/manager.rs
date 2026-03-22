use std::sync::Arc;
use std::time::Duration;

use russh::client::{self, Handle};
use russh::Disconnect;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, watch, Mutex};
use tokio::task::JoinHandle;

use crate::handler::SshHandler;
use crate::socks5::handle_socks5_connection;
use crate::types::{SshConfig, SshEvent, SshTunnelStatus};

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
    connection_params: Mutex<Option<SshConfig>>,
    manually_disconnected: Mutex<bool>,
    event_tx: broadcast::Sender<SshEvent>,
}

impl SshTunnelManager {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(64);
        Self {
            state: Mutex::new(None),
            connection_params: Mutex::new(None),
            manually_disconnected: Mutex::new(false),
            event_tx,
        }
    }

    /// Subscribe to SSH tunnel events.
    pub fn subscribe(&self) -> broadcast::Receiver<SshEvent> {
        self.event_tx.subscribe()
    }

    pub async fn connect(
        self: &Arc<Self>,
        config: SshConfig,
    ) -> Result<u16, String> {
        self.disconnect_inner().await;
        let result = Self::connect_fresh(self.clone(), config.clone()).await?;
        *self.connection_params.lock().await = Some(config);
        *self.manually_disconnected.lock().await = false;
        Ok(result)
    }

    /// Core connection logic. Does NOT call disconnect_inner (caller must handle cleanup).
    fn connect_fresh(
        this: Arc<Self>,
        config: SshConfig,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<u16, String>> + Send>> {
        Box::pin(async move {
            let ssh_config = client::Config {
                keepalive_interval: Some(Duration::from_secs(30)),
                keepalive_max: 3,
                ..Default::default()
            };

            let handler = SshHandler {
                host: config.host.clone(),
                port: config.port,
                known_hosts_path: config.known_hosts_path.clone(),
                event_tx: this.event_tx.clone(),
            };

            let addr = format!("{}:{}", config.host, config.port);
            let mut session = client::connect(Arc::new(ssh_config), &addr, handler)
                .await
                .map_err(|e| format!("SSH connect failed: {}", e))?;

            // Authenticate
            let auth_ok = match config.auth_type.as_str() {
                "key" => {
                    let key_path_or_data = config
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
                        .authenticate_publickey(&config.username, Arc::new(key_pair))
                        .await
                        .map_err(|e| format!("SSH key auth failed: {}", e))?
                }
                _ => {
                    let pwd = config.password.clone().unwrap_or_default();
                    session
                        .authenticate_password(&config.username, &pwd)
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
            let shutdown_rx_keepalive = shutdown_rx.clone();
            let manager_for_keepalive = this.clone();
            let event_tx = this.event_tx.clone();
            let keepalive_handle = tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(30));
                let mut shutdown = shutdown_rx_keepalive;
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            if let Err(_e) = ssh_for_keepalive.channel_open_session().await {
                                log::warn!("[ssh-tunnel] Keepalive failed, connection lost");

                                if *manager_for_keepalive.manually_disconnected.lock().await {
                                    let _ = event_tx.send(SshEvent::Disconnected);
                                    break;
                                }

                                // Clean up old state without aborting ourselves
                                {
                                    let mut state = manager_for_keepalive.state.lock().await;
                                    if let Some(old) = state.take() {
                                        let _ = old.shutdown_tx.send(true);
                                        old.listener_handle.abort();
                                        let _ = old.ssh_handle
                                            .disconnect(Disconnect::ByApplication, "", "en")
                                            .await;
                                    }
                                }

                                let _ = event_tx.send(SshEvent::Reconnecting);

                                let params = manager_for_keepalive.connection_params.lock().await.clone();
                                if let Some(params) = params {
                                    let mut delay_ms = 2000u64;
                                    let mut reconnected = false;

                                    for attempt in 1..=5u32 {
                                        if *manager_for_keepalive.manually_disconnected.lock().await {
                                            break;
                                        }

                                        let _ = event_tx.send(SshEvent::ReconnectAttempt {
                                            attempt,
                                            max_attempts: 5,
                                        });

                                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;

                                        match SshTunnelManager::connect_fresh(
                                            manager_for_keepalive.clone(),
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
                                                let _ = event_tx.send(SshEvent::Reconnected { port: new_port });
                                                let _ = event_tx.send(SshEvent::ProxySettingsChanged);
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
                                        let _ = event_tx.send(SshEvent::ReconnectFailed);
                                        let _ = event_tx.send(SshEvent::Disconnected);
                                    }
                                } else {
                                    let _ = event_tx.send(SshEvent::Disconnected);
                                }

                                break;
                            }
                        }
                        _ = shutdown.changed() => {
                            break;
                        }
                    }
                }
            });

            let _ = this.event_tx.send(SshEvent::Connected {
                host: config.host.clone(),
                port: local_port,
            });
            let _ = this.event_tx.send(SshEvent::ProxySettingsChanged);

            let mut state = this.state.lock().await;
            *state = Some(SshTunnelState {
                local_port,
                remote_host: config.host,
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
        })
    }

    pub async fn disconnect(&self) -> Result<(), String> {
        *self.manually_disconnected.lock().await = true;
        self.disconnect_inner().await;
        let _ = self.event_tx.send(SshEvent::Disconnected);
        let _ = self.event_tx.send(SshEvent::ProxySettingsChanged);
        Ok(())
    }

    async fn disconnect_inner(&self) {
        let mut state = self.state.lock().await;
        if let Some(tunnel) = state.take() {
            let _ = tunnel.shutdown_tx.send(true);
            tunnel.listener_handle.abort();
            tunnel.keepalive_handle.abort();
            let _ = tunnel
                .ssh_handle
                .disconnect(Disconnect::ByApplication, "user disconnect", "en")
                .await;
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

/// Remove a known host entry from the known_hosts file.
pub fn remove_known_host(
    known_hosts_path: &std::path::Path,
    host: &str,
    port: u16,
) -> Result<(), String> {
    let prefix = format!("{}:{} ", host, port);
    if known_hosts_path.exists() {
        let content = std::fs::read_to_string(known_hosts_path).map_err(|e| e.to_string())?;
        let filtered: String = content
            .lines()
            .filter(|l| !l.starts_with(&prefix))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        std::fs::write(known_hosts_path, filtered).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_new_manager_disconnected() {
        let manager = SshTunnelManager::new();
        assert!(!manager.is_connected().await);
        assert!(manager.get_proxy_url().await.is_none());

        let status = manager.get_status().await;
        assert!(!status.connected);
        assert!(status.local_port.is_none());
        assert!(status.remote_host.is_none());
    }

    #[tokio::test]
    async fn test_subscribe() {
        let manager = Arc::new(SshTunnelManager::new());
        let mut rx = manager.subscribe();
        let _ = manager.event_tx.send(SshEvent::Disconnected);
        let event = rx.recv().await.unwrap();
        assert!(matches!(event, SshEvent::Disconnected));
    }

    #[tokio::test]
    async fn test_remove_known_host() {
        let dir = std::env::temp_dir().join("uni-ssh-test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("known_hosts_test");

        std::fs::write(
            &path,
            "example.com:22 AAAAB3...\nother.com:22 BBBBB4...\nexample.com:2222 CCCCC5...\n",
        )
        .unwrap();

        remove_known_host(&path, "example.com", 22).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(!content.contains("example.com:22 "));
        assert!(content.contains("other.com:22 "));
        assert!(content.contains("example.com:2222 "));

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[tokio::test]
    async fn test_remove_known_host_missing_file() {
        let path = std::env::temp_dir().join("uni-ssh-test-nonexistent-known-hosts");
        let result = remove_known_host(&path, "example.com", 22);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_disconnect_when_not_connected() {
        let manager = SshTunnelManager::new();
        let result = manager.disconnect().await;
        assert!(result.is_ok());
    }
}
