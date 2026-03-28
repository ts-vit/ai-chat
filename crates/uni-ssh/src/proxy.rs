use russh::client;
use russh::{Channel, ChannelMsg};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::watch;

/// Proxy data bidirectionally between a TCP stream and an SSH channel.
///
/// Uses `tokio::select!` in a single task to avoid deadlock — no Mutex needed.
/// Both TCP read and SSH channel wait are polled concurrently without holding any lock.
pub(crate) async fn proxy_bidirectional(
    stream: TcpStream,
    mut channel: Channel<client::Msg>,
    mut shutdown: watch::Receiver<bool>,
) {
    let (mut tcp_read, mut tcp_write) = stream.into_split();
    let mut buf = [0u8; 8192];

    loop {
        tokio::select! {
            result = tcp_read.read(&mut buf) => {
                match result {
                    Ok(0) => {
                        eprintln!("[proxy] TCP->SSH: connection closed");
                        let _ = channel.eof().await;
                        break;
                    }
                    Ok(n) => {
                        eprintln!("[proxy] TCP->SSH: read {} bytes", n);
                        if channel.data(&buf[..n]).await.is_err() {
                            eprintln!("[proxy] TCP->SSH: channel data send error");
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("[proxy] TCP->SSH: read error: {}", e);
                        break;
                    }
                }
            }
            msg = channel.wait() => {
                match msg {
                    Some(ChannelMsg::Data { data }) => {
                        eprintln!("[proxy] SSH->TCP: received {} bytes", data.len());
                        if tcp_write.write_all(&data).await.is_err() {
                            eprintln!("[proxy] SSH->TCP: write error");
                            break;
                        }
                    }
                    Some(ChannelMsg::Eof) => {
                        eprintln!("[proxy] SSH->TCP: EOF");
                        break;
                    }
                    None => {
                        eprintln!("[proxy] SSH->TCP: channel closed");
                        break;
                    }
                    other => {
                        eprintln!("[proxy] SSH->TCP: other message: {:?}", other);
                    }
                }
            }
            _ = shutdown.changed() => {
                eprintln!("[proxy] shutdown signal received");
                break;
            }
        }
    }
    eprintln!("[proxy] bidirectional proxy ended");
}
