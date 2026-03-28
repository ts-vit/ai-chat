use std::sync::Arc;

use russh::client::Handle;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::watch;

use crate::handler::SshHandler;
use crate::proxy::proxy_bidirectional;

/// Handle a single SOCKS5 connection: parse SOCKS5 handshake, open SSH direct-tcpip channel,
/// and proxy data bidirectionally.
pub(crate) async fn handle_socks5_connection(
    mut stream: TcpStream,
    ssh: Arc<Handle<SshHandler>>,
    shutdown: watch::Receiver<bool>,
) -> Result<(), String> {
    // --- SOCKS5 handshake ---
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
            (host, u16::from_be_bytes(port_buf))
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
            (host, u16::from_be_bytes(port_buf))
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
            (host, u16::from_be_bytes(port_buf))
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

    proxy_bidirectional(stream, channel, shutdown).await;

    Ok(())
}
