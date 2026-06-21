// ─── Module Declarations ─────────────────────────────────────────────────────
mod commands;
mod infrastructures;
mod models;
mod services;
pub mod state;

// ─── Imports ─────────────────────────────────────────────────────────────────
use state::{DbPool, SerialState};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(SerialState::default())
        .setup(|app| {
            // ── Logging Plugin ───────────────────────────────────────────
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // ── SQLite Database Setup (non-blocking) ─────────────────────
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                match init_database(&app_handle).await {
                    Ok(pool) => {
                        app_handle.manage(DbPool(pool));
                        log::info!("SQLite database initialized successfully");
                    }
                    Err(e) => {
                        log::error!("Failed to initialize database: {}", e);
                        if let Ok(pool) = sqlx::SqlitePool::connect("sqlite::memory:").await {
                            app_handle.manage(DbPool(pool));
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::serial::start_monitoring,
            commands::serial::stop_monitoring,
            commands::serial::get_telemetry_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 初始化 SQLite 資料庫並建立遙測資料表
async fn init_database(app_handle: &tauri::AppHandle) -> Result<sqlx::SqlitePool, String> {

    // 取得應用程式資料目錄
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to get app data dir: {}", e))?;

    // 確保目錄存在
    std::fs::create_dir_all(&app_dir)
        .map_err(|e| format!("failed to create app data dir: {}", e))?;

    let db_path = app_dir.join("telemetry.db");
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    log::info!("database path: {}", db_path.display());

    // 建立連線池
    let pool = sqlx::SqlitePool::connect(&db_url)
        .await
        .map_err(|e| format!("failed to connect to database: {}", e))?;

    // 建立遙測資料表（如不存在）
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS telemetry (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            received_at TEXT NOT NULL DEFAULT (datetime('now')),
            x_acceleration REAL NOT NULL,
            y_acceleration REAL NOT NULL,
            z_acceleration REAL NOT NULL,
            x_angular_velocity REAL NOT NULL,
            y_angular_velocity REAL NOT NULL,
            z_angular_velocity REAL NOT NULL,
            longitude REAL NOT NULL,
            latitude REAL NOT NULL,
            altitude REAL NOT NULL,
            ground_speed REAL NOT NULL,
            vertical_velocity REAL NOT NULL,
            air_pressure REAL NOT NULL,
            temperature REAL NOT NULL
        )"
    )
    .execute(&pool)
    .await
    .map_err(|e| format!("failed to create telemetry table: {}", e))?;

    Ok(pool)
}
