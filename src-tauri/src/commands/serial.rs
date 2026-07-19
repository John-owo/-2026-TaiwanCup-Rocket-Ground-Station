use crate::infrastructures::flight::{FlightSessionMetadata, FlightStats};
use crate::infrastructures::serial::command::CommandRequest;
use crate::infrastructures::serial::receiver::SerialReceiver;
use crate::models::response::{
    InvokeError, InvokeResult, StoragePhase, StorageStatus, TestRunPhase, TestSessionStatus,
};
use crate::services::serial::Receiver;
use crate::state::{RunOutcome, SerialState, StorageState};

use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
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
        *serial_state
            .command_tx
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = None;
    }
}

fn session_status(serial_state: &SerialState) -> TestSessionStatus {
    serial_state
        .test_session_status
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}

fn set_session_status(
    serial_state: &SerialState,
    app_handle: &AppHandle,
    status: TestSessionStatus,
) {
    *serial_state
        .test_session_status
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = status.clone();
    let _ = app_handle.emit("test-session-status", &status);
}

fn terminal_phase(phase: &TestRunPhase) -> bool {
    matches!(
        phase,
        TestRunPhase::Completed
            | TestRunPhase::Interrupted
            | TestRunPhase::Failed
            | TestRunPhase::Disconnected
    )
}

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

#[tauri::command]
pub async fn start_test_monitoring(
    path: String,
    baud_rate: u32,
    metadata: FlightSessionMetadata,
    allow_unrecorded: bool,
    serial_state: State<'_, SerialState>,
    storage_state: State<'_, StorageState>,
    app_handle: AppHandle,
) -> InvokeResult<TestSessionStatus> {
    metadata
        .validate()
        .map_err(|_| InvokeError::ValidationFailed)?;
    if path.trim().is_empty() || baud_rate == 0 {
        return Err(InvokeError::ValidationFailed);
    }
    let storage_readiness = storage_state.check_recording_start(&app_handle);
    if allow_unrecorded {
        if storage_readiness.is_ok() || storage_state.status().phase != StoragePhase::Failed {
            return Err(InvokeError::ValidationFailed);
        }
    } else {
        storage_readiness.map_err(InvokeError::DatabaseError)?;
    }
    let cancellation_token = reserve_monitoring(&serial_state)?;
    serial_state
        .manual_stop_requested
        .store(false, Ordering::SeqCst);
    let starting = TestSessionStatus {
        phase: TestRunPhase::Starting,
        test_run_id: None,
        directory: None,
        purpose: Some(metadata.purpose.trim().to_string()),
        detail: None,
    };
    set_session_status(&serial_state, &app_handle, starting);

    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let mut receiver = SerialReceiver::new(
        app_handle.clone(),
        cancellation_token.clone(),
        command_rx,
    );
    if let Err(error) = receiver.get_connection(path.clone(), baud_rate).await {
        release_monitoring(&serial_state, &cancellation_token);
        set_session_status(
            &serial_state,
            &app_handle,
            TestSessionStatus {
                phase: TestRunPhase::Failed,
                test_run_id: None,
                directory: None,
                purpose: Some(metadata.purpose.trim().to_string()),
                detail: Some(error.clone()),
            },
        );
        return Err(InvokeError::SerialError(error));
    }

    let (test_run_id, directory, phase) = if allow_unrecorded {
        (None, None, TestRunPhase::MonitoringUnrecorded)
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        match storage_state.begin_run(id.clone(), metadata.clone()).await {
            Ok(directory) => (Some(id), Some(directory), TestRunPhase::Recording),
            Err(error) => {
                release_monitoring(&serial_state, &cancellation_token);
                set_session_status(
                    &serial_state,
                    &app_handle,
                    TestSessionStatus {
                        phase: TestRunPhase::Failed,
                        test_run_id: None,
                        directory: None,
                        purpose: Some(metadata.purpose.trim().to_string()),
                        detail: Some(error.clone()),
                    },
                );
                return Err(InvokeError::DatabaseError(error));
            }
        }
    };

    {
        *serial_state.path.lock().unwrap_or_else(|e| e.into_inner()) = Some(path.clone());
        *serial_state
            .baud_rate
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(baud_rate);
        *serial_state
            .command_tx
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(command_tx);
        serial_state
            .flight_stats
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .reset();
    }
    let active_status = TestSessionStatus {
        phase,
        test_run_id,
        directory,
        purpose: Some(metadata.purpose.trim().to_string()),
        detail: if allow_unrecorded {
            Some("operator acknowledged monitoring without persistent recording".to_string())
        } else {
            None
        },
    };
    set_session_status(&serial_state, &app_handle, active_status.clone());
    if !allow_unrecorded {
        let _ = storage_state.enqueue_event(
            &app_handle,
            "INFO",
            format!("serial connected: {path} @ {baud_rate}"),
        );
    }

    let handle_for_cleanup = app_handle.clone();
    let token_for_cleanup = cancellation_token.clone();
    tokio::spawn(async move {
        let receive_result = receiver.start_receive().await;
        let Some(state) = handle_for_cleanup.try_state::<SerialState>() else {
            return;
        };
        let manual = state.manual_stop_requested.load(Ordering::SeqCst);
        let outcome = if manual {
            RunOutcome::Completed
        } else {
            RunOutcome::Interrupted
        };
        let detail = match &receive_result {
            Ok(message) if manual => Some(format!("operator stopped monitoring: {message}")),
            Ok(message) => Some(format!("receiver ended unexpectedly: {message}")),
            Err(error) => Some(error.clone()),
        };
        let stats = state
            .flight_stats
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .snapshot();
        let current = session_status(state.inner());
        let directory = if current.phase == TestRunPhase::Recording
            || current.phase == TestRunPhase::Finishing
        {
            if let Some(storage) = handle_for_cleanup.try_state::<StorageState>() {
                storage
                    .finish_run(outcome, stats, detail.clone())
                    .await
                    .ok()
                    .flatten()
                    .or(current.directory.clone())
            } else {
                current.directory.clone()
            }
        } else {
            current.directory.clone()
        };
        let after_finish = session_status(state.inner());
        if !matches!(after_finish.phase, TestRunPhase::Interrupted | TestRunPhase::Failed) {
            let final_status = TestSessionStatus {
                phase: if manual {
                    TestRunPhase::Completed
                } else {
                    TestRunPhase::Interrupted
                },
                test_run_id: current.test_run_id,
                directory,
                purpose: current.purpose,
                detail: detail.clone(),
            };
            set_session_status(state.inner(), &handle_for_cleanup, final_status);
        }
        if !manual {
            let _ = handle_for_cleanup.emit(
                "serial-error",
                serde_json::json!({
                    "errorType": "SERIAL_ERROR",
                    "detail": detail.unwrap_or_else(|| "serial receiver stopped unexpectedly".to_string()),
                }),
            );
        }
        release_monitoring(state.inner(), &token_for_cleanup);
        state.terminal_notify.notify_waiters();
    });

    Ok(active_status)
}

