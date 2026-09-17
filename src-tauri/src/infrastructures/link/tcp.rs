//! TCP client transport for network bridges: FPGA / Ethernet boards, serial-to-TCP
//! gateways, or a remote PC forwarding a COM port. The peer is expected to relay
//! the raw protocol byte stream unchanged.

use super::{BoxFuture, LinkCapabilities, LinkIo, Transport, TransportKind};
use std::time::Duration;
use tokio::net::TcpStream;

pub const TCP_CONNECT_TIMEOUT_MS: u64 = 5_000;

pub struct TcpTransport {
    host: String,
    port: u16,
}

impl TcpTransport {
    pub fn new(host: String, port: u16) -> Self {
        Self { host, port }
    }

    pub const CAPABILITIES: LinkCapabilities = LinkCapabilities {
        kind: TransportKind::Tcp,
        stream_oriented: true,
        full_duplex: true,
    };
}

impl Transport for TcpTransport {
    fn label(&self) -> String {
        format!("tcp://{}:{}", self.host, self.port)
    }

    fn open(&self) -> BoxFuture<'_, Result<LinkIo, String>> {
        Box::pin(async move {
            let address = (self.host.as_str(), self.port);
            let connect = TcpStream::connect(address);
            let stream =
                tokio::time::timeout(Duration::from_millis(TCP_CONNECT_TIMEOUT_MS), connect)
                    .await
                    .map_err(|_| {
                        format!(
                            "tcp connect to {} timed out after {TCP_CONNECT_TIMEOUT_MS} ms",
                            self.label()
                        )
                    })?
                    .map_err(|error| format!("tcp connect to {} failed: {error}", self.label()))?;
            // Telemetry frames are small and latency sensitive; never let Nagle
            // hold back a command that must land inside the half-duplex window.
            if let Err(error) = stream.set_nodelay(true) {
                log::warn!("failed to set TCP_NODELAY on {}: {error}", self.label());
            }
            log::info!("tcp link connected: {}", self.label());
            Ok(LinkIo {
                stream: Box::new(stream),
                capabilities: Self::CAPABILITIES,
                label: self.label(),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn tcp_transport_round_trips_bytes_through_a_local_listener() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut buf = [0_u8; 4];
            socket.read_exact(&mut buf).await.expect("read");
            socket.write_all(&[0xA5, 0x5A]).await.expect("write");
            buf
        });

        let transport = TcpTransport::new("127.0.0.1".to_string(), port);
        let mut link = transport.open().await.expect("open");
        assert_eq!(link.capabilities, TcpTransport::CAPABILITIES);
        assert_eq!(link.label, format!("tcp://127.0.0.1:{port}"));
        link.stream.write_all(&[1, 2, 3, 4]).await.expect("write");
        let mut reply = [0_u8; 2];
        link.stream.read_exact(&mut reply).await.expect("read");
        assert_eq!(reply, [0xA5, 0x5A]);
        assert_eq!(server.await.expect("server"), [1, 2, 3, 4]);
    }

    #[tokio::test]
    async fn tcp_transport_reports_refused_connections() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("bind");
        let port = listener.local_addr().expect("addr").port();
        drop(listener);
        let transport = TcpTransport::new("127.0.0.1".to_string(), port);
        let error = transport.open().await.err().expect("must fail");
        assert!(error.contains("tcp connect"), "{error}");
    }
}
