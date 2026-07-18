use serde::{Deserialize, Serialize};

/// 錯誤類型，序列化為前端可讀的 JSON 格式
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(
    rename_all = "SCREAMING_SNAKE_CASE",
    tag = "error_type",
    content = "detail"
)]
pub enum InvokeError {
    Error(String),
    SerialError(String),
    ValidationFailed,
    DatabaseError(String),
}

/// Protocol v1/v2 TELEMETRY 封包，感測值均已還原為物理單位。
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryPayload {
    pub protocol_version: u8,
    pub session_id: u32,
    pub frame_seq: u32,
    pub uptime_ms: u32,
    pub restart_reason: u8,
    pub timer_state: u8,
    pub deploy_state: u8,
    pub sensor_flags: u8,
    pub remaining_s: u32,
    pub last_ack_command_id: u32,
    pub last_ack_result: u8,
    pub x_acceleration: f32,     // m/s²
    pub y_acceleration: f32,     // m/s²
    pub z_acceleration: f32,     // m/s²
    pub x_angular_velocity: f32, // deg/s
    pub y_angular_velocity: f32, // deg/s
    pub z_angular_velocity: f32, // deg/s
    pub longitude: f32,          // degrees
    pub latitude: f32,           // degrees
    pub altitude: f32,           // meters
    pub ground_speed: f32,       // m/s
    pub vertical_velocity: f32,  // m/s
    pub air_pressure: f32,       // hPa
    pub temperature: f32,        // °C
}

/// 空中端 session 第一次出現或改變時通知前端。
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AirborneSessionChanged {
    pub previous_session_id: Option<u32>,
    pub session_id: u32,
    pub restart_reason: u8,
}

/// Protocol v1/v2 ACK frame decoded from the airborne endpoint.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AckPayload {
    pub session_id: u32,
    /// v1 has an independent ACK sequence; compact v2 deliberately omits it.
    pub frame_seq: Option<u32>,
    pub command_id: u32,
    pub acked_type: u8,
    pub result: u8,
    pub timer_state: u8,
    pub deploy_state: u8,
    pub remaining_s: u32,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommandStatusEvent {
    pub command_id: Option<u32>,
    pub command_type: String,
    pub status: String,
    pub attempts: u32,
    pub result: Option<u8>,
    pub detail: String,
}

#[derive(Clone, Debug)]
pub enum ParsedFrame {
    Telemetry(TelemetryPayload),
    Ack(AckPayload),
}

/// 資料庫儲存的遙測記錄，包含時間戳記與資料庫 ID
/// received_at 使用 String 因為 SQLite 以 TEXT 存儲 datetime
#[derive(Clone, Serialize, Deserialize, Debug, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct DbTelemetry {
    pub id: i64,
    pub received_at: String,
    pub x_acceleration: f64,
    pub y_acceleration: f64,
    pub z_acceleration: f64,
    pub x_angular_velocity: f64,
    pub y_angular_velocity: f64,
    pub z_angular_velocity: f64,
    pub longitude: f64,
    pub latitude: f64,
    pub altitude: f64,
    pub ground_speed: f64,
    pub vertical_velocity: f64,
    pub air_pressure: f64,
    pub temperature: f64,
}

/// Tauri command 的標準回傳型別
pub type InvokeResult<T> = Result<T, InvokeError>;
