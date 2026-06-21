use crate::infrastructures::serial::parser::{PacketParser, PacketVerificator, ParseResult, TelemetryDecoder};
use crate::models::response::TelemetryPayload;
use crate::services::serial::{Parser, Receiver};

use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncReadExt;
use tokio_serial::{SerialPortBuilderExt, SerialStream};
use tokio_util::sync::CancellationToken;

/// 序列埠接收器
/// 負責建立連線、啟動接收迴圈、解析封包、廣播事件
pub struct SerialReceiver {
    pub parser: Option<PacketParser>,
    pub path: Option<String>,
    pub baud_rate: Option<u32>,
    pub serial_stream: Option<SerialStream>,
    pub cancellation_token: CancellationToken,
    pub verification_failed_count: Arc<Mutex<u32>>,
    pub total_packet_count: Arc<Mutex<u64>>,
    pub app_handle: AppHandle,
}

impl Receiver for SerialReceiver {
    async fn get_connection(&mut self, path: String, baud_rate: u32) -> Result<(), String> {
        match tokio_serial::new(&path, baud_rate).open_native_async() {
            Ok(serial_stream) => {
                self.serial_stream = Some(serial_stream);
                self.path = Some(path);
                self.baud_rate = Some(baud_rate);
                log::info!("serial port connected: {} @ {}", self.path.as_ref().unwrap(), baud_rate);
                Ok(())
            }
            Err(e) => Err(format!("failed to open serial port: {}", e)),
        }
    }

    async fn start_receive(&mut self, expect_packet_length: usize) -> Result<String, String> {
        if self.serial_stream.is_none() {
            return Err("not connected to serial port".to_string());
        }

        // 建立 parser（使用預設 verificator + decoder）
        let parser = PacketParser::new(
            expect_packet_length,
            Box::new(PacketVerificator::new()),
            Box::new(TelemetryDecoder),
        );
        self.parser = Some(parser);

        self.receive_task().await
    }

    async fn receive_task(&mut self) -> Result<String, String> {
        let serial_stream = self.serial_stream.as_mut()
            .ok_or("serial stream not available")?;
        let parser = self.parser.as_mut()
            .ok_or("parser not initialized")?;

        let failed_count = self.verification_failed_count.clone();
        let total_count = self.total_packet_count.clone();
        let app_handle = self.app_handle.clone();
        let cancellation_token = self.cancellation_token.clone();

        loop {
            tokio::select! {
                biased; // 優先檢查 cancellation token 以盡快退出

                _ = cancellation_token.cancelled() => {
                    log::info!("receive loop cancelled gracefully");
                    return Ok("receive loop stopped gracefully".to_string());
                }

                result = serial_stream.read_u8() => {
                    let byte = match result {
                        Ok(b) => b,
                        Err(e) => {
                            let err_msg = format!("serial read error: {}", e);
                            log::error!("{}", err_msg);
                            let _ = app_handle.emit("serial-error", serde_json::json!({
                                "errorType": "SERIAL_ERROR",
                                "detail": err_msg
                            }));
                            return Err(err_msg);
                        }
                    };

                    match parser.sink(byte) {
                        ParseResult::Incomplete => {}
                        ParseResult::Complete(payload) => {
                            // 更新總封包計數
                            {
                                let mut count = total_count.lock().unwrap();
                                *count += 1;
                            }

                            // 廣播遙測資料到前端
                            let _ = app_handle.emit("update-telemetry", &payload);

                            // 廣播封包統計
                            Self::emit_stats(&app_handle, &total_count, &failed_count);

                            // 異步寫入資料庫
                            Self::save_to_database(&app_handle, &payload).await;
                        }
                        ParseResult::ParseError(e) => {
                            log::warn!("parse error: {}", e);
                            // 更新失敗計數
                            {
                                let mut count = failed_count.lock().unwrap();
                                *count += 1;
                            }
                            // 也更新總封包計數（包含失敗）
                            {
                                let mut count = total_count.lock().unwrap();
                                *count += 1;
                            }
                            // 廣播統計更新
                            Self::emit_stats(&app_handle, &total_count, &failed_count);
                        }
                    }
                }
            }
        }
    }
}

impl SerialReceiver {
    /// 建立新的 SerialReceiver
    pub fn new(app_handle: AppHandle, cancellation_token: CancellationToken) -> Self {
        Self {
            parser: None,
            path: None,
            baud_rate: None,
            serial_stream: None,
            cancellation_token,
            verification_failed_count: Arc::new(Mutex::new(0)),
            total_packet_count: Arc::new(Mutex::new(0)),
            app_handle,
        }
    }

    /// 廣播封包統計到前端
    fn emit_stats(
        app_handle: &AppHandle,
        total_count: &Arc<Mutex<u64>>,
        failed_count: &Arc<Mutex<u32>>,
    ) {
        let total = *total_count.lock().unwrap();
        let failed = *failed_count.lock().unwrap();
        let _ = app_handle.emit("packet-stats", serde_json::json!({
            "totalPackets": total,
            "failedPackets": failed,
            "packetsPerSecond": 0.0  // 前端自行計算 Hz
        }));
    }

    /// 異步將遙測資料寫入 SQLite 資料庫
    async fn save_to_database(app_handle: &AppHandle, payload: &TelemetryPayload) {
        // 從 Tauri app state 取得資料庫連線池
        let db = app_handle.try_state::<crate::state::DbPool>();
        if let Some(pool) = db {
            let pool = pool.inner().0.clone();
            let p = payload.clone();
            // spawn 一個背景任務，不阻塞主接收迴圈
            tokio::spawn(async move {
                let result = sqlx::query(
                    "INSERT INTO telemetry (
                        received_at, x_acceleration, y_acceleration, z_acceleration,
                        x_angular_velocity, y_angular_velocity, z_angular_velocity,
                        longitude, latitude, altitude,
                        ground_speed, vertical_velocity, air_pressure, temperature
                    ) VALUES (
                        datetime('now'), ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13
                    )"
                )
                .bind(p.x_acceleration)
                .bind(p.y_acceleration)
                .bind(p.z_acceleration)
                .bind(p.x_angular_velocity)
                .bind(p.y_angular_velocity)
                .bind(p.z_angular_velocity)
                .bind(p.longitude)
                .bind(p.latitude)
                .bind(p.altitude)
                .bind(p.ground_speed)
                .bind(p.vertical_velocity)
                .bind(p.air_pressure)
                .bind(p.temperature)
                .execute(&pool)
                .await;

                if let Err(e) = result {
                    log::error!("failed to save telemetry to database: {}", e);
                }
            });
        }
    }
}
