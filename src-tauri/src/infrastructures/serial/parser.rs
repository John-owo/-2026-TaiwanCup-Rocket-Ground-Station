use crate::infrastructures::serial::crc::crc16_ccitt;
use crate::models::response::{AckPayload, ParsedFrame, TelemetryPayload};
use crate::services::serial::{Decoder, Parser};

pub const MAGIC_0: u8 = 0xA5;
pub const MAGIC_1: u8 = 0x5A;
pub const PROTOCOL_V1: u8 = 0x01;
pub const PROTOCOL_V2: u8 = 0x02;
pub const FRAME_TYPE_TELEMETRY: u8 = 0x01;
pub const FRAME_TYPE_SET_TIMER: u8 = 0x10;
pub const FRAME_TYPE_FORCE_RELEASE: u8 = 0x11;
pub const FRAME_TYPE_ACK: u8 = 0x7F;
pub const V1_HEADER_LENGTH: usize = 20;
pub const V2_HEADER_LENGTH: usize = 14;
pub const CRC_LENGTH: usize = 2;
pub const MAX_PAYLOAD_LENGTH: usize = 96;
pub const V1_TELEMETRY_PAYLOAD_LENGTH: usize = 72;
pub const V2_TELEMETRY_PAYLOAD_LENGTH: usize = 47;
pub const V1_TELEMETRY_FRAME_LENGTH: usize = 94;
pub const V2_TELEMETRY_FRAME_LENGTH: usize = 63;

const SENSOR_MPU6050_VALID: u8 = 0x01;
const SENSOR_BMP280_VALID: u8 = 0x02;
const SENSOR_GPS_LOCATION_FRESH: u8 = 0x04;
const SENSOR_GPS_SPEED_FRESH: u8 = 0x08;

#[derive(Debug)]
pub enum ParseResult {
    Incomplete,
    Complete(ParsedFrame),
    IgnoredFrame(u8),
    ParseError(String),
}

pub struct PacketParser {
    buffer: Vec<u8>,
    decoder: Box<dyn Decoder<ResultType = TelemetryPayload> + Send + Sync>,
}

struct HeaderMeta {
    version: u8,
    frame_type: u8,
    flags: u8,
    header_length: usize,
    payload_length: usize,
    session_id: u32,
    frame_seq: Option<u32>,
    command_id: u32,
}

impl PacketParser {
    fn align_to_magic(&mut self) {
        while self.buffer.len() >= 2
            && (self.buffer[0] != MAGIC_0 || self.buffer[1] != MAGIC_1)
        {
            self.buffer.remove(0);
        }
        if self.buffer.len() == 1 && self.buffer[0] != MAGIC_0 {
            self.buffer.clear();
        }
    }

    fn reject_candidate(&mut self, message: impl Into<String>) -> ParseResult {
        if !self.buffer.is_empty() {
            self.buffer.remove(0);
        }
        ParseResult::ParseError(message.into())
    }

    fn header(&self) -> Result<Option<HeaderMeta>, String> {
        if self.buffer.len() < 3 {
            return Ok(None);
        }
        let version = self.buffer[2];
        let required = match version {
            PROTOCOL_V1 => V1_HEADER_LENGTH,
            PROTOCOL_V2 => V2_HEADER_LENGTH,
            _ => return Err(format!("unsupported protocol version {version}")),
        };
        if self.buffer.len() < required {
            return Ok(None);
        }
        let frame_type = self.buffer[3];
        let flags = self.buffer[4];
        let (payload_length, session_id, frame_seq, command_id) = if version == PROTOCOL_V1 {
            if self.buffer[5] as usize != V1_HEADER_LENGTH {
                return Err(format!("invalid v1 header length {}", self.buffer[5]));
            }
            (
                read_u16(&self.buffer, 6) as usize,
                read_u32(&self.buffer, 8),
                Some(read_u32(&self.buffer, 12)),
                read_u32(&self.buffer, 16),
            )
        } else {
            let message_id = read_u32(&self.buffer, 10);
            (
                self.buffer[5] as usize,
                read_u32(&self.buffer, 6),
                (frame_type == FRAME_TYPE_TELEMETRY).then_some(message_id),
                if frame_type == FRAME_TYPE_TELEMETRY { 0 } else { message_id },
            )
        };
        Ok(Some(HeaderMeta {
            version,
            frame_type,
            flags,
            header_length: required,
            payload_length,
            session_id,
            frame_seq,
            command_id,
        }))
    }

