use crate::infrastructures::serial::receiver::SerialReceiver;
use crate::infrastructures::serial::command::CommandRequest;
use crate::infrastructures::flight::{FlightRecorder, FlightSessionMetadata, FlightStats};
use crate::models::response::{InvokeError, InvokeResult};
use crate::services::serial::Receiver;
use crate::state::SerialState;

use tauri::{AppHandle, Manager, State};
use tokio_util::sync::CancellationToken;

fn reserve_monitoring(serial_state: &SerialState) -> InvokeResult<CancellationToken> {
    let mut token_guard = serial_state
        .cancellation_token
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    if token_guard.is_some() {
        return Err(InvokeError::SerialError(
            "monitoring task already running".to_string(),
        ));
    }

    let cancellation_token = CancellationToken::new();
    *token_guard = Some(cancellation_token.clone());
    Ok(cancellation_token)
}

fn release_monitoring(serial_state: &SerialState, cancellation_token: &CancellationToken) {
    cancellation_token.cancel();
    let mut token_guard = serial_state
        .cancellation_token
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    if token_guard
        .as_ref()
        .is_some_and(CancellationToken::is_cancelled)
    {
        *token_guard = None;
        let mut command_guard = serial_state
            .command_tx
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *command_guard = None;
    }
}

/// 列出目前系統可用的序列埠名稱。
#[tauri::command]
pub async fn list_serial_ports() -> InvokeResult<Vec<String>> {
    let mut ports = tokio_serial::available_ports()
        .map_err(|error| InvokeError::SerialError(error.to_string()))?
        .into_iter()
        .map(|port| port.port_name)
        .collect::<Vec<_>>();
    ports.sort();
    Ok(ports)
}

/// 開始監控序列埠
/// 前端需傳入 COM port 路徑與鮑率
#[tauri::command]
pub async fn start_monitoring(
    path: String,
    baud_rate: u32,
    serial_state: State<'_, SerialState>,
    app_handle: AppHandle,
) -> InvokeResult<String> {
    // 原子性地保留啟動權，避免兩個 start command 同時通過檢查。
    let cancellation_token = reserve_monitoring(&serial_state)?;
    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let mut receiver = SerialReceiver::new(
        app_handle.clone(),
        cancellation_token.clone(),
        command_rx,
    );

    // 先實際開啟序列埠；失敗時由 invoke reject 直接回報前端。
    if let Err(error) = receiver.get_connection(path.clone(), baud_rate).await {
        release_monitoring(&serial_state, &cancellation_token);
        return Err(InvokeError::SerialError(error));
    }

    if cancellation_token.is_cancelled() {
        release_monitoring(&serial_state, &cancellation_token);
        return Err(InvokeError::SerialError(
            "monitoring start cancelled".to_string(),
        ));
    }

    // 開埠成功後才儲存路徑與鮑率。
    {
        let mut path_guard = serial_state.path.lock().unwrap_or_else(|e| e.into_inner());
        *path_guard = Some(path.clone());
    }
    {
        let mut command_guard = serial_state
            .command_tx
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *command_guard = Some(command_tx);
    }
    log_session_event(
        &serial_state,
        "INFO",
        &format!("serial connected: {path} @ {baud_rate}"),
    );
    {
        let mut baud_guard = serial_state
            .baud_rate
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *baud_guard = Some(baud_rate);
    }

    let handle_for_cleanup = app_handle.clone();
    let token_for_cleanup = cancellation_token.clone();

    // 開埠成功後，才在背景任務中啟動接收迴圈。
    tokio::spawn(async move {
        // 啟動接收迴圈（會持續執行直到被 cancel）
        match receiver.start_receive().await {
            Ok(msg) => log::info!("receive loop ended: {}", msg),
            Err(e) => log::error!("receive loop error: {}", e),
        }

        // 只釋放這個任務的保留，不清掉後續新任務。
        if let Some(state) = handle_for_cleanup.try_state::<SerialState>() {
            release_monitoring(state.inner(), &token_for_cleanup);
        }
    });

    Ok("monitoring started".to_string())
}

fn queue_command(serial_state: &SerialState, request: CommandRequest) -> InvokeResult<()> {
    let guard = serial_state
        .command_tx
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let sender = guard.as_ref().ok_or_else(|| {
        InvokeError::SerialError("monitoring must be running before sending commands".to_string())
    })?;
    sender
        .send(request)
        .map_err(|_| InvokeError::SerialError("serial command queue is closed".to_string()))
}

