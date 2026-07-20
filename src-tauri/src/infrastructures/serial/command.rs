use crate::infrastructures::serial::crc::crc16_ccitt;
use crate::models::response::{AckPayload, CommandStatusEvent, TelemetryPayload};

pub const MAGIC: [u8; 2] = [0xA5, 0x5A];
pub const PROTOCOL_V1: u8 = 0x01;
pub const PROTOCOL_V2: u8 = 0x02;
pub const V1_HEADER_LENGTH: usize = 20;
pub const V2_HEADER_LENGTH: usize = 14;
pub const FRAME_TYPE_SET_TIMER: u8 = 0x10;
pub const FRAME_TYPE_FORCE_RELEASE: u8 = 0x11;
pub const FLAG_RETRANSMISSION: u8 = 0x01;
pub const COMMAND_RETRY_INTERVAL_MS: u64 = 100;

pub const ACK_EXECUTED: u8 = 0x00;
pub const ACK_DUPLICATE: u8 = 0x01;
pub const ACK_ALREADY_DEPLOYED: u8 = 0x02;
const DEPLOY_STATE_DEPLOYED: u8 = 0x01;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandRequest {
    SetTimer(u32),
    ForceRelease,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommandKind {
    SetTimer,
    ForceRelease,
}

impl CommandKind {
    fn frame_type(self) -> u8 {
        match self {
            Self::SetTimer => FRAME_TYPE_SET_TIMER,
            Self::ForceRelease => FRAME_TYPE_FORCE_RELEASE,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::SetTimer => "SET_TIMER",
            Self::ForceRelease => "FORCE_RELEASE",
        }
    }
}

#[derive(Clone, Debug)]
struct PendingCommand {
    kind: CommandKind,
    command_id: u32,
    payload: Vec<u8>,
    attempts: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transmission {
    pub bytes: Vec<u8>,
    pub protocol_version: u8,
    pub command_id: u32,
    pub command_type: u8,
    pub attempts: u32,
}

#[derive(Default)]
pub struct CommandManager {
    current_session_id: Option<u32>,
    current_protocol_version: Option<u8>,
    latest_timer_s: Option<u32>,
    pending_timer: Option<PendingCommand>,
    pending_force: Option<PendingCommand>,
    next_command_id: u32,
    next_frame_seq: u32,
}

impl CommandManager {
    pub fn request(&mut self, request: CommandRequest) -> Result<CommandStatusEvent, String> {
        match request {
            CommandRequest::SetTimer(duration_s) => {
                if duration_s == 0 {
                    return Err("timer duration must be greater than zero".to_string());
                }
                self.latest_timer_s = Some(duration_s);
                self.pending_timer = self
                    .current_session_id
                    .map(|_| self.new_pending(CommandKind::SetTimer, duration_s.to_be_bytes().to_vec()))
                    .transpose()?;
                Ok(status_for(self.pending_timer.as_ref(), "queued", None, "latest timer retained"))
            }
            CommandRequest::ForceRelease => {
                if self.current_session_id.is_none() {
                    return Err(
                        "FORCE_RELEASE requires a live airborne telemetry session".to_string(),
                    );
                }
                self.pending_force = Some(self.new_pending(CommandKind::ForceRelease, Vec::new())?);
                Ok(status_for(self.pending_force.as_ref(), "queued", None, "highest priority"))
            }
        }
    }

    pub fn observe_telemetry(&mut self, telemetry: &TelemetryPayload) -> Vec<CommandStatusEvent> {
        let mut events = Vec::new();
        let changed = self.current_session_id != Some(telemetry.session_id)
            || self.current_protocol_version != Some(telemetry.protocol_version);
        if changed {
            let previous = self.current_session_id.replace(telemetry.session_id);
            self.current_protocol_version = Some(telemetry.protocol_version);
            self.next_command_id = telemetry.last_ack_command_id;

            if self.pending_force.take().is_some() && previous.is_some() {
                events.push(CommandStatusEvent {
                    command_id: None,
                    command_type: "FORCE_RELEASE".to_string(),
                    status: "cancelled".to_string(),
                    attempts: 0,
                    result: None,
                    detail: "airborne session changed; force release was not replayed".to_string(),
                });
            }

            self.pending_timer = None;
            if let Some(duration_s) = self.latest_timer_s {
                match self.new_pending(CommandKind::SetTimer, duration_s.to_be_bytes().to_vec()) {
                    Ok(command) => {
                        self.pending_timer = Some(command);
                        events.push(status_for(
                            self.pending_timer.as_ref(),
                            "queued",
                            None,
                            "airborne session changed; latest timer requeued",
                        ));
                    }
                    Err(error) => events.push(error_status("SET_TIMER", error)),
                }
            }
        } else {
            if let Some(status) = self.reconcile_telemetry_ack(telemetry) {
                events.push(status);
            }
            if telemetry.last_ack_command_id > self.next_command_id {
                self.next_command_id = telemetry.last_ack_command_id;
            }
        }
        self.finish_commands_invalidated_by_deployment(telemetry, &mut events);
        events
    }

    pub fn cancel_pending_force(&mut self, detail: &str) -> Option<CommandStatusEvent> {
        let pending = self.pending_force.take()?;
        Some(CommandStatusEvent {
            command_id: Some(pending.command_id),
            command_type: pending.kind.label().to_string(),
            status: "cancelled".to_string(),
            attempts: pending.attempts,
            result: None,
            detail: detail.to_string(),
        })
    }

    fn finish_commands_invalidated_by_deployment(
        &mut self,
        telemetry: &TelemetryPayload,
        events: &mut Vec<CommandStatusEvent>,
    ) {
        if telemetry.deploy_state != DEPLOY_STATE_DEPLOYED {
            return;
        }
        if let Some(pending) = self.pending_force.take() {
            events.push(CommandStatusEvent {
                command_id: Some(pending.command_id),
                command_type: pending.kind.label().to_string(),
                status: "acked".to_string(),
                attempts: pending.attempts,
                result: Some(ACK_ALREADY_DEPLOYED),
                detail: "airborne telemetry confirms DEPLOYED; force retries stopped".to_string(),
            });
        }
        if let Some(pending) = self.pending_timer.take() {
            events.push(CommandStatusEvent {
                command_id: Some(pending.command_id),
                command_type: pending.kind.label().to_string(),
                status: "cancelled".to_string(),
                attempts: pending.attempts,
                result: None,
                detail: "airborne is DEPLOYED; pending timer cancelled as invalid".to_string(),
            });
        }
    }

    fn reconcile_telemetry_ack(&mut self, telemetry: &TelemetryPayload) -> Option<CommandStatusEvent> {
        if telemetry.last_ack_command_id == 0 {
            return None;
        }
        let matching_force = self.pending_force.as_ref().is_some_and(|pending| {
            pending.command_id == telemetry.last_ack_command_id
        });
        let matching_timer = self.pending_timer.as_ref().is_some_and(|pending| {
            pending.command_id == telemetry.last_ack_command_id
        });
        let pending = if matching_force {
            self.pending_force.as_ref()
        } else if matching_timer {
            self.pending_timer.as_ref()
        } else {
            return None;
        };
        let attempts = pending.map_or(0, |command| command.attempts);
        let command_type = pending.map_or("UNKNOWN", |command| command.kind.label()).to_string();
        let success = matches!(telemetry.last_ack_result, ACK_EXECUTED | ACK_DUPLICATE)
            || (matching_force && telemetry.last_ack_result == ACK_ALREADY_DEPLOYED);
        if matching_force {
            self.pending_force = None;
        } else {
            self.pending_timer = None;
        }
        Some(CommandStatusEvent {
            command_id: Some(telemetry.last_ack_command_id),
            command_type,
            status: if success { "acked" } else { "failed" }.to_string(),
            attempts,
            result: Some(telemetry.last_ack_result),
            detail: if success {
                "matching command completion recovered from telemetry last-ACK".to_string()
            } else {
                format!(
                    "airborne telemetry reports command rejection 0x{:02X}",
                    telemetry.last_ack_result
                )
            },
        })
    }

    pub fn next_transmission(&mut self) -> Result<Option<Transmission>, String> {
        let Some(session_id) = self.current_session_id else {
            return Ok(None);
        };
        let protocol_version = self
            .current_protocol_version
            .ok_or_else(|| "airborne protocol version is unknown".to_string())?;
        let pending = self
            .pending_force
            .as_mut()
            .or(self.pending_timer.as_mut());
        let Some(pending) = pending else {
            return Ok(None);
        };
        self.next_frame_seq = self
            .next_frame_seq
            .checked_add(1)
            .ok_or_else(|| "command frame sequence exhausted".to_string())?;
        let retransmission = pending.attempts > 0;
        pending.attempts = pending
            .attempts
            .checked_add(1)
            .ok_or_else(|| "command retry count exhausted".to_string())?;
        let bytes = encode_command(
            protocol_version,
            pending.kind.frame_type(),
            retransmission,
            session_id,
            self.next_frame_seq,
            pending.command_id,
            &pending.payload,
        )?;
        Ok(Some(Transmission {
            bytes,
            protocol_version,
            command_id: pending.command_id,
            command_type: pending.kind.frame_type(),
            attempts: pending.attempts,
        }))
    }

    pub fn handle_ack(&mut self, ack: &AckPayload) -> CommandStatusEvent {
        if self.current_session_id != Some(ack.session_id) {
            return ignored_ack(ack, "ACK belongs to an old or unknown session");
        }
        let matching_force = self.pending_force.as_ref().is_some_and(|pending| {
            pending.command_id == ack.command_id && pending.kind.frame_type() == ack.acked_type
        });
        let matching_timer = self.pending_timer.as_ref().is_some_and(|pending| {
            pending.command_id == ack.command_id && pending.kind.frame_type() == ack.acked_type
        });
        let pending = if matching_force {
            self.pending_force.as_ref()
        } else if matching_timer {
            self.pending_timer.as_ref()
        } else {
            return ignored_ack(ack, "ACK does not match the current pending command");
        };
        let attempts = pending.map_or(0, |command| command.attempts);
        let command_type = pending.map_or("UNKNOWN", |command| command.kind.label()).to_string();
        let success = matches!(ack.result, ACK_EXECUTED | ACK_DUPLICATE)
            || (ack.acked_type == FRAME_TYPE_FORCE_RELEASE && ack.result == ACK_ALREADY_DEPLOYED);
        if matching_force {
            self.pending_force = None;
        } else {
            self.pending_timer = None;
        }
        CommandStatusEvent {
            command_id: Some(ack.command_id),
            command_type,
            status: if success { "acked" } else { "failed" }.to_string(),
            attempts,
            result: Some(ack.result),
            detail: if success {
                "matching ACK accepted".to_string()
            } else {
                format!("airborne rejected command with result 0x{:02X}", ack.result)
            },
        }
    }

    fn new_pending(&mut self, kind: CommandKind, payload: Vec<u8>) -> Result<PendingCommand, String> {
        self.next_command_id = self
            .next_command_id
            .checked_add(1)
            .ok_or_else(|| "command ID exhausted for the current session".to_string())?;
        if self.next_command_id == 0 {
            return Err("command ID must be non-zero".to_string());
        }
        Ok(PendingCommand {
            kind,
            command_id: self.next_command_id,
            payload,
            attempts: 0,
        })
    }
}

pub fn encode_command(
    protocol_version: u8,
    frame_type: u8,
    retransmission: bool,
    session_id: u32,
    frame_seq: u32,
    command_id: u32,
    payload: &[u8],
) -> Result<Vec<u8>, String> {
    if session_id == 0 || command_id == 0 {
        return Err("session_id and command_id must be non-zero".to_string());
    }
    let expected_length = match frame_type {
        FRAME_TYPE_SET_TIMER => 4,
        FRAME_TYPE_FORCE_RELEASE => 0,
        _ => return Err(format!("unsupported command type 0x{frame_type:02X}")),
    };
    if payload.len() != expected_length {
        return Err(format!("command payload must be {expected_length} bytes"));
    }
    let (header_length, payload_offset) = match protocol_version {
        PROTOCOL_V1 => (V1_HEADER_LENGTH, V1_HEADER_LENGTH),
        PROTOCOL_V2 => (V2_HEADER_LENGTH, V2_HEADER_LENGTH),
        _ => return Err(format!("unsupported airborne protocol version {protocol_version}")),
    };
    let mut frame = vec![0_u8; header_length + payload.len() + 2];
    frame[0..2].copy_from_slice(&MAGIC);
    frame[2] = protocol_version;
    frame[3] = frame_type;
    frame[4] = if retransmission { FLAG_RETRANSMISSION } else { 0 };
    if protocol_version == PROTOCOL_V1 {
        frame[5] = V1_HEADER_LENGTH as u8;
        frame[6..8].copy_from_slice(&(payload.len() as u16).to_be_bytes());
        frame[8..12].copy_from_slice(&session_id.to_be_bytes());
        frame[12..16].copy_from_slice(&frame_seq.to_be_bytes());
        frame[16..20].copy_from_slice(&command_id.to_be_bytes());
    } else {
        frame[5] = payload.len() as u8;
        frame[6..10].copy_from_slice(&session_id.to_be_bytes());
        frame[10..14].copy_from_slice(&command_id.to_be_bytes());
    }
    frame[payload_offset..payload_offset + payload.len()].copy_from_slice(payload);
    let crc = crc16_ccitt(&frame[2..payload_offset + payload.len()]);
    let crc_offset = payload_offset + payload.len();
    frame[crc_offset..crc_offset + 2].copy_from_slice(&crc.to_be_bytes());
    Ok(frame)
}

fn status_for(
    pending: Option<&PendingCommand>,
    status: &str,
    result: Option<u8>,
    detail: &str,
) -> CommandStatusEvent {
    CommandStatusEvent {
        command_id: pending.map(|command| command.command_id),
        command_type: pending.map_or("SET_TIMER", |command| command.kind.label()).to_string(),
        status: status.to_string(),
        attempts: pending.map_or(0, |command| command.attempts),
        result,
        detail: detail.to_string(),
    }
}

fn ignored_ack(ack: &AckPayload, detail: &str) -> CommandStatusEvent {
    CommandStatusEvent {
        command_id: Some(ack.command_id),
        command_type: format!("0x{:02X}", ack.acked_type),
        status: "ignored_ack".to_string(),
        attempts: 0,
        result: Some(ack.result),
        detail: detail.to_string(),
    }
}

fn error_status(command_type: &str, detail: String) -> CommandStatusEvent {
    CommandStatusEvent {
        command_id: None,
        command_type: command_type.to_string(),
        status: "failed".to_string(),
        attempts: 0,
        result: None,
        detail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const V1_VECTORS: &str = include_str!("../../../../../protocol/test_vectors_v1.json");
    const V2_VECTORS: &str = include_str!("../../../../../protocol/test_vectors_v2.json");

    fn vector_hex(document: &str, vector_id: &str) -> String {
        let document: Value = serde_json::from_str(document).unwrap();
        document["vectors"]
            .as_array()
            .unwrap()
            .iter()
            .find(|vector| vector["id"] == vector_id)
            .unwrap()["frame_hex"]
            .as_str()
            .unwrap()
            .to_string()
    }

    fn telemetry(session_id: u32, last_ack_command_id: u32) -> TelemetryPayload {
        telemetry_version(PROTOCOL_V1, session_id, last_ack_command_id)
    }

    fn telemetry_version(protocol_version: u8, session_id: u32, last_ack_command_id: u32) -> TelemetryPayload {
        TelemetryPayload {
            protocol_version,
            session_id,
            frame_seq: 1,
            uptime_ms: 1,
            restart_reason: 1,
            timer_state: 0,
            deploy_state: 0,
            sensor_flags: 0,
            remaining_s: 0,
            last_ack_command_id,
            last_ack_result: if last_ack_command_id == 0 { 0xFF } else { 0 },
            x_acceleration: 0.0,
            y_acceleration: 0.0,
            z_acceleration: 0.0,
            x_angular_velocity: 0.0,
            y_angular_velocity: 0.0,
            z_angular_velocity: 0.0,
            longitude: 0.0,
            latitude: 0.0,
            altitude: 0.0,
            ground_speed: 0.0,
            vertical_velocity: 0.0,
            air_pressure: 0.0,
            temperature: 0.0,
        }
    }

    #[test]
    fn encoder_matches_shared_command_golden_vectors() {
        assert_eq!(COMMAND_RETRY_INTERVAL_MS, 100);
        let set_timer = encode_command(
            PROTOCOL_V1,
            FRAME_TYPE_SET_TIMER,
            false,
            0x1234_5678,
            7,
            0x0102_0305,
            &30_u32.to_be_bytes(),
        )
        .unwrap();
        assert_eq!(
            hex(&set_timer),
            vector_hex(V1_VECTORS, "set_timer_nominal")
        );
        let retry = encode_command(
            PROTOCOL_V1,
            FRAME_TYPE_SET_TIMER,
            true,
            0x1234_5678,
            8,
            0x0102_0305,
            &30_u32.to_be_bytes(),
        )
        .unwrap();
        assert_eq!(
            hex(&retry),
            vector_hex(V1_VECTORS, "set_timer_retry_duplicate")
        );
        let force = encode_command(
            PROTOCOL_V1,
            FRAME_TYPE_FORCE_RELEASE,
            false,
            0x1234_5678,
            9,
            0x0102_0306,
            &[],
        )
        .unwrap();
        assert_eq!(
            hex(&force),
            vector_hex(V1_VECTORS, "force_release_nominal")
        );

        let v2_timer = encode_command(
            PROTOCOL_V2,
            FRAME_TYPE_SET_TIMER,
            false,
            0x1234_5678,
            99,
            0x0102_0305,
            &30_u32.to_be_bytes(),
        )
        .unwrap();
        assert_eq!(hex(&v2_timer), vector_hex(V2_VECTORS, "set_timer_nominal_v2"));
        let v2_retry = encode_command(
            PROTOCOL_V2,
            FRAME_TYPE_SET_TIMER,
            true,
            0x1234_5678,
            100,
            0x0102_0305,
            &30_u32.to_be_bytes(),
        )
        .unwrap();
        assert_eq!(hex(&v2_retry), vector_hex(V2_VECTORS, "set_timer_retry_v2"));
    }

    #[test]
    fn force_has_priority_and_only_latest_timer_is_retained() {
        let mut manager = CommandManager::default();
        manager.observe_telemetry(&telemetry(7, 0));
        manager.request(CommandRequest::SetTimer(30)).unwrap();
        let old_timer_id = manager.pending_timer.as_ref().unwrap().command_id;
        manager.request(CommandRequest::SetTimer(12)).unwrap();
        assert_ne!(manager.pending_timer.as_ref().unwrap().command_id, old_timer_id);
        assert_eq!(manager.pending_timer.as_ref().unwrap().payload, 12_u32.to_be_bytes());
        manager.request(CommandRequest::ForceRelease).unwrap();
        assert_eq!(
            manager.next_transmission().unwrap().unwrap().command_type,
            FRAME_TYPE_FORCE_RELEASE
        );
    }

    #[test]
    fn manager_uses_the_protocol_version_observed_in_telemetry() {
        let mut manager = CommandManager::default();
        manager.observe_telemetry(&telemetry_version(PROTOCOL_V2, 0x1234_5678, 0x0102_0304));
        manager.request(CommandRequest::SetTimer(30)).unwrap();
        let transmission = manager.next_transmission().unwrap().unwrap();
        assert_eq!(transmission.protocol_version, PROTOCOL_V2);
        assert_eq!(hex(&transmission.bytes), vector_hex(V2_VECTORS, "set_timer_nominal_v2"));
    }

    #[test]
    fn retries_keep_command_id_and_old_ack_is_ignored() {
        let mut manager = CommandManager::default();
        manager.observe_telemetry(&telemetry(9, 40));
        manager.request(CommandRequest::SetTimer(20)).unwrap();
        let first = manager.next_transmission().unwrap().unwrap();
        let retry = manager.next_transmission().unwrap().unwrap();
        assert_eq!(first.command_id, retry.command_id);
        assert_eq!(retry.bytes[4], FLAG_RETRANSMISSION);
        let ignored = manager.handle_ack(&AckPayload {
            session_id: 9,
            frame_seq: Some(1),
            command_id: first.command_id - 1,
            acked_type: FRAME_TYPE_SET_TIMER,
            result: ACK_EXECUTED,
            timer_state: 1,
            deploy_state: 0,
            remaining_s: 20,
        });
        assert_eq!(ignored.status, "ignored_ack");
        assert!(manager.pending_timer.is_some());
    }

    #[test]
    fn session_change_requeues_latest_timer_but_not_force() {
        let mut manager = CommandManager::default();
        manager.observe_telemetry(&telemetry(1, 0));
        manager.request(CommandRequest::SetTimer(33)).unwrap();
        manager.request(CommandRequest::ForceRelease).unwrap();
        let events = manager.observe_telemetry(&telemetry(2, 7));
        assert!(manager.pending_force.is_none());
        assert_eq!(manager.pending_timer.as_ref().unwrap().payload, 33_u32.to_be_bytes());
        assert!(events.iter().any(|event| event.detail.contains("latest timer requeued")));
    }

    #[test]
    fn force_without_session_is_rejected_and_never_deferred() {
        let mut manager = CommandManager::default();

        assert!(manager.request(CommandRequest::ForceRelease).is_err());
        manager.observe_telemetry(&telemetry(1, 0));

        assert!(manager.next_transmission().unwrap().is_none());
    }

    #[test]
    fn link_loss_cancels_force_without_replaying_it_on_recovery() {
        let mut manager = CommandManager::default();
        manager.observe_telemetry(&telemetry(1, 0));
        manager.request(CommandRequest::ForceRelease).unwrap();

        let cancelled = manager
            .cancel_pending_force("telemetry link lost")
            .expect("pending FORCE must be cancelled");
        assert_eq!(cancelled.status, "cancelled");
        assert!(manager.next_transmission().unwrap().is_none());

        manager.observe_telemetry(&telemetry(1, 0));
        assert!(manager.next_transmission().unwrap().is_none());
    }

    #[test]
    fn matching_ack_stops_retries_while_duplicate_ack_is_safe() {
        let mut manager = CommandManager::default();
        manager.observe_telemetry(&telemetry(3, 0));
        manager.request(CommandRequest::SetTimer(5)).unwrap();
        let sent = manager.next_transmission().unwrap().unwrap();
        let accepted = manager.handle_ack(&AckPayload {
            session_id: 3,
            frame_seq: Some(2),
            command_id: sent.command_id,
            acked_type: FRAME_TYPE_SET_TIMER,
            result: ACK_DUPLICATE,
            timer_state: 1,
            deploy_state: 0,
            remaining_s: 5,
        });
        assert_eq!(accepted.status, "acked");
        assert!(manager.next_transmission().unwrap().is_none());
    }

    #[test]
    fn telemetry_last_ack_stops_retry_when_the_ack_frame_was_lost() {
        let mut manager = CommandManager::default();
        manager.observe_telemetry(&telemetry(4, 0));
        manager.request(CommandRequest::SetTimer(15)).unwrap();
        let sent = manager.next_transmission().unwrap().unwrap();
        let mut completion = telemetry(4, sent.command_id);
        completion.last_ack_result = ACK_DUPLICATE;
        let events = manager.observe_telemetry(&completion);
        assert!(events.iter().any(|event| {
            event.status == "acked" && event.command_id == Some(sent.command_id)
        }));
        assert!(manager.next_transmission().unwrap().is_none());
    }

    #[test]
    fn telemetry_last_ack_releases_force_priority_for_the_retained_timer() {
        let mut manager = CommandManager::default();
        manager.observe_telemetry(&telemetry(5, 0));
        manager.request(CommandRequest::SetTimer(30)).unwrap();
        manager.request(CommandRequest::ForceRelease).unwrap();
        let force = manager.next_transmission().unwrap().unwrap();
        assert_eq!(force.command_type, FRAME_TYPE_FORCE_RELEASE);

        let mut completion = telemetry(5, force.command_id);
        completion.last_ack_result = ACK_ALREADY_DEPLOYED;
        completion.deploy_state = DEPLOY_STATE_DEPLOYED;
        let events = manager.observe_telemetry(&completion);
        assert!(events.iter().any(|event| event.status == "acked"));
        assert_eq!(
            manager.next_transmission().unwrap(),
            None
        );
    }

    #[test]
    fn deployed_telemetry_stops_force_and_cancels_timer_retries() {
        let mut manager = CommandManager::default();
        manager.observe_telemetry(&telemetry(6, 0));
        manager.request(CommandRequest::SetTimer(45)).unwrap();
        manager.request(CommandRequest::ForceRelease).unwrap();
        manager.next_transmission().unwrap().unwrap();

        let mut deployed = telemetry(6, 0);
        deployed.deploy_state = DEPLOY_STATE_DEPLOYED;
        let events = manager.observe_telemetry(&deployed);
        assert!(events.iter().any(|event| {
            event.command_type == "FORCE_RELEASE" && event.status == "acked"
        }));
        assert!(events.iter().any(|event| {
            event.command_type == "SET_TIMER" && event.status == "cancelled"
        }));
        assert!(manager.next_transmission().unwrap().is_none());

        manager.observe_telemetry(&telemetry(7, 0));
        assert_eq!(
            manager.next_transmission().unwrap().unwrap().command_type,
            FRAME_TYPE_SET_TIMER
        );
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02X}")).collect()
    }
}