    fn expected_payload_length(version: u8, frame_type: u8) -> Option<usize> {
        match (version, frame_type) {
            (PROTOCOL_V1, FRAME_TYPE_TELEMETRY) => Some(V1_TELEMETRY_PAYLOAD_LENGTH),
            (PROTOCOL_V2, FRAME_TYPE_TELEMETRY) => Some(V2_TELEMETRY_PAYLOAD_LENGTH),
            (_, FRAME_TYPE_SET_TIMER) => Some(4),
            (_, FRAME_TYPE_FORCE_RELEASE) => Some(0),
            (PROTOCOL_V1, FRAME_TYPE_ACK) => Some(8),
            (PROTOCOL_V2, FRAME_TYPE_ACK) => Some(7),
            _ => None,
        }
    }

    fn try_parse(&mut self) -> ParseResult {
        self.align_to_magic();
        let header = match self.header() {
            Ok(Some(header)) => header,
            Ok(None) => return ParseResult::Incomplete,
            Err(error) => return self.reject_candidate(error),
        };
        if header.flags & !0x01 != 0 {
            return self.reject_candidate(format!(
                "reserved flag bits are set: 0x{:02X}",
                header.flags
            ));
        }
        if header.payload_length > MAX_PAYLOAD_LENGTH {
            return self.reject_candidate(format!(
                "payload length {} exceeds maximum",
                header.payload_length
            ));
        }
        if header.session_id == 0 {
            return self.reject_candidate("session_id must be non-zero");
        }
        if header.version == PROTOCOL_V2
            && header.frame_seq.unwrap_or(header.command_id) == 0
        {
            return self.reject_candidate("v2 message_id must be non-zero");
        }

        let total_length = header.header_length + header.payload_length + CRC_LENGTH;
        if self.buffer.len() < total_length {
            return ParseResult::Incomplete;
        }
        let frame = &self.buffer[..total_length];
        let received_crc = read_u16(frame, total_length - CRC_LENGTH);
        let calculated_crc = crc16_ccitt(&frame[2..total_length - CRC_LENGTH]);
        if received_crc != calculated_crc {
            return self.reject_candidate(format!(
                "CRC verification failed: received 0x{received_crc:04X}, calculated 0x{calculated_crc:04X}"
            ));
        }
        if let Some(expected) = Self::expected_payload_length(header.version, header.frame_type) {
            if header.payload_length != expected {
                self.buffer.drain(..total_length);
                return ParseResult::ParseError(format!(
                    "protocol v{} frame type 0x{:02X} requires {expected} payload bytes, got {}",
                    header.version, header.frame_type, header.payload_length
                ));
            }
        }

        let frame = self.buffer.drain(..total_length).collect::<Vec<_>>();
        match header.frame_type {
            FRAME_TYPE_TELEMETRY => {
                if header.flags != 0 || header.frame_seq == Some(0) {
                    return ParseResult::ParseError("invalid TELEMETRY identity/flags".to_string());
                }
                match self.decoder.decode(&frame) {
                    Ok(payload) => ParseResult::Complete(ParsedFrame::Telemetry(payload)),
                    Err(error) => ParseResult::ParseError(format!("decode error: {error}")),
                }
            }
            FRAME_TYPE_ACK => {
                if header.flags != 0 || header.command_id == 0 {
                    return ParseResult::ParseError("invalid ACK identity/flags".to_string());
                }
                match decode_ack(&frame, &header) {
                    Ok(payload) => ParseResult::Complete(ParsedFrame::Ack(payload)),
                    Err(error) => ParseResult::ParseError(format!("decode error: {error}")),
                }
            }
            FRAME_TYPE_SET_TIMER | FRAME_TYPE_FORCE_RELEASE if header.command_id == 0 => {
                ParseResult::ParseError("command_id must be non-zero".to_string())
            }
            _ => ParseResult::IgnoredFrame(header.frame_type),
        }
    }
}

