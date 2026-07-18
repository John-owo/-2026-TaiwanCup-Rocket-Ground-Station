use crate::infrastructures::serial::command::{
    CommandManager, CommandRequest, COMMAND_RETRY_INTERVAL_MS, FRAME_TYPE_FORCE_RELEASE,
    FRAME_TYPE_SET_TIMER,
};
use crate::infrastructures::serial::parser::{PacketParser, ParseResult};
use crate::models::response::{
    AirborneSessionChanged, CommandStatusEvent, ParsedFrame, TelemetryPayload,
};
use crate::services::serial::{Parser, Receiver};

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio::time::MissedTickBehavior;
use tokio_serial::{SerialPortBuilderExt, SerialStream};
use tokio_util::sync::CancellationToken;

const COMMAND_WINDOW_START_MS: u64 = 150;
const COMMAND_WINDOW_END_MS: u64 = 300;
const SERIAL_RX_IDLE_GUARD_MS: u64 = 30;
const TELEMETRY_RECOVERY_GAP_MS: u64 = 4_500;

#[derive(Default)]
struct HalfDuplexSchedule {
    last_telemetry_ms: Option<u64>,
    last_rx_byte_ms: Option<u64>,
    command_sent_for_current_telemetry: bool,
}

impl HalfDuplexSchedule {
    fn observe_rx_byte(&mut self, now_ms: u64) {
        self.last_rx_byte_ms = Some(now_ms);
    }

    fn observe_telemetry(&mut self, now_ms: u64) {
        self.last_telemetry_ms = Some(now_ms);
        self.command_sent_for_current_telemetry = false;
    }

    fn command_window_open(&self, now_ms: u64) -> bool {
        if self.command_sent_for_current_telemetry {
            return false;
        }
        let Some(last_telemetry_ms) = self.last_telemetry_ms else {
            return false;
        };
        if self
            .last_rx_byte_ms
            .is_some_and(|last_rx_ms| now_ms.saturating_sub(last_rx_ms) < SERIAL_RX_IDLE_GUARD_MS)
        {
            return false;
        }
        let elapsed_ms = now_ms.saturating_sub(last_telemetry_ms);
        (COMMAND_WINDOW_START_MS..COMMAND_WINDOW_END_MS).contains(&elapsed_ms)
    }

    fn mark_command_transmitted(&mut self) {
        self.command_sent_for_current_telemetry = true;
    }
}

pub struct SerialReceiver {
    pub parser: Option<PacketParser>,
    pub path: Option<String>,
    pub baud_rate: Option<u32>,
    pub serial_stream: Option<SerialStream>,
    pub cancellation_token: CancellationToken,
    pub verification_failed_count: Arc<Mutex<u32>>,
    pub total_packet_count: Arc<Mutex<u64>>,
    pub app_handle: AppHandle,
    pub command_rx: Option<mpsc::UnboundedReceiver<CommandRequest>>,
}

#[derive(Default)]
struct SessionTracker {
    current_session_id: Option<u32>,
}

impl SessionTracker {
    fn observe(&mut self, session_id: u32, restart_reason: u8) -> Option<AirborneSessionChanged> {
        if self.current_session_id == Some(session_id) {
            return None;
        }
        let change = AirborneSessionChanged {
            previous_session_id: self.current_session_id,
            session_id,
            restart_reason,
        };
        self.current_session_id = Some(session_id);
        Some(change)
    }
}

impl Receiver for SerialReceiver {
    async fn get_connection(&mut self, path: String, baud_rate: u32) -> Result<(), String> {
        match tokio_serial::new(&path, baud_rate).open_native_async() {
            Ok(serial_stream) => {
                self.serial_stream = Some(serial_stream);
                self.path = Some(path);
                self.baud_rate = Some(baud_rate);
                log::info!(
                    "serial port connected: {} @ {}",
                    self.path.as_ref().expect("path set"),
                    baud_rate
                );
                Ok(())
            }
            Err(error) => Err(format!("failed to open serial port: {error}")),
        }
    }

    async fn start_receive(&mut self) -> Result<String, String> {
        if self.serial_stream.is_none() {
            return Err("not connected to serial port".to_string());
        }
        self.parser = Some(PacketParser::default());
        self.receive_task().await
    }

