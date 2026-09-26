//! Hardware-free integration checks using the real transport, parser and
//! command manager. These do not exercise Tauri, persistence or radio timing.
use super::{memory::MemoryTransport, LinkIo, Transport};
use crate::infrastructures::serial::command::{CommandManager, CommandRequest};
use crate::infrastructures::serial::parser::{PacketParser, ParseResult};
use crate::models::response::ParsedFrame;
use crate::services::serial::Parser;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const V1: &str = include_str!("../../../../docs/protocol/test_vectors_v1.json");
const V2: &str = include_str!("../../../../docs/protocol/test_vectors_v2.json");

fn vector(document: &str, id: &str) -> Vec<u8> {
    let document: serde_json::Value = serde_json::from_str(document).expect("vectors");
    let hex = document["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|value| value["id"] == id)
        .expect("named vector")["frame_hex"]
        .as_str()
        .unwrap();
    (0..hex.len())
        .step_by(2)
        .map(|offset| u8::from_str_radix(&hex[offset..offset + 2], 16).unwrap())
        .collect()
}

async fn receive_to_eof(link: &mut LinkIo) -> (Vec<ParsedFrame>, Vec<String>) {
    let mut parser = PacketParser::default();
    let mut frames = Vec::new();
    let mut errors = Vec::new();
    let mut buffer = [0; 7];
    loop {
        let count = link.stream.read(&mut buffer).await.expect("receive");
        if count == 0 {
            return (frames, errors);
        }
        for &byte in &buffer[..count] {
            match parser.sink(byte) {
                ParseResult::Complete(frame) => frames.push(frame),
                ParseResult::ParseError(error) => errors.push(error),
                _ => {}
            }
        }
    }
}

async fn replay(bytes: Vec<u8>, chunk_size: usize) -> (Vec<ParsedFrame>, Vec<String>) {
    // A tiny buffer forces backpressure and splits headers/payloads across reads.
    let (transport, mut radio) = MemoryTransport::pair_with_capacity("protocol-replay", 8);
    let mut link = transport.open().await.expect("open memory link");
    tokio::time::timeout(Duration::from_secs(5), async {
        let send = async {
            for chunk in bytes.chunks(chunk_size) {
                radio.write_all(chunk).await.expect("inject replay");
            }
            radio.shutdown().await.expect("finish replay");
        };
        let (_, received) = tokio::join!(send, receive_to_eof(&mut link));
        received
    })
    .await
    .expect("memory replay must not hang")
}

#[tokio::test]
async fn fragmented_and_coalesced_v1_v2_telemetry_survives_memory_transport() {
    let mut bytes = vector(V1, "telemetry_nominal");
    bytes.extend(vector(V2, "telemetry_nominal_v2"));
    for chunk_size in [1, 3, bytes.len()] {
        let (frames, errors) = replay(bytes.clone(), chunk_size).await;
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(frames.len(), 2);
        for (index, frame) in frames.iter().enumerate() {
            let ParsedFrame::Telemetry(payload) = frame else {
                panic!("expected telemetry");
            };
            assert_eq!(payload.protocol_version, index as u8 + 1);
            assert_eq!(payload.session_id, 0x1234_5678);
            assert_eq!(payload.frame_seq, 42);
            assert!((payload.altitude - 123.45).abs() < 0.01);
        }
    }
}

#[tokio::test]
async fn corrupted_replay_frame_does_not_hide_the_following_telemetry() {
    let mut bytes = vec![0, 0xA5, 0, 0xFF];
    bytes.extend(vector(V2, "telemetry_crc_error_v2"));
    bytes.extend(vector(V2, "telemetry_nominal_v2"));
    let (frames, errors) = replay(bytes, 5).await;
    assert_eq!(frames.len(), 1);
    assert!(
        errors.iter().any(|error| error.contains("CRC")),
        "{errors:?}"
    );
    let ParsedFrame::Telemetry(payload) = &frames[0] else {
        panic!("expected recovered telemetry");
    };
    assert_eq!(payload.session_id, 0x1234_5678);
}

async fn receive_frame(link: &mut LinkIo, length: usize) -> ParsedFrame {
    let mut bytes = vec![0; length];
    tokio::time::timeout(Duration::from_secs(5), link.stream.read_exact(&mut bytes))
        .await
        .expect("frame timeout")
        .expect("frame bytes");
    let mut parser = PacketParser::default();
    for byte in bytes {
        match parser.sink(byte) {
            ParseResult::Complete(frame) => return frame,
            ParseResult::ParseError(error) => panic!("invalid frame: {error}"),
            _ => {}
        }
    }
    panic!("incomplete frame");
}

#[tokio::test]
async fn timer_retry_and_matching_ack_round_trip_over_memory_transport() {
    let (transport, mut radio) = MemoryTransport::pair("timer-ack");
    let mut link = transport.open().await.expect("open");
    let mut manager = CommandManager::default();
    manager
        .request(CommandRequest::SetTimer(30))
        .expect("queue timer");
    assert!(manager.next_transmission().unwrap().is_none());

    let telemetry = vector(V2, "telemetry_nominal_v2");
    radio.write_all(&telemetry).await.expect("telemetry");
    let ParsedFrame::Telemetry(payload) = receive_frame(&mut link, telemetry.len()).await else {
        panic!("expected telemetry");
    };
    manager.observe_telemetry(&payload);

    for id in ["set_timer_nominal_v2", "set_timer_retry_v2"] {
        let transmission = manager.next_transmission().unwrap().expect("pending timer");
        link.stream
            .write_all(&transmission.bytes)
            .await
            .expect("uplink");
        let expected = vector(V2, id);
        let mut observed = vec![0; expected.len()];
        tokio::time::timeout(Duration::from_secs(5), radio.read_exact(&mut observed))
            .await
            .expect("uplink timeout")
            .expect("observe uplink");
        assert_eq!(observed, expected);
    }

    let ack = vector(V2, "ack_executed_v2");
    radio.write_all(&ack).await.expect("send ACK");
    let ParsedFrame::Ack(ack) = receive_frame(&mut link, ack.len()).await else {
        panic!("expected ACK");
    };
    let status = manager.handle_ack(&ack);
    assert_eq!(status.status, "acked");
    assert_eq!(status.attempts, 2);
    assert!(manager.next_transmission().unwrap().is_none());
}
