use std::sync::Arc;

use russh::client::{self, Handle};
use russh::{Channel, ChannelMsg};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{watch, Mutex};

use crate::handler::SshHandler;

/// Handle a single port-forwarded connection: open SSH direct-tcpip channel
/// and proxy data bidirectionally between the local TCP stream and the remote endpoint.
pub(crate) async fn handle_port_forward_connection(
    stream: TcpStream,
    ssh: Arc<Handle<SshHandler>>,
    remote_host: String,
    remote_port: u16,
    shutdown: watch::Receiver<bool>,
) -> Result<(), String> {
    let channel = ssh
        .channel_open_direct_tcpip(&remote_host, remote_port as u32, "127.0.0.1", 0)
        .await
        .map_err(|e| {
            format!(
                "Failed to open direct-tcpip channel to {}:{}: {}",
                remote_host, remote_port, e
            )
        })?;

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