    async fn receive_task(&mut self) -> Result<String, String> {
        let serial_stream = self
            .serial_stream
            .as_mut()
            .ok_or("serial stream not available")?;
        let parser = self.parser.as_mut().ok_or("parser not initialized")?;
        let mut command_rx = self
            .command_rx
            .take()
            .ok_or("command receiver not initialized")?;

        let failed_count = self.verification_failed_count.clone();
        let total_count = self.total_packet_count.clone();
        let app_handle = self.app_handle.clone();
        let cancellation_token = self.cancellation_token.clone();
        let mut session_tracker = SessionTracker::default();
        let mut command_manager = CommandManager::default();
        let mut resend_tick = tokio::time::interval(Duration::from_millis(COMMAND_RETRY_INTERVAL_MS));
        resend_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let (mut serial_reader, mut serial_writer) = tokio::io::split(serial_stream);
        let receive_started = Instant::now();
        let mut last_telemetry_ms: Option<u64> = None;
        let mut last_deploy_state = 0_u8;
        let mut half_duplex_schedule = HalfDuplexSchedule::default();

        loop {
            tokio::select! {
                biased;

                _ = cancellation_token.cancelled() => {
                    log::info!("receive loop cancelled gracefully");
                    return Ok("receive loop stopped gracefully".to_string());
                }

                Some(request) = command_rx.recv() => {
                    match command_manager.request(request) {
                        Ok(status) => emit_command_status(&app_handle, &status),
                        Err(error) => {
                            let status = CommandStatusEvent {
                                command_id: None,
                                command_type: "UNKNOWN".to_string(),
                                status: "failed".to_string(),
                                attempts: 0,
                                result: None,
                                detail: error,
                            };
                            emit_command_status(&app_handle, &status);
                        }
                    }
                }

                _ = resend_tick.tick() => {
                    let now_ms = receive_started.elapsed().as_millis() as u64;
                    update_link_stats(&app_handle, now_ms);
                    if !half_duplex_schedule.command_window_open(now_ms) {
                        continue;
                    }
                    match command_manager.next_transmission() {
                        Ok(Some(transmission)) => {
                            if let Err(error) = serial_writer.write_all(&transmission.bytes).await {
                                let detail = format!("serial command write error: {error}");
                                log_flight_event(&app_handle, "ERROR", &detail);
                                let _ = app_handle.emit("serial-error", serde_json::json!({
                                    "errorType": "SERIAL_ERROR",
                                    "detail": detail,
                                }));
                                return Err(detail);
                            }
                            if let Err(error) = serial_writer.flush().await {
                                let detail = format!("serial command flush error: {error}");
                                log_flight_event(&app_handle, "ERROR", &detail);
                                return Err(detail);
                            }
                            half_duplex_schedule.mark_command_transmitted();
                            let status = CommandStatusEvent {
                                command_id: Some(transmission.command_id),
                                command_type: command_label(transmission.command_type).to_string(),
                                status: "sending".to_string(),
                                attempts: transmission.attempts,
                                result: None,
                                detail: format!("Protocol v{} command transmitted in half-duplex uplink window; waiting for matching ACK", transmission.protocol_version),
                            };
                            emit_command_status(&app_handle, &status);
                        }
                        Ok(None) => {}
                        Err(error) => {
                            let status = CommandStatusEvent {
                                command_id: None,
                                command_type: "UNKNOWN".to_string(),
                                status: "failed".to_string(),
                                attempts: 0,
                                result: None,
                                detail: error,
                            };
                            emit_command_status(&app_handle, &status);
                        }
                    }
                }

                result = serial_reader.read_u8() => {
                    let rx_now_ms = receive_started.elapsed().as_millis() as u64;
                    half_duplex_schedule.observe_rx_byte(rx_now_ms);
                    let byte = match result {
                        Ok(byte) => byte,
                        Err(error) => {
                            let detail = format!("serial read error: {error}");
                            log_flight_event(&app_handle, "ERROR", &detail);
                            let _ = app_handle.emit("serial-error", serde_json::json!({
                                "errorType": "SERIAL_ERROR",
                                "detail": detail,
                            }));
                            return Err(detail);
                        }
                    };

                    match parser.sink(byte) {
                        ParseResult::Incomplete => {}
                        ParseResult::Complete(ParsedFrame::Telemetry(payload)) => {
                            let telemetry_now_ms = receive_started.elapsed().as_millis() as u64;
                            half_duplex_schedule.observe_telemetry(telemetry_now_ms);
                            if let Some(previous_ms) = last_telemetry_ms {
                                let gap_ms = telemetry_now_ms.saturating_sub(previous_ms);
                                if gap_ms > TELEMETRY_RECOVERY_GAP_MS {
                                    log_flight_event(
                                        &app_handle,
                                        "WARN",
                                        &format!("telemetry link recovered after {gap_ms} ms"),
                                    );
                                }
                            }
                            last_telemetry_ms = Some(telemetry_now_ms);
                            if last_deploy_state == 0 && payload.deploy_state == 1 {
                                log_flight_event(
                                    &app_handle,
                                    "WARN",
                                    "airborne telemetry changed to DEPLOYED",
                                );
                            }
                            last_deploy_state = payload.deploy_state;
                            for status in command_manager.observe_telemetry(&payload) {
                                emit_command_status(&app_handle, &status);
                            }
                            if let Some(change) = session_tracker.observe(
                                payload.session_id,
                                payload.restart_reason,
                            ) {
                                if let Some(previous) = change.previous_session_id {
                                    log::warn!(
                                        "airborne session changed: 0x{previous:08X} -> 0x{:08X}, restart_reason={}",
                                        change.session_id,
                                        change.restart_reason
                                    );
                                }
                                let _ = app_handle.emit("airborne-session-changed", &change);
                                log_flight_event(
                                    &app_handle,
                                    "WARN",
                                    &format!(
                                        "airborne session changed: previous={:?}, current={}, restart_reason={}",
                                        change.previous_session_id, change.session_id, change.restart_reason
                                    ),
                                );
                            }
                            {
                                let mut count = total_count.lock().unwrap_or_else(|error| error.into_inner());
                                *count += 1;
                            }
                            let _ = app_handle.emit("update-telemetry", &payload);
                            record_flight_telemetry(
                                &app_handle,
                                &payload,
                                telemetry_now_ms,
                            );
                            Self::emit_stats(&app_handle, &total_count, &failed_count);
                            Self::save_to_database(&app_handle, &payload).await;
                        }
                        ParseResult::Complete(ParsedFrame::Ack(ack)) => {
                            let status = command_manager.handle_ack(&ack);
                            emit_command_status(&app_handle, &status);
                        }
                        ParseResult::IgnoredFrame(frame_type) => {
                            log::debug!("ignoring protocol frame type 0x{frame_type:02X}");
                        }
                        ParseResult::ParseError(error) => {
                            log::warn!("parse error: {error}");
                            {
                                let mut count = failed_count.lock().unwrap_or_else(|poison| poison.into_inner());
                                *count += 1;
                            }
                            {
                                let mut count = total_count.lock().unwrap_or_else(|poison| poison.into_inner());
                                *count += 1;
                            }
                            Self::emit_stats(&app_handle, &total_count, &failed_count);
                            record_parse_error(&app_handle, &error);
                        }
                    }
                }
            }
        }
    }
}