fn decode_ack(frame: &[u8], header: &HeaderMeta) -> Result<AckPayload, String> {
    let payload = header.header_length;
    let (timer_state, deploy_state, remaining_s) = if header.version == PROTOCOL_V1 {
        (frame[payload + 2], frame[payload + 3], read_u32(frame, payload + 4))
    } else {
        let status = frame[payload + 2];
        if status & 0xF8 != 0 {
            return Err("v2 ACK reserved status bits must be zero".to_string());
        }
        (status & 0x03, (status >> 2) & 0x01, read_u32(frame, payload + 3))
    };
    validate_state(timer_state, deploy_state, remaining_s, "ACK")?;
    Ok(AckPayload {
        session_id: header.session_id,
        frame_seq: header.frame_seq,
        command_id: header.command_id,
        acked_type: frame[payload],
        result: frame[payload + 1],
        timer_state,
        deploy_state,
        remaining_s,
    })
}

impl Parser for PacketParser {
    fn default() -> Self {
        Self {
            buffer: Vec::with_capacity(V1_HEADER_LENGTH + MAX_PAYLOAD_LENGTH + CRC_LENGTH),
            decoder: Box::new(TelemetryDecoder),
        }
    }

    fn sink(&mut self, byte: u8) -> ParseResult {
        self.buffer.push(byte);
        self.try_parse()
    }

    fn parse_to_payload(&self, frame: &[u8]) -> Result<TelemetryPayload, String> {
        self.decoder.decode(frame)
    }
}

pub struct TelemetryDecoder;

impl Decoder for TelemetryDecoder {
    type ResultType = TelemetryPayload;

    fn decode(&self, frame: &[u8]) -> Result<Self::ResultType, String> {
        match frame.get(2).copied() {
            Some(PROTOCOL_V1) => decode_v1_telemetry(frame),
            Some(PROTOCOL_V2) => decode_v2_telemetry(frame),
            Some(version) => Err(format!("unsupported protocol version {version}")),
            None => Err("truncated telemetry".to_string()),
        }
    }
}

fn decode_v1_telemetry(frame: &[u8]) -> Result<TelemetryPayload, String> {
    if frame.len() != V1_TELEMETRY_FRAME_LENGTH
        || frame[3] != FRAME_TYPE_TELEMETRY
        || frame[4] != 0
        || frame[5] as usize != V1_HEADER_LENGTH
        || read_u16(frame, 6) as usize != V1_TELEMETRY_PAYLOAD_LENGTH
        || read_u32(frame, 16) != 0
    {
        return Err("invalid Protocol v1 TELEMETRY frame".to_string());
    }
    let payload = V1_HEADER_LENGTH;
    if frame[payload + 17..payload + 20] != [0, 0, 0] {
        return Err("v1 TELEMETRY reserved bytes must be zero".to_string());
    }
    let timer_state = frame[payload + 5];
    let deploy_state = frame[payload + 6];
    let sensor_flags = frame[payload + 7];
    let remaining_s = read_u32(frame, payload + 8);
    let last_ack_command_id = read_u32(frame, payload + 12);
    let last_ack_result = frame[payload + 16];
    validate_common_telemetry(timer_state, deploy_state, sensor_flags, remaining_s, last_ack_command_id, last_ack_result)?;
    let mut sensors = [0.0; 13];
    for (index, value) in sensors.iter_mut().enumerate() {
        *value = read_f32(frame, payload + 20 + index * 4);
    }
    validate_invalid_sensor_values(&sensors, sensor_flags)?;
    Ok(payload_from_values(
        PROTOCOL_V1,
        read_u32(frame, 8),
        read_u32(frame, 12),
        read_u32(frame, payload),
        frame[payload + 4],
        timer_state,
        deploy_state,
        sensor_flags,
        remaining_s,
        last_ack_command_id,
        last_ack_result,
        sensors,
    ))
}

