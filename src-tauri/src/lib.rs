mod commands;
mod infrastructures;
mod models;
mod services;
pub mod state;

use models::response::TestRunPhase;
use state::{SerialState, StorageState};
use std::sync::atomic::Ordering;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .manage(SerialState::default())
        .setup(|app| {
            app.handle().plugin(
                tauri_plugin_log::Builder::default()
                    .level(log::LevelFilter::Info)
                    .build(),
            )?;

            let app_dir = app
                .handle()
                .path()
                .app_data_dir()
                .map_err(|error| error.to_string())?;
            let (storage_state, storage_receiver) = StorageState::new(app_dir);
            app.manage(storage_state);
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Some(storage) = app_handle.try_state::<StorageState>() {
                    storage
                        .initialize(app_handle.clone(), storage_receiver)
                        .await;
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::serial::list_serial_ports,
            commands::serial::start_test_monitoring,
            commands::serial::stop_test_monitoring,
            commands::serial::get_test_session_status,
            commands::serial::get_storage_status,
            commands::serial::set_timer,
            commands::serial::force_release,
            commands::serial::get_flight_stats,
            commands::serial::get_telemetry_history,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::ExitRequested { code, api, .. } = event {
            if code.is_some() {
                return;
            }
            let Some(serial_state) = app_handle.try_state::<SerialState>() else {
                return;
            };
            let phase = serial_state
                .test_session_status
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .phase
                .clone();
            if !matches!(
                phase,
                TestRunPhase::Starting
                    | TestRunPhase::Recording
                    | TestRunPhase::MonitoringUnrecorded
                    | TestRunPhase::Finishing
            ) {
                return;
            }
            api.prevent_exit();
            if serial_state.shutdown_started.swap(true, Ordering::SeqCst) {
                return;
            }
            let handle = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                if let (Some(serial), Some(storage)) = (
                    handle.try_state::<SerialState>(),
                    handle.try_state::<StorageState>(),
                ) {
                    commands::serial::interrupt_test_monitoring(
                        serial.inner(),
                        storage.inner(),
                        &handle,
                        "application exit requested before the run was completed",
                    )
                    .await;
                }
                handle.exit(0);
            });
        }
    });
}
