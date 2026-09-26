//! Serial port transport: COM ports behind TTL-to-USB bridges, USB CDC devices,
//! or any other OS-level serial device.

use super::{BoxFuture, LinkCapabilities, LinkIo, Transport, TransportKind};
use tokio_serial::SerialPortBuilderExt;

pub struct SerialTransport {
    path: String,
    baud_rate: u32,
}

impl SerialTransport {
    pub fn new(path: String, baud_rate: u32) -> Self {
        Self { path, baud_rate }
    }

    pub const CAPABILITIES: LinkCapabilities = LinkCapabilities {
        kind: TransportKind::Serial,
        stream_oriented: true,
        full_duplex: true,
    };
}

impl Transport for SerialTransport {
    fn label(&self) -> String {
        format!("{} @ {}", self.path, self.baud_rate)
    }

    fn open(&self) -> BoxFuture<'_, Result<LinkIo, String>> {
        Box::pin(async move {
            let stream = tokio_serial::new(&self.path, self.baud_rate)
                .open_native_async()
                .map_err(|error| format!("failed to open serial port: {error}"))?;
            log::info!("serial port connected: {}", self.label());
            Ok(LinkIo {
                stream: Box::new(stream),
                capabilities: Self::CAPABILITIES,
                label: self.label(),
            })
        })
    }
}
