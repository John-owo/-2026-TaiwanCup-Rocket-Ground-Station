use crate::models::response::{InvokeError, TelemetryPayload};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// 通知中心：透過 Tauri v2 的 emit 機制將事件廣播到前端
pub struct NotificationCenter<'a> {
    pub app_handle: &'a AppHandle,
}

/// 序列埠錯誤響應
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub error_type: String,
    pub detail: String,
}

impl ErrorResponse {
    pub fn new(error_type: String, detail: String) -> Self {
        Self { error_type, detail }
    }
}

impl NotificationCenter<'_> {
    /// 廣播錯誤事件到前端
    pub fn broadcast_error(&self, error: &InvokeError) {
        let response = match error {
            InvokeError::Error(msg) => ErrorResponse::new("ERROR".to_string(), msg.clone()),
            InvokeError::SerialError(msg) => ErrorResponse::new("SERIAL_ERROR".to_string(), msg.clone()),
            InvokeError::ValidationFailed => ErrorResponse::new("VALIDATION_FAILED".to_string(), "packet validation failed".to_string()),
            InvokeError::DatabaseError(msg) => ErrorResponse::new("DATABASE_ERROR".to_string(), msg.clone()),
        };
        let _ = self.app_handle.emit("serial-error", &response);
    }

    /// 將解析後的遙測資料即時推送到前端
    pub fn update_telemetry(&self, payload: &TelemetryPayload) {
        let _ = self.app_handle.emit("update-telemetry", payload);
    }

    /// 推送封包統計資訊到前端
    pub fn update_stats(
        &self,
        total_packets: u64,
        failed_packets: u32,
        packets_per_second: f64,
    ) {
        let _ = self.app_handle.emit("packet-stats", serde_json::json!({
            "totalPackets": total_packets,
            "failedPackets": failed_packets,
            "packetsPerSecond": packets_per_second
        }));
    }
}
