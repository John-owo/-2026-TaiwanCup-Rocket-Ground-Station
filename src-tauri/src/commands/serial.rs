use crate::infrastructures::serial::receiver::SerialReceiver;
use crate::models::response::{InvokeError, InvokeResult};
use crate::services::serial::Receiver;
use crate::state::SerialState;

use tauri::{AppHandle, Emitter, Manager, State};
use tokio_util::sync::CancellationToken;

/// 預設封包 payload 長度：13 個 f32 = 52 bytes
const EXPECT_PACKET_LENGTH: usize = 52;

/// 開始監控序列埠
/// 前端需傳入 COM port 路徑與鮑率
#[tauri::command]
pub async fn start_monitoring(
    path: String,
    baud_rate: u32,
    serial_state: State<'_, SerialState>,
    app_handle: AppHandle,
) -> InvokeResult<String> {
    // 檢查是否已有正在執行的監控任務
    {
        let token_guard = serial_state.cancellation_token.lock()
            .unwrap_or_else(|e| e.into_inner());
        if token_guard.is_some() {
            return Err(InvokeError::SerialError(
                "monitoring task already running".to_string(),
            ));
        }
    }

    // 建立新的 cancellation token
    let cancellation_token = CancellationToken::new();

    // 儲存到 state 中，讓 stop_monitoring 可以取消
    {
        let mut token_guard = serial_state.cancellation_token.lock()
            .unwrap_or_else(|e| e.into_inner());
        *token_guard = Some(cancellation_token.clone());
    }

    // 儲存路徑與鮑率
    {
        let mut path_guard = serial_state.path.lock()
            .unwrap_or_else(|e| e.into_inner());
        *path_guard = Some(path.clone());
    }
    {
        let mut baud_guard = serial_state.baud_rate.lock()
            .unwrap_or_else(|e| e.into_inner());
        *baud_guard = Some(baud_rate);
    }

    let handle_clone = app_handle.clone();
    let handle_for_cleanup = app_handle.clone();
    let token_clone = cancellation_token.clone();

    // 在背景任務中啟動接收迴圈
    tokio::spawn(async move {
        let mut receiver = SerialReceiver::new(handle_clone, token_clone);

        // 建立序列埠連線
        if let Err(e) = receiver.get_connection(path, baud_rate).await {
            log::error!("serial connection failed: {}", e);
            let _ = handle_for_cleanup.emit(
                "serial-error",
                serde_json::json!({
                    "errorType": "SERIAL_ERROR",
                    "detail": e,
                }),
            );
            // 清除 cancellation token，讓使用者可以重新連線
            if let Some(state) = handle_for_cleanup.try_state::<SerialState>() {
                let mut guard = state.cancellation_token.lock()
                    .unwrap_or_else(|e| e.into_inner());
                *guard = None;
            }
            return;
        }

        // 啟動接收迴圈（會持續執行直到被 cancel）
        match receiver.start_receive(EXPECT_PACKET_LENGTH).await {
            Ok(msg) => log::info!("receive loop ended: {}", msg),
            Err(e) => log::error!("receive loop error: {}", e),
        }

        // 接收迴圈結束後，清除 cancellation token
        if let Some(state) = handle_for_cleanup.try_state::<SerialState>() {
            let mut guard = state.cancellation_token.lock()
                .unwrap_or_else(|e| e.into_inner());
            *guard = None;
        }
    });

    Ok("monitoring started".to_string())
}

/// 停止監控序列埠
#[tauri::command]
pub async fn stop_monitoring(
    serial_state: State<'_, SerialState>,
) -> InvokeResult<String> {
    let mut token_guard = serial_state.cancellation_token.lock()
        .unwrap_or_else(|e| e.into_inner());

    if let Some(cancellation_token) = token_guard.take() {
        cancellation_token.cancel();
        log::info!("monitoring stopped by user");
        Ok("monitoring stopped".to_string())
    } else {
        Err(InvokeError::SerialError(
            "no monitoring task running".to_string(),
        ))
    }
}

/// 查詢遙測歷史記錄
#[tauri::command]
pub async fn get_telemetry_history(
    limit: i64,
    db_pool: State<'_, crate::state::DbPool>,
) -> InvokeResult<Vec<crate::models::response::DbTelemetry>> {
    let rows = sqlx::query_as::<_, crate::models::response::DbTelemetry>(
        "SELECT id, received_at, x_acceleration, y_acceleration, z_acceleration,
                x_angular_velocity, y_angular_velocity, z_angular_velocity,
                longitude, latitude, altitude,
                ground_speed, vertical_velocity, air_pressure, temperature
         FROM telemetry ORDER BY id DESC LIMIT ?1"
    )
    .bind(limit)
    .fetch_all(&db_pool.0)
    .await
    .map_err(|e| InvokeError::DatabaseError(e.to_string()))?;

    Ok(rows)
}
