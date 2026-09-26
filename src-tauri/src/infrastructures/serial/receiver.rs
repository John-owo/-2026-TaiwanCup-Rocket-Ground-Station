//! Protocol session over an abstract link.
//!
//! `LinkReceiver` owns one opened [`LinkIo`] and runs the protocol session:
//! framing bytes into telemetry / ACK frames, keeping the half-duplex uplink
//! window, retrying commands, and publishing statistics. It never touches a
//! serial port directly; opening the pipe is the transport's job
//! (`infrastructures::link`).

use crate::infrastructures::flight::LINK_LOSS_THRESHOLD_MS;
use crate::infrastructures::link::{LinkIo, TransportConfig};
use crate::infrastructures::serial::command::{
    CommandManager, CommandRequest, COMMAND_RETRY_INTERVAL_MS, FRAME_TYPE_FORCE_RELEASE,
    FRAME_TYPE_SET_TIMER,
};
use crate::infrastructures::serial::parser::{PacketParser, ParseResult};
use crate::models::response::{
    AirborneSessionChanged, CommandStatusEvent, ParsedFrame, TelemetryPayload,
};
use crate::services::serial::{Parser, Receiver};

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio::time::MissedTickBehavior;
use tokio_util::sync::CancellationToken;

const COMMAND_WINDOW_START_MS: u64 = 150;
const COMMAND_WINDOW_END_MS: u64 = 300;
const SERIAL_RX_IDLE_GUARD_MS: u64 = 30;
const PACKET_RATE_WINDOW_MS: u64 = 10_000;
const STATS_EMIT_INTERVAL_MS: u64 = 500;
/// Read buffer for one link read. Frames are at most 118 bytes, so this
/// comfortably covers a telemetry frame plus a stray ACK in one read.
const LINK_READ_BUFFER_BYTES: usize = 512;

#[derive(Default)]
struct PacketRateTracker {
    telemetry_timestamps_ms: VecDeque<u64>,
}

impl PacketRateTracker {
    fn observe(&mut self, now_ms: u64) {
        self.telemetry_timestamps_ms.push_back(now_ms);
        self.prune(now_ms);
    }

    fn packets_per_second(&mut self, now_ms: u64) -> f64 {
        self.prune(now_ms);
        if self.telemetry_timestamps_ms.len() < 2 {
            return 0.0;
        }
        let oldest = *self
            .telemetry_timestamps_ms
            .front()
            .expect("at least two telemetry timestamps");
        let span_ms = now_ms.saturating_sub(oldest);
        if span_ms == 0 {
            return 0.0;
        }
        let intervals = self.telemetry_timestamps_ms.len().saturating_sub(1) as f64;
        intervals * 1_000.0 / span_ms as f64
    }

    fn prune(&mut self, now_ms: u64) {
        let cutoff_ms = now_ms.saturating_sub(PACKET_RATE_WINDOW_MS);
        while self
            .telemetry_timestamps_ms
            .front()
            .is_some_and(|timestamp| *timestamp < cutoff_ms)
        {
            self.telemetry_timestamps_ms.pop_front();
        }
    }
}

/// Timing rules of the radio's half-duplex uplink window.
///
/// These describe the *radio module*, not the transport: an E22 behind a TCP
/// bridge still needs the same quiet gap after each telemetry frame. A future
/// module adapter supplies its own values; the defaults are the E22 figures
/// the current avionics were tuned against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HalfDuplexTiming {
    /// Earliest uplink moment after a complete telemetry frame.
    pub window_start_ms: u64,
    /// Exclusive end of the uplink window after a complete telemetry frame.
    pub window_end_ms: u64,
    /// Minimum silence on the receive side before writing.
    pub rx_idle_guard_ms: u64,
}

impl Default for HalfDuplexTiming {
    fn default() -> Self {
        Self {
            window_start_ms: COMMAND_WINDOW_START_MS,
            window_end_ms: COMMAND_WINDOW_END_MS,
            rx_idle_guard_ms: SERIAL_RX_IDLE_GUARD_MS,
        }
    }
}

#[derive(Default)]
struct HalfDuplexSchedule {
    timing: HalfDuplexTiming,
    last_telemetry_ms: Option<u64>,
    last_rx_byte_ms: Option<u64>,
    command_sent_for_current_telemetry: bool,
}

impl HalfDuplexSchedule {
    fn new(timing: HalfDuplexTiming) -> Self {
        Self {
            timing,
            ..Self::default()
        }
    }

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
        if self.last_rx_byte_ms.is_some_and(|last_rx_ms| {
            now_ms.saturating_sub(last_rx_ms) < self.timing.rx_idle_guard_ms
        }) {
            return false;
        }
        let elapsed_ms = now_ms.saturating_sub(last_telemetry_ms);
        (self.timing.window_start_ms..self.timing.window_end_ms).contains(&elapsed_ms)
    }

    fn mark_command_transmitted(&mut self) {
        self.command_sent_for_current_telemetry = true;
    }
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

