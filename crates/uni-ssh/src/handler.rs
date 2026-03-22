use std::path::PathBuf;

use async_trait::async_trait;
use russh::client;
use russh::keys::key::PublicKey;
use russh_keys::PublicKeyBase64;
use tokio::sync::broadcast;

use crate::types::SshEvent;

pub(crate) struct SshHandler {
    pub host: String,
    pub port: u16,
    pub known_hosts_path: PathBuf,
    pub event_tx: broadcast::Sender<SshEvent>,
}

#[async_trait]
impl client::Handler for SshHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        let host_entry = format!("{}:{}", self.host, self.port);
        let key_b64 = server_public_key.public_key_base64();

        let contents = std::fs::read_to_string(&self.known_hosts_path).unwrap_or_default();
        for line in contents.lines() {
            if let Some(stored_key) = line.strip_prefix(&format!("{} ", host_entry)) {
                if stored_key == key_b64 {
                    log::info!("[ssh-tunnel] Host key verified for {}", host_entry);
                    return Ok(true);
                } else {
                    log::warn!("[ssh-tunnel] HOST KEY CHANGED for {}", host_entry);
                    let _ = self.event_tx.send(SshEvent::HostKeyChanged {
                        host: self.host.clone(),
                        port: self.port,
                    });
                    return Ok(false);
                }
            }
        }

        // First connection — TOFU: save and trust
        if let Some(parent) = self.known_hosts_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.known_hosts_path)
            .map_err(russh::Error::IO)?;
        writeln!(f, "{} {}", host_entry, key_b64).map_err(russh::Error::IO)?;
        log::info!("[ssh-tunnel] TOFU: saved host key for {}", host_entry);
        Ok(true)
    }
}
