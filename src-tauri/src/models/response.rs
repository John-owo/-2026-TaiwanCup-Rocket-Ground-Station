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

/// 遙測資料封包，對應序列埠解析後的 13 個感測器欄位
/// 順序與韌體二進位格式相同（Big-Endian f32）
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryPayload {
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
