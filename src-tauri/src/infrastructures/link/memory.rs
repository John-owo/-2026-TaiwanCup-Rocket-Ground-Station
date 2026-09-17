//! In-memory transport backed by `tokio::io::duplex`. The ground station side
//! receives a normal [`LinkIo`]; the test or replay tool keeps the far end and
//! can inject telemetry bytes or observe uplink commands without any hardware.
//!
//! Only tests construct this today; the replay / simulator feature will be the
//! first production user.
#![allow(dead_code)]

use super::{BoxFuture, LinkCapabilities, LinkIo, Transport, TransportKind};
use std::sync::Mutex;
use tokio::io::DuplexStream;

pub const DEFAULT_MEMORY_BUFFER_BYTES: usize = 4_096;

pub struct MemoryTransport {
    label: String,
    near_end: Mutex<Option<DuplexStream>>,
}

impl MemoryTransport {
    pub const CAPABILITIES: LinkCapabilities = LinkCapabilities {
        kind: TransportKind::Memory,
        stream_oriented: true,
        full_duplex: true,
    };

    /// Returns the transport (ground station side) and the far end the caller
    /// drives as the simulated radio.
    pub fn pair(label: impl Into<String>) -> (Self, DuplexStream) {
        Self::pair_with_capacity(label, DEFAULT_MEMORY_BUFFER_BYTES)
    }

    pub fn pair_with_capacity(label: impl Into<String>, capacity: usize) -> (Self, DuplexStream) {
        let (near_end, far_end) = tokio::io::duplex(capacity);
        (
            Self {
                label: label.into(),
                near_end: Mutex::new(Some(near_end)),
            },
            far_end,
        )
    }
}

impl Transport for MemoryTransport {
    fn label(&self) -> String {
        format!("memory://{}", self.label)
    }

    fn open(&self) -> BoxFuture<'_, Result<LinkIo, String>> {
        Box::pin(async move {
            let stream = self
                .near_end
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .take()
                .ok_or_else(|| format!("memory link {} was already opened", self.label()))?;
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

    #[tokio::test]
    async fn memory_transport_delivers_bytes_both_ways() {
        let (transport, mut radio) = MemoryTransport::pair("unit");
        let mut link = transport.open().await.expect("open");
        assert_eq!(link.label, "memory://unit");
        assert_eq!(link.capabilities, MemoryTransport::CAPABILITIES);

        radio.write_all(&[0xA5, 0x5A, 0x01]).await.expect("inject");
        let mut rx = [0_u8; 3];
        link.stream.read_exact(&mut rx).await.expect("read");
        assert_eq!(rx, [0xA5, 0x5A, 0x01]);

        link.stream.write_all(&[0x10, 0x11]).await.expect("uplink");
        let mut tx = [0_u8; 2];
        radio.read_exact(&mut tx).await.expect("observe");
        assert_eq!(tx, [0x10, 0x11]);
    }

    #[tokio::test]
    async fn memory_transport_can_only_be_opened_once() {
        let (transport, _radio) = MemoryTransport::pair("once");
        transport.open().await.expect("first open");
        let error = transport.open().await.err().expect("second open fails");
        assert!(error.contains("already opened"), "{error}");
    }
}