/// Owns one link and runs the protocol session on it.
pub struct LinkReceiver {
    pub link: Option<LinkIo>,
    pub timing: HalfDuplexTiming,
    pub cancellation_token: CancellationToken,
    pub verification_failed_count: Arc<Mutex<u32>>,
    pub total_packet_count: Arc<Mutex<u64>>,
    pub app_handle: AppHandle,
    pub command_rx: Option<mpsc::UnboundedReceiver<CommandRequest>>,
}

impl Receiver for LinkReceiver {
    async fn connect(&mut self, config: TransportConfig) -> Result<(), String> {
        config.validate()?;
        let transport = config.into_transport();
        let link = transport.open().await?;
        log::info!(
            "link connected: {} ({:?})",
            link.label,
            link.capabilities.kind
        );
        self.link = Some(link);
        Ok(())
    }

    async fn start_receive(&mut self) -> Result<String, String> {
        let link = self.link.take().ok_or("link not connected")?;
        let command_rx = self
            .command_rx
            .take()
            .ok_or("command receiver not initialized")?;
        let mut session = LinkSession::new(
            self.app_handle.clone(),
            PacketParser::default(),
            self.timing,
            self.verification_failed_count.clone(),
            self.total_packet_count.clone(),
        );
        session
            .run(link, command_rx, self.cancellation_token.clone())
            .await
    }
}

impl LinkReceiver {
    pub fn new(
        app_handle: AppHandle,
        cancellation_token: CancellationToken,
        command_rx: mpsc::UnboundedReceiver<CommandRequest>,
    ) -> Self {
        Self {
            link: None,
            timing: HalfDuplexTiming::default(),
            cancellation_token,
            verification_failed_count: Arc::new(Mutex::new(0)),
            total_packet_count: Arc::new(Mutex::new(0)),
            app_handle,
            command_rx: Some(command_rx),
        }
    }
}

/// Per-run protocol state. Every handler is a plain method so the select loop
/// in [`LinkSession::run`] only routes events.
struct LinkSession {
    app_handle: AppHandle,
    parser: PacketParser,
    command_manager: CommandManager,
    session_tracker: SessionTracker,
    half_duplex: HalfDuplexSchedule,
    packet_rate: PacketRateTracker,
    failed_count: Arc<Mutex<u32>>,
    total_count: Arc<Mutex<u64>>,
    started: Instant,
    last_telemetry_ms: Option<u64>,
    last_deploy_state: u8,
    last_stats_emit_ms: u64,
}

impl LinkSession {
    fn new(
        app_handle: AppHandle,
        parser: PacketParser,
        timing: HalfDuplexTiming,
        failed_count: Arc<Mutex<u32>>,
        total_count: Arc<Mutex<u64>>,
    ) -> Self {
        Self {
            app_handle,
            parser,
            command_manager: CommandManager::default(),
            session_tracker: SessionTracker::default(),
            half_duplex: HalfDuplexSchedule::new(timing),
            packet_rate: PacketRateTracker::default(),
            failed_count,
            total_count,
            started: Instant::now(),
            last_telemetry_ms: None,
            last_deploy_state: 0,
            last_stats_emit_ms: 0,
        }
    }

    fn now_ms(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }

    async fn run(
        &mut self,
        link: LinkIo,
        mut command_rx: mpsc::UnboundedReceiver<CommandRequest>,
        cancellation_token: CancellationToken,
    ) -> Result<String, String> {
        let LinkIo {
            stream,
            capabilities,
            label,
        } = link;
        log::info!(
            "protocol session started on {label} (stream_oriented={}, full_duplex={})",
            capabilities.stream_oriented,
            capabilities.full_duplex
        );
        let (mut reader, mut writer) = tokio::io::split(stream);
        let mut read_buffer = [0_u8; LINK_READ_BUFFER_BYTES];
        let mut resend_tick =
            tokio::time::interval(Duration::from_millis(COMMAND_RETRY_INTERVAL_MS));
        resend_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                biased;

                _ = cancellation_token.cancelled() => {
                    log::info!("receive loop cancelled gracefully");
                    return Ok("receive loop stopped gracefully".to_string());
                }

                Some(request) = command_rx.recv() => {
                    self.on_command_request(request);
                }

                _ = resend_tick.tick() => {
                    self.on_tick(&mut writer).await?;
                }

                result = reader.read(&mut read_buffer) => {
                    match result {
                        Ok(0) => {
                            let detail = format!("link closed by peer: {label}");
                            self.fail_link(&detail);
                            return Err(detail);
                        }
                        Ok(count) => self.on_rx_bytes(&read_buffer[..count]),
                        Err(error) => {
                            let detail = format!("serial read error: {error}");
                            self.fail_link(&detail);
                            return Err(detail);
                        }
                    }
                }
            }
        }
    }

    fn fail_link(&self, detail: &str) {
        log_flight_event(&self.app_handle, "ERROR", detail);
        let _ = self.app_handle.emit(
            "serial-error",
            serde_json::json!({
                "errorType": "SERIAL_ERROR",
                "detail": detail,
            }),
        );
    }

    fn on_command_request(&mut self, request: CommandRequest) {
        match self.command_manager.request(request) {
            Ok(status) => emit_command_status(&self.app_handle, &status),
            Err(error) => emit_command_status(&self.app_handle, &failed_status(error)),
        }
    }

    /// Periodic housekeeping plus the only place that writes to the link.
    async fn on_tick<W: AsyncWrite + Unpin>(&mut self, writer: &mut W) -> Result<(), String> {
        let now_ms = self.now_ms();
        update_link_stats(&self.app_handle, now_ms);
        if now_ms.saturating_sub(self.last_stats_emit_ms) >= STATS_EMIT_INTERVAL_MS {
            self.emit_stats(now_ms);
            self.last_stats_emit_ms = now_ms;
        }
        if self
            .last_telemetry_ms
            .is_some_and(|last| now_ms.saturating_sub(last) > LINK_LOSS_THRESHOLD_MS)
        {
            if let Some(status) = self.command_manager.cancel_pending_force(
                "telemetry link lost; FORCE_RELEASE cancelled and will not be replayed",
            ) {
                emit_command_status(&self.app_handle, &status);
            }
        }
        if !self.half_duplex.command_window_open(now_ms) {
            return Ok(());
        }
        let transmission = match self.command_manager.next_transmission() {
            Ok(Some(transmission)) => transmission,
            Ok(None) => return Ok(()),
            Err(error) => {
                emit_command_status(&self.app_handle, &failed_status(error));
                return Ok(());
            }
        };
        if let Err(error) = writer.write_all(&transmission.bytes).await {
            let detail = format!("serial command write error: {error}");
            self.fail_link(&detail);
            return Err(detail);
        }
        if let Err(error) = writer.flush().await {
            let detail = format!("serial command flush error: {error}");
            log_flight_event(&self.app_handle, "ERROR", &detail);
            return Err(detail);
        }
        self.half_duplex.mark_command_transmitted();
        let status = CommandStatusEvent {
            command_id: Some(transmission.command_id),
            command_type: command_label(transmission.command_type).to_string(),
            status: "sending".to_string(),
            attempts: transmission.attempts,
            result: None,
            detail: format!(
                "Protocol v{} command transmitted in half-duplex uplink window; waiting for matching ACK",
                transmission.protocol_version
            ),
        };
        emit_command_status(&self.app_handle, &status);
        Ok(())
    }

    fn on_rx_bytes(&mut self, bytes: &[u8]) {
        let rx_now_ms = self.now_ms();
        self.half_duplex.observe_rx_byte(rx_now_ms);
        for &byte in bytes {
            match self.parser.sink(byte) {
                ParseResult::Incomplete => {}
                ParseResult::Complete(ParsedFrame::Telemetry(payload)) => {
                    self.on_telemetry(payload);
                }
                ParseResult::Complete(ParsedFrame::Ack(ack)) => {
                    let status = self.command_manager.handle_ack(&ack);
                    emit_command_status(&self.app_handle, &status);
                }
                ParseResult::IgnoredFrame(frame_type) => {
                    log::debug!("ignoring protocol frame type 0x{frame_type:02X}");
                }
                ParseResult::ParseError(error) => {
                    self.on_parse_error(&error, rx_now_ms);
                }
            }
        }
    }

    fn on_telemetry(&mut self, payload: TelemetryPayload) {
        let now_ms = self.now_ms();
        self.half_duplex.observe_telemetry(now_ms);
        if let Some(previous_ms) = self.last_telemetry_ms {
            let gap_ms = now_ms.saturating_sub(previous_ms);
            if gap_ms > LINK_LOSS_THRESHOLD_MS {
                log_flight_event(
                    &self.app_handle,
                    "WARN",
                    &format!("telemetry link recovered after {gap_ms} ms"),
                );
            }
        }
        self.last_telemetry_ms = Some(now_ms);
        self.packet_rate.observe(now_ms);
        if let Some(state) = self.app_handle.try_state::<crate::state::SerialState>() {
            state
                .airborne_link
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .observe_telemetry(payload.session_id, Instant::now());
        }
        if self.last_deploy_state == 0 && payload.deploy_state == 1 {
            log_flight_event(
                &self.app_handle,
                "WARN",
                "airborne telemetry changed to DEPLOYED",
            );
        }
        self.last_deploy_state = payload.deploy_state;
        for status in self.command_manager.observe_telemetry(&payload) {
            emit_command_status(&self.app_handle, &status);
        }
        if let Some(change) = self
            .session_tracker
            .observe(payload.session_id, payload.restart_reason)
        {
            if let Some(previous) = change.previous_session_id {
                log::warn!(
                    "airborne session changed: 0x{previous:08X} -> 0x{:08X}, restart_reason={}",
                    change.session_id,
                    change.restart_reason
                );
            }
            let _ = self.app_handle.emit("airborne-session-changed", &change);
            log_flight_event(
                &self.app_handle,
                "WARN",
                &format!(
                    "airborne session changed: previous={:?}, current={}, restart_reason={}",
                    change.previous_session_id, change.session_id, change.restart_reason
                ),
            );
        }
        {
            let mut count = self
                .total_count
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            *count += 1;
        }
        let _ = self.app_handle.emit("update-telemetry", &payload);
        record_flight_telemetry(&self.app_handle, &payload, now_ms);
        self.emit_stats(now_ms);
    }

    fn on_parse_error(&mut self, error: &str, now_ms: u64) {
        log::warn!("parse error: {error}");
        {
            let mut count = self
                .failed_count
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            *count += 1;
        }
        {
            let mut count = self
                .total_count
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            *count += 1;
        }
        self.emit_stats(now_ms);
        record_parse_error(&self.app_handle, error);
    }

    fn emit_stats(&mut self, now_ms: u64) {
        let packets_per_second = self.packet_rate.packets_per_second(now_ms);
        let total = *self
            .total_count
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let failed = *self
            .failed_count
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let _ = self.app_handle.emit(
            "packet-stats",
            serde_json::json!({
                "totalPackets": total,
                "failedPackets": failed,
                "packetsPerSecond": packets_per_second
            }),
        );
    }
}