#[tauri::command]
pub async fn set_timer(duration_s: u32, serial_state: State<'_, SerialState>) -> InvokeResult<String> {
    if duration_s == 0 {
        return Err(InvokeError::ValidationFailed);
    }
    queue_command(&serial_state, CommandRequest::SetTimer(duration_s))?;
    log_session_event(&serial_state, "INFO", &format!("SET_TIMER requested: {duration_s} s"));
    Ok("SET_TIMER queued".to_string())
}

#[tauri::command]
pub async fn force_release(serial_state: State<'_, SerialState>) -> InvokeResult<String> {
    queue_command(&serial_state, CommandRequest::ForceRelease)?;
    log_session_event(&serial_state, "WARN", "FORCE_RELEASE requested after UI safety unlock");
    Ok("FORCE_RELEASE queued".to_string())
}

fn log_session_event(serial_state: &SerialState, level: &str, message: &str) {
    let mut guard = serial_state
        .flight_recorder
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if let Some(recorder) = guard.as_mut() {
        if let Err(error) = recorder.log_event(level, message) {
            log::error!("failed to write session log: {error}");
        }
    }
}

#[tauri::command]
pub async fn start_flight_session(
    metadata: FlightSessionMetadata,
    serial_state: State<'_, SerialState>,
    app_handle: AppHandle,
) -> InvokeResult<String> {
    let root = app_handle
        .path()
        .app_data_dir()
        .map_err(|error| InvokeError::DatabaseError(error.to_string()))?;
    let mut stats_guard = serial_state
        .flight_stats
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut recorder_guard = serial_state
        .flight_recorder
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if recorder_guard.is_some() {
        return Err(InvokeError::ValidationFailed);
    }
    let recorder = FlightRecorder::start(&root, metadata)
        .map_err(InvokeError::DatabaseError)?;
    let directory = recorder.directory().display().to_string();
    stats_guard.reset();
    *recorder_guard = Some(recorder);
    Ok(directory)
}

#[tauri::command]
pub async fn stop_flight_session(
    serial_state: State<'_, SerialState>,
) -> InvokeResult<String> {
    let stats = serial_state
        .flight_stats
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .snapshot();
    let mut recorder_guard = serial_state
        .flight_recorder
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let recorder = recorder_guard.as_mut().ok_or(InvokeError::ValidationFailed)?;
    recorder.finish(&stats).map_err(InvokeError::DatabaseError)?;
    let directory = recorder.directory().display().to_string();
    *recorder_guard = None;
    Ok(directory)
}

#[tauri::command]
pub async fn get_flight_stats(serial_state: State<'_, SerialState>) -> InvokeResult<FlightStats> {
    Ok(serial_state
        .flight_stats
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .snapshot())
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
        *serial_state
            .command_tx
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = None;
        log_session_event(&serial_state, "INFO", "serial monitoring stopped by operator");
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

#[cfg(test)]
mod tests {
    use super::{release_monitoring, reserve_monitoring};
    use crate::state::SerialState;

    #[test]
    fn monitoring_reservation_is_atomic() {
        let state = SerialState::default();

        let token = reserve_monitoring(&state).expect("first start should reserve monitoring");

        assert!(!token.is_cancelled());
        assert!(reserve_monitoring(&state).is_err());
    }

    #[test]
    fn old_task_cleanup_does_not_clear_a_new_reservation() {
        let state = SerialState::default();
        let old_token = reserve_monitoring(&state).expect("old task should reserve monitoring");

        {
            let mut guard = state
                .cancellation_token
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            guard.take().expect("old reservation should exist").cancel();
        }

        let new_token = reserve_monitoring(&state).expect("new task should reserve monitoring");
        release_monitoring(&state, &old_token);

        let guard = state
            .cancellation_token
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        assert!(guard.is_some());
        assert!(!new_token.is_cancelled());
    }

    #[test]
    fn current_task_cleanup_releases_its_reservation() {
        let state = SerialState::default();
        let token = reserve_monitoring(&state).expect("task should reserve monitoring");

        release_monitoring(&state, &token);

        let guard = state
            .cancellation_token
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        assert!(guard.is_none());
    }
}