fn decode_v2_telemetry(frame: &[u8]) -> Result<TelemetryPayload, String> {
    if frame.len() != V2_TELEMETRY_FRAME_LENGTH
        || frame[3] != FRAME_TYPE_TELEMETRY
        || frame[4] != 0
        || frame[5] as usize != V2_TELEMETRY_PAYLOAD_LENGTH
    {
        return Err("invalid Protocol v2 TELEMETRY frame".to_string());
    }
    let payload = V2_HEADER_LENGTH;
    let status = frame[payload + 5];
    if status & 0x80 != 0 {
        return Err("v2 TELEMETRY reserved status bit must be zero".to_string());
    }
    let timer_state = status & 0x03;
    let deploy_state = (status >> 2) & 0x01;
    let sensor_flags = (status >> 3) & 0x0F;
    let remaining_s = read_u32(frame, payload + 6);
    let last_ack_command_id = read_u32(frame, payload + 10);
    let last_ack_result = frame[payload + 14];
    validate_common_telemetry(timer_state, deploy_state, sensor_flags, remaining_s, last_ack_command_id, last_ack_result)?;
    let sensors = [
        read_i16(frame, payload + 15) as f32 / 100.0,
        read_i16(frame, payload + 17) as f32 / 100.0,
        read_i16(frame, payload + 19) as f32 / 100.0,
        read_i16(frame, payload + 21) as f32 / 10.0,
        read_i16(frame, payload + 23) as f32 / 10.0,
        read_i16(frame, payload + 25) as f32 / 10.0,
        read_i32(frame, payload + 27) as f32 / 1_000_000.0,
        read_i32(frame, payload + 31) as f32 / 1_000_000.0,
        read_i32(frame, payload + 35) as f32 / 100.0,
        read_u16(frame, payload + 39) as f32 / 100.0,
        read_i16(frame, payload + 41) as f32 / 10.0,
        read_u16(frame, payload + 43) as f32 / 10.0,
        read_i16(frame, payload + 45) as f32 / 100.0,
    ];
    validate_invalid_sensor_values(&sensors, sensor_flags)?;
    Ok(payload_from_values(
        PROTOCOL_V2,
        read_u32(frame, 6),
        read_u32(frame, 10),
        read_u32(frame, payload),
        frame[payload + 4],
        timer_state,
        deploy_state,
        sensor_flags,
        remaining_s,
        last_ack_command_id,
        last_ack_result,
        sensors,
    ))
}

#[allow(clippy::too_many_arguments)]
fn payload_from_values(
    protocol_version: u8,
    session_id: u32,
    frame_seq: u32,
    uptime_ms: u32,
    restart_reason: u8,
    timer_state: u8,
    deploy_state: u8,
    sensor_flags: u8,
    remaining_s: u32,
    last_ack_command_id: u32,
    last_ack_result: u8,
    sensors: [f32; 13],
) -> TelemetryPayload {
    TelemetryPayload {
        protocol_version,
        session_id,
        frame_seq,
        uptime_ms,
        restart_reason,
        timer_state,
        deploy_state,
        sensor_flags,
        remaining_s,
        last_ack_command_id,
        last_ack_result,
        x_acceleration: sensors[0],
        y_acceleration: sensors[1],
        z_acceleration: sensors[2],
        x_angular_velocity: sensors[3],
        y_angular_velocity: sensors[4],
        z_angular_velocity: sensors[5],
        longitude: sensors[6],
        latitude: sensors[7],
        altitude: sensors[8],
        ground_speed: sensors[9],
        vertical_velocity: sensors[10],
        air_pressure: sensors[11],
        temperature: sensors[12],
    }
}