fn queue_command(serial_state: &SerialState, request: CommandRequest) -> InvokeResult<()> {
    let status = session_status(serial_state);
    if !matches!(
        status.phase,
        TestRunPhase::Recording | TestRunPhase::MonitoringUnrecorded
    ) {
        return Err(InvokeError::SerialError(
            "test monitoring must be active before sending commands".to_string(),
        ));
    }
    let guard = serial_state
        .command_tx
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let sender = guard.as_ref().ok_or_else(|| {
        InvokeError::SerialError("serial command queue is closed".to_string())
    })?;
    sender
        .send(request)
        .map_err(|_| InvokeError::SerialError("serial command queue is closed".to_string()))
}

#[tauri::command]
pub async fn set_timer(
    duration_s: u32,
    serial_state: State<'_, SerialState>,
    storage_state: State<'_, StorageState>,
    app_handle: AppHandle,
) -> InvokeResult<String> {
    if duration_s == 0 {
        return Err(InvokeError::ValidationFailed);
    }
    queue_command(&serial_state, CommandRequest::SetTimer(duration_s))?;
    let _ = storage_state.enqueue_event(
        &app_handle,
        "INFO",
        format!("SET_TIMER requested: {duration_s} s"),
    );
    Ok("SET_TIMER queued".to_string())
}

#[tauri::command]
pub async fn force_release(
    serial_state: State<'_, SerialState>,
    storage_state: State<'_, StorageState>,
    app_handle: AppHandle,
) -> InvokeResult<String> {
    queue_command(&serial_state, CommandRequest::ForceRelease)?;
    let _ = storage_state.enqueue_event(
        &app_handle,
        "WARN",
        "FORCE_RELEASE requested after UI safety unlock",
    );
    Ok("FORCE_RELEASE queued".to_string())
}

#[tauri::command]
pub async fn get_flight_stats(serial_state: State<'_, SerialState>) -> InvokeResult<FlightStats> {
    Ok(serial_state
        .flight_stats
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .snapshot())
}