fn failed_status(detail: String) -> CommandStatusEvent {
    CommandStatusEvent {
        command_id: None,
        command_type: "UNKNOWN".to_string(),
        status: "failed".to_string(),
        attempts: 0,
        result: None,
        detail,
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
    if let Some(storage) = app_handle.try_state::<crate::state::StorageState>() {
        if let Err(error) = storage.enqueue_telemetry(app_handle, payload.clone(), stats) {
            log::error!("failed to queue flight telemetry: {error}");
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
    if let Some(storage) = app_handle.try_state::<crate::state::StorageState>() {
        if let Err(error) = storage.enqueue_event(app_handle, level, message) {
            log::error!("failed to queue flight event: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{HalfDuplexSchedule, HalfDuplexTiming, PacketRateTracker, SessionTracker};

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

    #[test]
    fn half_duplex_timing_defaults_match_the_e22_window() {
        let timing = HalfDuplexTiming::default();
        assert_eq!(timing.window_start_ms, 150);
        assert_eq!(timing.window_end_ms, 300);
        assert_eq!(timing.rx_idle_guard_ms, 30);
    }

    #[test]
    fn half_duplex_schedule_honours_custom_module_timing() {
        let mut schedule = HalfDuplexSchedule::new(HalfDuplexTiming {
            window_start_ms: 50,
            window_end_ms: 120,
            rx_idle_guard_ms: 10,
        });
        schedule.observe_telemetry(0);
        assert!(!schedule.command_window_open(49));
        assert!(schedule.command_window_open(50));
        assert!(schedule.command_window_open(119));
        assert!(!schedule.command_window_open(120));

        schedule.observe_rx_byte(60);
        assert!(!schedule.command_window_open(69));
        assert!(schedule.command_window_open(70));
    }

    #[test]
    fn packet_rate_tracks_the_formal_1800_ms_telemetry_interval() {
        let mut tracker = PacketRateTracker::default();
        for now_ms in [0, 1_800, 3_600, 5_400, 7_200, 9_000] {
            tracker.observe(now_ms);
        }
        assert!((tracker.packets_per_second(9_000) - 0.555_555).abs() < 0.001);
    }

    #[test]
    fn packet_rate_reports_bursts_and_decays_to_zero_when_idle() {
        let mut tracker = PacketRateTracker::default();
        for now_ms in [0, 100, 200] {
            tracker.observe(now_ms);
        }
        assert!((tracker.packets_per_second(200) - 10.0).abs() < 0.001);
        assert_eq!(tracker.packets_per_second(10_201), 0.0);
    }
}
