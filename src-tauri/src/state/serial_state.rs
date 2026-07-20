use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;
use tokio::sync::mpsc;
use tokio::sync::Notify;
use crate::infrastructures::serial::command::CommandRequest;
use crate::infrastructures::flight::FlightStatsTracker;
use crate::infrastructures::flight::LINK_LOSS_THRESHOLD_MS;
use crate::models::response::TestSessionStatus;

#[derive(Default)]
pub struct AirborneLinkState {
    current_session_id: Option<u32>,
    last_valid_telemetry_at: Option<Instant>,
}

impl AirborneLinkState {
    pub fn observe_telemetry(&mut self, session_id: u32, now: Instant) {
        self.current_session_id = Some(session_id);
        self.last_valid_telemetry_at = Some(now);
    }

    pub fn live_session_id(&self, now: Instant) -> Option<u32> {
        let last = self.last_valid_telemetry_at?;
        if now.saturating_duration_since(last)
            <= Duration::from_millis(LINK_LOSS_THRESHOLD_MS)
        {
            self.current_session_id
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

/// 序列埠監控的應用程式狀態
/// 透過 Tauri managed state 在所有 command 間共享
pub struct SerialState {
    /// 序列埠路徑 (e.g., "COM3", "/dev/ttyUSB0")
    pub path: Mutex<Option<String>>,
    /// 鮑率
    pub baud_rate: Mutex<Option<u32>>,
    /// 取消令牌：用來控制接收迴圈的生命週期
    pub cancellation_token: Mutex<Option<CancellationToken>>,
    /// Active receiver command channel. All writes stay in the serial receive task.
    pub command_tx: Mutex<Option<mpsc::UnboundedSender<CommandRequest>>>,
    /// CRC 驗證失敗計數
    pub verification_failed_count: Arc<Mutex<u32>>,
    /// 總封包計數
    pub total_packet_count: Arc<Mutex<u64>>,
    pub flight_stats: Arc<Mutex<FlightStatsTracker>>,
    pub airborne_link: Arc<Mutex<AirborneLinkState>>,
    pub test_session_status: Arc<Mutex<TestSessionStatus>>,
    pub manual_stop_requested: AtomicBool,
    pub terminal_notify: Arc<Notify>,
    pub shutdown_started: AtomicBool,
}

impl SerialState {
    pub fn new(path: String, baud_rate: u32) -> Self {
        Self {
            path: Mutex::new(Some(path)),
            baud_rate: Mutex::new(Some(baud_rate)),
            cancellation_token: Mutex::new(None),
            command_tx: Mutex::new(None),
            verification_failed_count: Arc::new(Mutex::new(0)),
            total_packet_count: Arc::new(Mutex::new(0)),
            flight_stats: Arc::new(Mutex::new(FlightStatsTracker::default())),
            airborne_link: Arc::new(Mutex::new(AirborneLinkState::default())),
            test_session_status: Arc::new(Mutex::new(TestSessionStatus::default())),
            manual_stop_requested: AtomicBool::new(false),
            terminal_notify: Arc::new(Notify::new()),
            shutdown_started: AtomicBool::new(false),
        }
    }
}

impl Default for SerialState {
    fn default() -> Self {
        Self {
            path: Mutex::new(None),
            baud_rate: Mutex::new(None),
            cancellation_token: Mutex::new(None),
            command_tx: Mutex::new(None),
            verification_failed_count: Arc::new(Mutex::new(0)),
            total_packet_count: Arc::new(Mutex::new(0)),
            flight_stats: Arc::new(Mutex::new(FlightStatsTracker::default())),
            airborne_link: Arc::new(Mutex::new(AirborneLinkState::default())),
            test_session_status: Arc::new(Mutex::new(TestSessionStatus::default())),
            manual_stop_requested: AtomicBool::new(false),
            terminal_notify: Arc::new(Notify::new()),
            shutdown_started: AtomicBool::new(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AirborneLinkState;
    use std::time::{Duration, Instant};

    #[test]
    fn live_session_requires_recent_valid_telemetry() {
        let mut link = AirborneLinkState::default();
        let started = Instant::now();
        assert_eq!(link.live_session_id(started), None);

        link.observe_telemetry(0x1234_5678, started);
        assert_eq!(
            link.live_session_id(started + Duration::from_millis(4_500)),
            Some(0x1234_5678),
        );
        assert_eq!(
            link.live_session_id(started + Duration::from_millis(4_501)),
            None,
        );

        link.clear();
        assert_eq!(link.live_session_id(started), None);
    }
}
