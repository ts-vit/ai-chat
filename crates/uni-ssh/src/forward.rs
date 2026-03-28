use std::sync::Arc;

use russh::client::Handle;
use tokio::net::TcpStream;
use tokio::sync::watch;

use crate::handler::SshHandler;
use crate::proxy::proxy_bidirectional;

/// Handle a single port-forwarded connection: open SSH direct-tcpip channel
/// and proxy data bidirectionally between the local TCP stream and the remote endpoint.
pub(crate) async fn handle_port_forward_connection(
    stream: TcpStream,
    ssh: Arc<Handle<SshHandler>>,
    remote_host: String,
    remote_port: u16,
    shutdown: watch::Receiver<bool>,
) -> Result<(), String> {
    eprintln!(
        "[port-forward] New connection, opening channel to {}:{}",
        remote_host, remote_port
    );

    let channel = ssh
        .channel_open_direct_tcpip(&remote_host, remote_port as u32, "127.0.0.1", 0)
        .await
        .map_err(|e| {
            eprintln!(
                "[port-forward] Failed to open channel to {}:{}: {}",
                remote_host, remote_port, e
            );
            format!(
                "Failed to open direct-tcpip channel to {}:{}: {}",
                remote_host, remote_port, e
            )
        })?;

    eprintln!(
        "[port-forward] Channel opened to {}:{}",
        remote_host, remote_port
    );

    proxy_bidirectional(stream, channel, shutdown).await;

    Ok(())
}