fn command_label(frame_type: u8) -> &'static str {
    match frame_type {
        FRAME_TYPE_SET_TIMER => "SET_TIMER",
        FRAME_TYPE_FORCE_RELEASE => "FORCE_RELEASE",
        _ => "UNKNOWN",
    }
}

fn emit_command_status(app_handle: &AppHandle, status: &CommandStatusEvent) {
    match status.status.as_str() {
        "failed" => log::warn!("{} {:?}: {}", status.command_type, status.command_id, status.detail),
        "ignored_ack" => log::info!("ignored ACK {:?}: {}", status.command_id, status.detail),
        _ => log::info!("{} {:?}: {}", status.command_type, status.command_id, status.status),
    }
    let _ = app_handle.emit("command-status", status);
    log_flight_event(
        app_handle,
        if status.status == "failed" { "ERROR" } else { "INFO" },
        &format!(
            "command type={} id={:?} status={} attempts={} result={:?}: {}",
            status.command_type,
            status.command_id,
            status.status,
            status.attempts,
            status.result,
            status.detail
        ),
    );
}

fn update_link_stats(app_handle: &AppHandle, now_ms: u64) {
    let Some(state) = app_handle.try_state::<crate::state::SerialState>() else {
        return;
    };
    let (stats, outage_started) = {
        let mut tracker = state
            .flight_stats
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let before = tracker.snapshot().link_outages;
        tracker.tick(now_ms);
        let snapshot = tracker.snapshot();
        let started = snapshot.link_outages > before;
        (snapshot, started)
    };
    let _ = app_handle.emit("flight-stats", &stats);
    if outage_started {
        log_flight_event(app_handle, "WARN", "telemetry link-loss interval started");
    }
}

fn record_flight_telemetry(app_handle: &AppHandle, payload: &TelemetryPayload, now_ms: u64) {
    let Some(state) = app_handle.try_state::<crate::state::SerialState>() else {
        return;
    };
    let stats = {
        let mut tracker = state
            .flight_stats
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        tracker.observe_telemetry(payload.session_id, payload.frame_seq, now_ms);
        tracker.snapshot()
    };
    let _ = app_handle.emit("flight-stats", &stats);
    let mut recorder_guard = state
        .flight_recorder
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if let Some(recorder) = recorder_guard.as_mut() {
        if let Err(error) = recorder.record_telemetry(payload, &stats) {
            log::error!("failed to write flight telemetry: {error}");
        }
    }
}

fn record_parse_error(app_handle: &AppHandle, error: &str) {
    if let Some(state) = app_handle.try_state::<crate::state::SerialState>() {
        if error.contains("CRC") {
            state
                .flight_stats
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .record_crc_error();
        }
    }
    log_flight_event(app_handle, "WARN", &format!("protocol parse error: {error}"));
}

