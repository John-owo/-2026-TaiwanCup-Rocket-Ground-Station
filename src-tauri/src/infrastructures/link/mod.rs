//! Link layer: pluggable byte transports between the ground station and the radio.
//!
//! The protocol layer (`infrastructures::serial::parser` / `command`) only needs a
//! bidirectional byte pipe. Today that pipe is a COM port behind a TTL-to-USB
//! bridge, but the same protocol can arrive over TCP from an FPGA / Ethernet
//! bridge, a USB CDC device, or an in-memory pipe used by tests and replay tools.
//!
//! Layering:
//!
//! ```text
//! Tauri commands
//!   └─ LinkReceiver (protocol session: framing, half-duplex window, stats)
//!       └─ LinkIo (opened pipe + capabilities)        ← this module
//!           └─ Transport impls: serial / tcp / memory ← this module
//! ```
//!
//! Radio-module specific behaviour (E22 RSSI byte, fixed-mode address header,
//! AT command configuration) belongs to a future adapter layer that sits between
//! `LinkIo` and the protocol parser. Nothing in this module knows about LoRa.

pub mod memory;
pub mod serial;
pub mod tcp;

use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use tokio::io::{AsyncRead, AsyncWrite};

/// Any full-duplex async byte pipe. Serial ports, TCP sockets and in-memory
/// duplex streams all satisfy this automatically.
pub trait AsyncByteStream: AsyncRead + AsyncWrite + Send + Unpin {}
impl<T: AsyncRead + AsyncWrite + Send + Unpin> AsyncByteStream for T {}

pub type BoxedByteStream = Box<dyn AsyncByteStream>;
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Which physical / logical carrier the bytes travel over.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportKind {
    Serial,
    Tcp,
    Memory,
}

/// What the protocol session may assume about an opened transport.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkCapabilities {
    pub kind: TransportKind,
    /// `true` when bytes arrive as a continuous stream that may be split or
    /// coalesced arbitrarily (serial, TCP). A datagram transport would be `false`.
    pub stream_oriented: bool,
    /// `true` when the transport itself can read and write simultaneously.
    /// The radio behind it is usually still half duplex, so the protocol
    /// session keeps its uplink window regardless of this flag.
    pub full_duplex: bool,
}

/// An opened byte pipe together with its capabilities.
pub struct LinkIo {
    pub stream: BoxedByteStream,
    pub capabilities: LinkCapabilities,
    /// Operator-facing description, e.g. `COM3 @ 115200` or `tcp://10.0.0.5:5000`.
    pub label: String,
}

/// A transport knows how to open a byte pipe to the radio hardware.
///
/// Implementations are cheap value objects describing *where* to connect; the
/// expensive work happens in [`Transport::open`]. The trait is object safe so
/// the active transport can be chosen at runtime from configuration.
pub trait Transport: Send + Sync {
    fn label(&self) -> String;
    fn open(&self) -> BoxFuture<'_, Result<LinkIo, String>>;
}

/// Serializable description of a transport. The UI sends one of these when a
/// monitoring run starts; the backend turns it into a [`Transport`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TransportConfig {
    /// COM port (TTL-to-USB bridge, USB CDC device, or any OS serial port).
    Serial { path: String, baud_rate: u32 },
    /// TCP client to a network bridge (FPGA / Ethernet / serial-to-TCP gateway).
    Tcp { host: String, port: u16 },
}

impl TransportConfig {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Serial { path, baud_rate } => {
                if path.trim().is_empty() {
                    return Err("serial path must not be empty".to_string());
                }
                if *baud_rate == 0 {
                    return Err("baud rate must be greater than zero".to_string());
                }
                Ok(())
            }
            Self::Tcp { host, port } => {
                if host.trim().is_empty() {
                    return Err("tcp host must not be empty".to_string());
                }
                if *port == 0 {
                    return Err("tcp port must be greater than zero".to_string());
                }
                Ok(())
            }
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::Serial { path, baud_rate } => format!("{} @ {baud_rate}", path.trim()),
            Self::Tcp { host, port } => format!("tcp://{}:{port}", host.trim()),
        }
    }

    pub fn into_transport(self) -> Box<dyn Transport> {
        match self {
            Self::Serial { path, baud_rate } => Box::new(serial::SerialTransport::new(
                path.trim().to_string(),
                baud_rate,
            )),
            Self::Tcp { host, port } => {
                Box::new(tcp::TcpTransport::new(host.trim().to_string(), port))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serial_config_requires_path_and_baud() {
        assert!(TransportConfig::Serial {
            path: "  ".to_string(),
            baud_rate: 115_200,
        }
        .validate()
        .is_err());
        assert!(TransportConfig::Serial {
            path: "COM3".to_string(),
            baud_rate: 0,
        }
        .validate()
        .is_err());
        let ok = TransportConfig::Serial {
            path: " COM3 ".to_string(),
            baud_rate: 115_200,
        };
        assert!(ok.validate().is_ok());
        assert_eq!(ok.label(), "COM3 @ 115200");
    }

    #[test]
    fn tcp_config_requires_host_and_port() {
        assert!(TransportConfig::Tcp {
            host: String::new(),
            port: 5000,
        }
        .validate()
        .is_err());
        assert!(TransportConfig::Tcp {
            host: "10.0.0.5".to_string(),
            port: 0,
        }
        .validate()
        .is_err());
        let ok = TransportConfig::Tcp {
            host: "10.0.0.5".to_string(),
            port: 5000,
        };
        assert!(ok.validate().is_ok());
        assert_eq!(ok.label(), "tcp://10.0.0.5:5000");
    }

    #[test]
    fn config_round_trips_through_json_with_kind_tag() {
        let config = TransportConfig::Serial {
            path: "COM7".to_string(),
            baud_rate: 57_600,
        };
        let json = serde_json::to_string(&config).expect("serialize");
        assert!(json.contains("\"kind\":\"serial\""));
        let parsed: TransportConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, config);

        let tcp: TransportConfig =
            serde_json::from_str(r#"{"kind":"tcp","host":"fpga.local","port":9000}"#)
                .expect("deserialize tcp");
        assert_eq!(
            tcp,
            TransportConfig::Tcp {
                host: "fpga.local".to_string(),
                port: 9000,
            }
        );
    }

    #[test]
    fn config_builds_matching_transport_labels() {
        let serial = TransportConfig::Serial {
            path: "COM3".to_string(),
            baud_rate: 115_200,
        }
        .into_transport();
        assert_eq!(serial.label(), "COM3 @ 115200");
        let tcp = TransportConfig::Tcp {
            host: "192.168.4.1".to_string(),
            port: 5000,
        }
        .into_transport();
        assert_eq!(tcp.label(), "tcp://192.168.4.1:5000");
    }
}