#[tauri::command]
pub async fn get_test_session_status(
    serial_state: State<'_, SerialState>,
) -> InvokeResult<TestSessionStatus> {
    Ok(session_status(&serial_state))
}

#[tauri::command]
pub async fn get_storage_status(
    storage_state: State<'_, StorageState>,
) -> InvokeResult<StorageStatus> {
    Ok(storage_state.status())
}

async fn wait_for_terminal(serial_state: &SerialState) -> TestSessionStatus {
    loop {
        let current = session_status(serial_state);
        if terminal_phase(&current.phase) {
            return current;
        }
        serial_state.terminal_notify.notified().await;
    }
}

#[tauri::command]
pub async fn stop_test_monitoring(
    serial_state: State<'_, SerialState>,
    storage_state: State<'_, StorageState>,
    app_handle: AppHandle,
) -> InvokeResult<TestSessionStatus> {
    let cancellation_token = serial_state
        .cancellation_token
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
        .ok_or_else(|| InvokeError::SerialError("no monitoring task running".to_string()))?;
    serial_state
        .manual_stop_requested
        .store(true, Ordering::SeqCst);
    let current = session_status(&serial_state);
    set_session_status(
        &serial_state,
        &app_handle,
        TestSessionStatus {
            phase: TestRunPhase::Finishing,
            ..current.clone()
        },
    );
    let _ = storage_state.enqueue_event(
        &app_handle,
        "INFO",
        "serial monitoring stopped by operator",
    );
    cancellation_token.cancel();
    match tokio::time::timeout(Duration::from_secs(5), wait_for_terminal(&serial_state)).await {
        Ok(status) => Ok(status),
        Err(_) => {
            serial_state
                .manual_stop_requested
                .store(false, Ordering::SeqCst);
            storage_state.force_interrupt_current();
            let status = TestSessionStatus {
                phase: TestRunPhase::Interrupted,
                detail: Some(
                    "stop timed out after 5 seconds; queued data is retained and the run is interrupted"
                        .to_string(),
                ),
                ..current
            };
            set_session_status(&serial_state, &app_handle, status.clone());
            release_monitoring(&serial_state, &cancellation_token);
            Ok(status)
        }
    }
}

pub async fn interrupt_test_monitoring(
    serial_state: &SerialState,
    storage_state: &StorageState,
    app_handle: &AppHandle,
    reason: &str,
) -> TestSessionStatus {
    let token = serial_state
        .cancellation_token
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    let current = session_status(serial_state);
    let Some(token) = token else { return current; };
    serial_state
        .manual_stop_requested
        .store(false, Ordering::SeqCst);
    token.cancel();
    if let Ok(status) = tokio::time::timeout(Duration::from_secs(5), wait_for_terminal(serial_state)).await {
        return status;
    }
    let stats = serial_state
        .flight_stats
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .snapshot();
    let _ = storage_state
        .finish_run(RunOutcome::Interrupted, stats, Some(reason.to_string()))
        .await;
    let status = TestSessionStatus {
        phase: TestRunPhase::Interrupted,
        detail: Some(reason.to_string()),
        ..current
    };
    set_session_status(serial_state, app_handle, status.clone());
    release_monitoring(serial_state, &token);
    status
}

#[tauri::command]
pub async fn get_telemetry_history(
    limit: i64,
    test_run_id: Option<String>,
    storage_state: State<'_, StorageState>,
) -> InvokeResult<Vec<crate::models::response::DbTelemetry>> {
    if !(1..=10_000).contains(&limit) {
        return Err(InvokeError::ValidationFailed);
    }
    let pool = storage_state
        .pool()
        .await
        .ok_or_else(|| InvokeError::DatabaseError("SQLite is unavailable".to_string()))?;
    let rows = sqlx::query_as::<_, crate::models::response::DbTelemetry>(
        "SELECT id, test_run_id, received_at,
                x_acceleration, y_acceleration, z_acceleration,
                x_angular_velocity, y_angular_velocity, z_angular_velocity,
                longitude, latitude, altitude,
                ground_speed, vertical_velocity, air_pressure, temperature
         FROM telemetry
         WHERE (?1 IS NULL OR test_run_id = ?1)
         ORDER BY id DESC LIMIT ?2",
    )
    .bind(test_run_id.as_deref())
    .bind(limit)
    .fetch_all(&pool)
    .await
    .map_err(|error| InvokeError::DatabaseError(error.to_string()))?;
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
            guard.take().expect("old reservation").cancel();
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
}