fn validate_state(timer_state: u8, deploy_state: u8, remaining_s: u32, label: &str) -> Result<(), String> {
    if timer_state > 2 || deploy_state > 1 {
        return Err(format!("invalid {label} state"));
    }
    if (timer_state == 0 || timer_state == 2) && remaining_s != 0 {
        return Err(format!("{label} UNSET/EXPIRED timer must report remaining_s=0"));
    }
    if timer_state == 1 && remaining_s == 0 {
        return Err(format!("{label} RUNNING timer must report remaining_s>0"));
    }
    Ok(())
}

fn validate_common_telemetry(
    timer_state: u8,
    deploy_state: u8,
    sensor_flags: u8,
    remaining_s: u32,
    last_ack_command_id: u32,
    last_ack_result: u8,
) -> Result<(), String> {
    if sensor_flags & 0xF0 != 0 {
        return Err("reserved sensor flag bits are set".to_string());
    }
    validate_state(timer_state, deploy_state, remaining_s, "TELEMETRY")?;
    if (last_ack_command_id == 0) != (last_ack_result == 0xFF) {
        return Err("last ACK id/result presence is inconsistent".to_string());
    }
    Ok(())
}

fn validate_invalid_sensor_values(sensors: &[f32; 13], sensor_flags: u8) -> Result<(), String> {
    let groups: &[(u8, &[usize])] = &[
        (SENSOR_MPU6050_VALID, &[0, 1, 2, 3, 4, 5]),
        (SENSOR_GPS_LOCATION_FRESH, &[6, 7]),
        (SENSOR_BMP280_VALID, &[8, 10, 11, 12]),
        (SENSOR_GPS_SPEED_FRESH, &[9]),
    ];
    for &(flag, indexes) in groups {
        if sensor_flags & flag == 0 && indexes.iter().any(|&index| sensors[index].to_bits() != 0) {
            return Err(format!("sensor group 0x{flag:02X} must be +0 when invalid"));
        }
    }
    if sensor_flags & SENSOR_GPS_LOCATION_FRESH != 0
        && (!(-180.0..=180.0).contains(&sensors[6])
            || !(-90.0..=90.0).contains(&sensors[7]))
    {
        return Err("valid GPS coordinates are outside longitude/latitude range".to_string());
    }
    Ok(())
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes(bytes[offset..offset + 2].try_into().expect("checked frame length"))
}

fn read_i16(bytes: &[u8], offset: usize) -> i16 {
    i16::from_be_bytes(bytes[offset..offset + 2].try_into().expect("checked frame length"))
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(bytes[offset..offset + 4].try_into().expect("checked frame length"))
}

fn read_i32(bytes: &[u8], offset: usize) -> i32 {
    i32::from_be_bytes(bytes[offset..offset + 4].try_into().expect("checked frame length"))
}

fn read_f32(bytes: &[u8], offset: usize) -> f32 {
    f32::from_be_bytes(bytes[offset..offset + 4].try_into().expect("checked frame length"))
}

#[cfg(test)]
mod tests {
    use super::{PacketParser, ParseResult};
    use crate::models::response::ParsedFrame;
    use crate::services::serial::Parser;
    use serde_json::Value;

    const V1_VECTORS: &str = include_str!("../../../../../protocol/test_vectors_v1.json");
    const V2_VECTORS: &str = include_str!("../../../../../protocol/test_vectors_v2.json");