fn log_flight_event(app_handle: &AppHandle, level: &str, message: &str) {
    let Some(state) = app_handle.try_state::<crate::state::SerialState>() else {
        return;
    };
    let mut recorder_guard = state
        .flight_recorder
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if let Some(recorder) = recorder_guard.as_mut() {
        if let Err(error) = recorder.log_event(level, message) {
            log::error!("failed to write flight event: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{HalfDuplexSchedule, SessionTracker};

    #[test]
    fn session_tracker_reports_initial_session_and_real_restart_once() {
        let mut tracker = SessionTracker::default();
        let initial = tracker.observe(0x1111_1111, 1).expect("initial session");
        assert_eq!(initial.previous_session_id, None);
        assert_eq!(initial.session_id, 0x1111_1111);
        assert!(tracker.observe(0x1111_1111, 1).is_none());
        let restart = tracker.observe(0x2222_2222, 4).expect("changed session");
        assert_eq!(restart.previous_session_id, Some(0x1111_1111));
        assert_eq!(restart.session_id, 0x2222_2222);
        assert!(tracker.observe(0x2222_2222, 4).is_none());
    }

    #[test]
    fn half_duplex_schedule_reserves_the_telemetry_window() {
        let mut schedule = HalfDuplexSchedule::default();
        assert!(!schedule.command_window_open(100));

        schedule.observe_rx_byte(0);
        schedule.observe_telemetry(0);
        assert!(!schedule.command_window_open(149));
        assert!(schedule.command_window_open(150));
        assert!(schedule.command_window_open(299));
        assert!(!schedule.command_window_open(300));

        schedule.mark_command_transmitted();
        assert!(!schedule.command_window_open(250));
        assert!(!schedule.command_window_open(1_150));

        schedule.observe_telemetry(1_500);
        assert!(schedule.command_window_open(1_650));
    }

    #[test]
    fn half_duplex_schedule_never_writes_while_serial_bytes_are_arriving() {
        let mut schedule = HalfDuplexSchedule::default();
        schedule.observe_telemetry(0);
        schedule.observe_rx_byte(200);
        assert!(!schedule.command_window_open(220));
        assert!(schedule.command_window_open(230));
    }
}

impl SerialReceiver {
    pub fn new(
        app_handle: AppHandle,
        cancellation_token: CancellationToken,
        command_rx: mpsc::UnboundedReceiver<CommandRequest>,
    ) -> Self {
        Self {
            parser: None,
            path: None,
            baud_rate: None,
            serial_stream: None,
            cancellation_token,
            verification_failed_count: Arc::new(Mutex::new(0)),
            total_packet_count: Arc::new(Mutex::new(0)),
            app_handle,
            command_rx: Some(command_rx),
        }
    }

    fn emit_stats(
        app_handle: &AppHandle,
        total_count: &Arc<Mutex<u64>>,
        failed_count: &Arc<Mutex<u32>>,
    ) {
        let total = *total_count.lock().unwrap_or_else(|error| error.into_inner());
        let failed = *failed_count.lock().unwrap_or_else(|error| error.into_inner());
        let _ = app_handle.emit("packet-stats", serde_json::json!({
            "totalPackets": total,
            "failedPackets": failed,
            "packetsPerSecond": 0.0
        }));
    }

    async fn save_to_database(app_handle: &AppHandle, payload: &TelemetryPayload) {
        let db = app_handle.try_state::<crate::state::DbPool>();
        if let Some(pool) = db {
            let pool = pool.inner().0.clone();
            let payload = payload.clone();
            tokio::spawn(async move {
                let result = sqlx::query(
                    "INSERT INTO telemetry (
                        received_at, x_acceleration, y_acceleration, z_acceleration,
                        x_angular_velocity, y_angular_velocity, z_angular_velocity,
                        longitude, latitude, altitude,
                        ground_speed, vertical_velocity, air_pressure, temperature
                    ) VALUES (
                        datetime('now'), ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13
                    )",
                )
                .bind(payload.x_acceleration)
                .bind(payload.y_acceleration)
                .bind(payload.z_acceleration)
                .bind(payload.x_angular_velocity)
                .bind(payload.y_angular_velocity)
                .bind(payload.z_angular_velocity)
                .bind(payload.longitude)
                .bind(payload.latitude)
                .bind(payload.altitude)
                .bind(payload.ground_speed)
                .bind(payload.vertical_velocity)
                .bind(payload.air_pressure)
                .bind(payload.temperature)
                .execute(&pool)
                .await;
                if let Err(error) = result {
                    log::error!("failed to save telemetry to database: {error}");
                }
            });
        }
    }
}