    fn hex_bytes(text: &str) -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&text[index..index + 2], 16).unwrap())
            .collect()
    }

    fn feed(parser: &mut PacketParser, bytes: &[u8]) -> ParseResult {
        let mut result = ParseResult::Incomplete;
        for byte in bytes {
            let next = parser.sink(*byte);
            if !matches!(next, ParseResult::Incomplete) {
                result = next;
            }
        }
        result
    }

    fn vectors(document: &str) -> Vec<Value> {
        serde_json::from_str::<Value>(document).unwrap()["vectors"]
            .as_array()
            .unwrap()
            .clone()
    }

    fn verify_vectors(document: &str) {
        for vector in vectors(document) {
            let id = vector["id"].as_str().unwrap();
            let expected = vector["expected"]["parse"].as_str().unwrap();
            let bytes = hex_bytes(vector["frame_hex"].as_str().unwrap());
            let result = feed(&mut PacketParser::default(), &bytes);
            match expected {
                "OK" if id.contains("telemetry_nominal") => assert!(matches!(result, ParseResult::Complete(ParsedFrame::Telemetry(_))), "{id}"),
                "OK" if id.contains("ack_") => assert!(matches!(result, ParseResult::Complete(ParsedFrame::Ack(_))), "{id}"),
                "OK" => assert!(matches!(result, ParseResult::IgnoredFrame(_)), "{id}"),
                "CRC_ERROR" => match result {
                    ParseResult::ParseError(message) => assert!(message.contains("CRC"), "{id}"),
                    other => panic!("{id}: expected CRC error, got {other:?}"),
                },
                "TRUNCATED" => assert!(matches!(result, ParseResult::Incomplete), "{id}"),
                other => panic!("unsupported expected result {other}"),
            }
        }
    }

    #[test]
    fn parses_all_v1_and_v2_golden_vectors() {
        verify_vectors(V1_VECTORS);
        verify_vectors(V2_VECTORS);
    }

    #[test]
    fn v2_telemetry_restores_physical_units() {
        let vector = vectors(V2_VECTORS).into_iter().find(|v| v["id"] == "telemetry_nominal_v2").unwrap();
        let ParseResult::Complete(ParsedFrame::Telemetry(payload)) = feed(&mut PacketParser::default(), &hex_bytes(vector["frame_hex"].as_str().unwrap())) else { panic!("v2 telemetry did not decode") };
        assert_eq!(payload.protocol_version, 2);
        assert_eq!(payload.session_id, 0x1234_5678);
        assert_eq!(payload.frame_seq, 42);
        assert_eq!(payload.sensor_flags, 0x0F);
        assert_eq!(payload.z_acceleration, 9.81);
        assert_eq!(payload.z_angular_velocity, 180.0);
        assert!((payload.longitude - 121.5654).abs() < 0.00001);
        assert_eq!(payload.altitude, 123.45);
        assert_eq!(payload.vertical_velocity, -3.3);
        assert_eq!(payload.air_pressure, 1001.2);
    }

    #[test]
    fn one_parser_accepts_v1_then_v2_without_reset() {
        let v1 = vectors(V1_VECTORS).into_iter().find(|v| v["id"] == "telemetry_nominal").unwrap();
        let v2 = vectors(V2_VECTORS).into_iter().find(|v| v["id"] == "telemetry_nominal_v2").unwrap();
        let mut parser = PacketParser::default();
        assert!(matches!(feed(&mut parser, &hex_bytes(v1["frame_hex"].as_str().unwrap())), ParseResult::Complete(ParsedFrame::Telemetry(_))));
        assert!(matches!(feed(&mut parser, &hex_bytes(v2["frame_hex"].as_str().unwrap())), ParseResult::Complete(ParsedFrame::Telemetry(_))));
    }

    #[test]
    fn v2_ack_has_no_independent_frame_sequence() {
        let vector = vectors(V2_VECTORS).into_iter().find(|v| v["id"] == "ack_executed_v2").unwrap();
        let ParseResult::Complete(ParsedFrame::Ack(ack)) = feed(&mut PacketParser::default(), &hex_bytes(vector["frame_hex"].as_str().unwrap())) else { panic!("v2 ACK did not decode") };
        assert_eq!(ack.frame_seq, None);
        assert_eq!(ack.command_id, 0x0102_0305);
        assert_eq!(ack.remaining_s, 30);
    }
}
