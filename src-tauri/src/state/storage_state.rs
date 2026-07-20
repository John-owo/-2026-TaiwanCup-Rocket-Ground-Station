use crate::infrastructures::flight::{FlightRecorder, FlightSessionMetadata, FlightStats};
use crate::models::response::{StoragePhase, StorageStatus, TelemetryPayload};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, oneshot, RwLock};

pub const STORAGE_QUEUE_CAPACITY: usize = 4096;
pub const STORAGE_WARNING_BYTES: u64 = 512 * 1024 * 1024;
pub const STORAGE_HARD_LIMIT_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunOutcome {
    Completed,
    Interrupted,
}

impl RunOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Interrupted => "interrupted",
        }
    }
}

struct ActiveRun {
    test_run_id: String,
    recorder: FlightRecorder,
    file_available: bool,
    database_available: bool,
}

pub(crate) enum StorageCommand {
    BeginRun {
        test_run_id: String,
        metadata: FlightSessionMetadata,
        response: oneshot::Sender<Result<String, String>>,
    },
    Telemetry {
        payload: TelemetryPayload,
        stats: FlightStats,
        received_unix_ms: u64,
    },
    Event {
        level: String,
        message: String,
        created_unix_ms: u64,
    },
    FinishRun {
        outcome: RunOutcome,
        stats: FlightStats,
        detail: Option<String>,
        response: oneshot::Sender<Result<Option<String>, String>>,
    },
}

struct StorageInner {
    app_dir: PathBuf,
    status: Mutex<StorageStatus>,
    pool: RwLock<Option<SqlitePool>>,
    sender: mpsc::Sender<StorageCommand>,
    queue_depth: AtomicUsize,
    dropped_writes: AtomicU64,
    force_interrupt_current: AtomicBool,
}

#[derive(Clone)]
pub struct StorageState {
    inner: Arc<StorageInner>,
}

impl StorageState {
    pub(crate) fn new(app_dir: PathBuf) -> (Self, mpsc::Receiver<StorageCommand>) {
        let (sender, receiver) = mpsc::channel(STORAGE_QUEUE_CAPACITY);
        let status = StorageStatus {
            phase: StoragePhase::Initializing,
            data_path: app_dir.display().to_string(),
            available_bytes: available_space(&app_dir).ok(),
            queue_depth: 0,
            queue_capacity: STORAGE_QUEUE_CAPACITY,
            last_write_unix_ms: None,
            last_error: None,
            dropped_writes: 0,
        };
        (
            Self {
                inner: Arc::new(StorageInner {
                    app_dir,
                    status: Mutex::new(status),
                    pool: RwLock::new(None),
                    sender,
                    queue_depth: AtomicUsize::new(0),
                    dropped_writes: AtomicU64::new(0),
                    force_interrupt_current: AtomicBool::new(false),
                }),
            },
            receiver,
        )
    }

    pub fn app_dir(&self) -> &Path {
        &self.inner.app_dir
    }

    pub fn status(&self) -> StorageStatus {
        let mut status = self
            .inner
            .status
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        status.queue_depth = self.inner.queue_depth.load(Ordering::Relaxed);
        status.dropped_writes = self.inner.dropped_writes.load(Ordering::Relaxed);
        status
    }

    pub fn force_interrupt_current(&self) {
        self.inner
            .force_interrupt_current
            .store(true, Ordering::SeqCst);
    }

    pub async fn pool(&self) -> Option<SqlitePool> {
        self.inner.pool.read().await.clone()
    }

    pub fn check_recording_start(&self, app_handle: &AppHandle) -> Result<(), String> {
        let free = available_space(&self.inner.app_dir).map_err(|error| {
            let detail = format!("failed to check available disk space: {error}");
            self.set_status(
                app_handle,
                StoragePhase::Failed,
                Some(detail.clone()),
                None,
                false,
            );
            detail
        })?;
        if free < STORAGE_HARD_LIMIT_BYTES {
            let detail = format!(
                "available disk space is below the {} MiB hard limit",
                STORAGE_HARD_LIMIT_BYTES / 1024 / 1024
            );
            self.set_status(
                app_handle,
                StoragePhase::Failed,
                Some(detail.clone()),
                Some(free),
                false,
            );
            return Err(detail);
        }
        let current = self.status();
        if !matches!(current.phase, StoragePhase::Healthy | StoragePhase::Degraded) {
            return Err(current
                .last_error
                .unwrap_or_else(|| "persistent storage is not ready".to_string()));
        }
        if free < STORAGE_WARNING_BYTES {
            self.set_status(
                app_handle,
                StoragePhase::Degraded,
                Some("available disk space is below the warning threshold".to_string()),
                Some(free),
                false,
            );
        }
        Ok(())
    }

    pub(crate) async fn initialize(
        &self,
        app_handle: AppHandle,
        receiver: mpsc::Receiver<StorageCommand>,
    ) {
        let pool_result = initialize_database(&self.inner.app_dir).await;
        match pool_result {
            Ok(pool) => {
                *self.inner.pool.write().await = Some(pool.clone());
                let free = available_space(&self.inner.app_dir).ok();
                let (phase, error) = disk_phase(free, StoragePhase::Healthy, None);
                self.set_status(
                    &app_handle,
                    phase,
                    error,
                    free,
                    false,
                );
                self.writer_loop(app_handle, receiver, Some(pool)).await;
            }
            Err(error) => {
                self.set_status(
                    &app_handle,
                    StoragePhase::Failed,
                    Some(error),
                    available_space(&self.inner.app_dir).ok(),
                    false,
                );
                self.writer_loop(app_handle, receiver, None).await;
            }
        }
    }

    pub async fn begin_run(
        &self,
        test_run_id: String,
        metadata: FlightSessionMetadata,
    ) -> Result<String, String> {
        metadata.validate()?;
        let status = self.status();
        if status.phase != StoragePhase::Healthy && status.phase != StoragePhase::Degraded {
            return Err(status
                .last_error
                .unwrap_or_else(|| "persistent storage is not ready".to_string()));
        }
        if status
            .available_bytes
            .is_some_and(|bytes| bytes < STORAGE_HARD_LIMIT_BYTES)
        {
            return Err(format!(
                "available disk space is below the {} MiB hard limit",
                STORAGE_HARD_LIMIT_BYTES / 1024 / 1024
            ));
        }
        let (response_tx, response_rx) = oneshot::channel();
        self.send_command(StorageCommand::BeginRun {
            test_run_id,
            metadata,
            response: response_tx,
        })
        .await?;
        response_rx
            .await
            .map_err(|_| "storage writer closed before starting the run".to_string())?
    }

    pub fn enqueue_telemetry(
        &self,
        app_handle: &AppHandle,
        payload: TelemetryPayload,
        stats: FlightStats,
    ) -> Result<(), String> {
        self.try_send(
            app_handle,
            StorageCommand::Telemetry {
                payload,
                stats,
                received_unix_ms: unix_ms(),
            },
        )
    }

    pub fn enqueue_event(
        &self,
        app_handle: &AppHandle,
        level: impl Into<String>,
        message: impl Into<String>,
    ) -> Result<(), String> {
        self.try_send(
            app_handle,
            StorageCommand::Event {
                level: level.into(),
                message: message.into(),
                created_unix_ms: unix_ms(),
            },
        )
    }

    pub async fn finish_run(
        &self,
        outcome: RunOutcome,
        stats: FlightStats,
        detail: Option<String>,
    ) -> Result<Option<String>, String> {
        let (response_tx, response_rx) = oneshot::channel();
        self.send_command(StorageCommand::FinishRun {
            outcome,
            stats,
            detail,
            response: response_tx,
        })
        .await?;
        response_rx
            .await
            .map_err(|_| "storage writer closed before finishing the run".to_string())?
    }

    async fn send_command(&self, command: StorageCommand) -> Result<(), String> {
        self.inner.queue_depth.fetch_add(1, Ordering::Relaxed);
        if self.inner.sender.send(command).await.is_err() {
            self.inner.queue_depth.fetch_sub(1, Ordering::Relaxed);
            return Err("storage writer queue is closed".to_string());
        }
        Ok(())
    }

    fn try_send(&self, app_handle: &AppHandle, command: StorageCommand) -> Result<(), String> {
        self.inner.queue_depth.fetch_add(1, Ordering::Relaxed);
        match self.inner.sender.try_send(command) {
            Ok(()) => Ok(()),
            Err(error) => {
                self.inner.queue_depth.fetch_sub(1, Ordering::Relaxed);
                self.inner.dropped_writes.fetch_add(1, Ordering::Relaxed);
                let detail = match error {
                    mpsc::error::TrySendError::Full(_) => {
                        "storage writer queue is full; one record was not persisted".to_string()
                    }
                    mpsc::error::TrySendError::Closed(_) => {
                        "storage writer queue is closed; one record was not persisted".to_string()
                    }
                };
                self.set_status(
                    app_handle,
                    StoragePhase::Failed,
                    Some(detail.clone()),
                    available_space(&self.inner.app_dir).ok(),
                    false,
                );
                Err(detail)
            }
        }
    }

    async fn writer_loop(
        &self,
        app_handle: AppHandle,
        mut receiver: mpsc::Receiver<StorageCommand>,
        pool: Option<SqlitePool>,
    ) {
        let mut active_run: Option<ActiveRun> = None;
        let mut disk_tick = tokio::time::interval(Duration::from_secs(30));
        loop {
            tokio::select! {
                _ = disk_tick.tick() => {
                    let free = available_space(&self.inner.app_dir).ok();
                    let current = self.status();
                    let (phase, error) = disk_phase(free, current.phase, current.last_error);
                    self.set_status(&app_handle, phase, error, free, false);
                }
                command = receiver.recv() => {
                    let Some(command) = command else { break; };
                    self.inner.queue_depth.fetch_sub(1, Ordering::Relaxed);
                    match command {
                        StorageCommand::BeginRun { test_run_id, metadata, response } => {
                            let result = self.handle_begin_run(
                                &app_handle,
                                pool.as_ref(),
                                &mut active_run,
                                test_run_id,
                                metadata,
                            ).await;
                            let _ = response.send(result);
                        }
                        StorageCommand::Telemetry { payload, stats, received_unix_ms } => {
                            self.handle_telemetry(
                                &app_handle,
                                pool.as_ref(),
                                active_run.as_mut(),
                                &payload,
                                &stats,
                                received_unix_ms,
                            ).await;
                        }
                        StorageCommand::Event { level, message, created_unix_ms } => {
                            self.handle_event(
                                &app_handle,
                                pool.as_ref(),
                                active_run.as_mut(),
                                &level,
                                &message,
                                created_unix_ms,
                            ).await;
                        }
                        StorageCommand::FinishRun { outcome, stats, detail, response } => {
                            let result = self.handle_finish_run(
                                &app_handle,
                                pool.as_ref(),
                                &mut active_run,
                                outcome,
                                &stats,
                                detail.as_deref(),
                            ).await;
                            let _ = response.send(result);
                        }
                    }
                }
            }
        }
    }

    async fn handle_begin_run(
        &self,
        app_handle: &AppHandle,
        pool: Option<&SqlitePool>,
        active_run: &mut Option<ActiveRun>,
        test_run_id: String,
        metadata: FlightSessionMetadata,
    ) -> Result<String, String> {
        if active_run.is_some() {
            return Err("a storage run is already active".to_string());
        }
        let pool = pool.ok_or_else(|| "SQLite is unavailable".to_string())?;
        let recorder = FlightRecorder::start(
            &self.inner.app_dir,
            test_run_id.clone(),
            metadata.clone(),
        )?;
        let directory = recorder.directory().display().to_string();
        let insert = sqlx::query(
            "INSERT INTO test_runs (
                test_run_id, started_unix_ms, purpose, operator, location,
                initial_battery_voltage, notes, status, directory
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'recording', ?8)",
        )
        .bind(&test_run_id)
        .bind(unix_ms() as i64)
        .bind(metadata.purpose.trim())
        .bind(metadata.operator.trim())
        .bind(metadata.location.trim())
        .bind(metadata.initial_battery_voltage)
        .bind(metadata.notes.trim())
        .bind(&directory)
        .execute(pool)
        .await;
        if let Err(error) = insert {
            let cleanup = recorder.cleanup_failed_start().err();
            let detail = format!(
                "failed to create test run in SQLite: {error}{}",
                cleanup
                    .map(|value| format!("; failed to remove empty run directory: {value}"))
                    .unwrap_or_default()
            );
            self.set_status(
                app_handle,
                StoragePhase::Failed,
                Some(detail.clone()),
                available_space(&self.inner.app_dir).ok(),
                true,
            );
            return Err(detail);
        }
        *active_run = Some(ActiveRun {
            test_run_id,
            recorder,
            file_available: true,
            database_available: true,
        });
        self.set_status(
            app_handle,
            StoragePhase::Healthy,
            None,
            available_space(&self.inner.app_dir).ok(),
            true,
        );
        Ok(directory)
    }

    async fn handle_telemetry(
        &self,
        app_handle: &AppHandle,
        pool: Option<&SqlitePool>,
        active_run: Option<&mut ActiveRun>,
        payload: &TelemetryPayload,
        stats: &FlightStats,
        received_unix_ms: u64,
    ) {
        let Some(run) = active_run else { return; };
        let file_result = run.recorder.record_telemetry(payload, stats);
        if file_result.is_err() {
            run.file_available = false;
        }

        let db_result = if let Some(pool) = pool {
            insert_telemetry(pool, &run.test_run_id, payload, stats, received_unix_ms).await
        } else {
            Err("SQLite is unavailable".to_string())
        };
        if db_result.is_err() {
            run.database_available = false;
        }
        self.update_sink_status(
            app_handle,
            run,
            file_result.err().or_else(|| db_result.err()),
        );
    }

    async fn handle_event(
        &self,
        app_handle: &AppHandle,
        pool: Option<&SqlitePool>,
        active_run: Option<&mut ActiveRun>,
        level: &str,
        message: &str,
        created_unix_ms: u64,
    ) {
        let Some(run) = active_run else { return; };
        let file_result = run.recorder.log_event(level, message);
        if file_result.is_err() {
            run.file_available = false;
        }
        let db_result = if let Some(pool) = pool {
            sqlx::query(
                "INSERT INTO test_run_events
                 (test_run_id, created_unix_ms, level, message)
                 VALUES (?1, ?2, ?3, ?4)",
            )
            .bind(&run.test_run_id)
            .bind(created_unix_ms as i64)
            .bind(level)
            .bind(message)
            .execute(pool)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
        } else {
            Err("SQLite is unavailable".to_string())
        };
        if db_result.is_err() {
            run.database_available = false;
        }
        self.update_sink_status(
            app_handle,
            run,
            file_result.err().or_else(|| db_result.err()),
        );
    }

    async fn handle_finish_run(
        &self,
        app_handle: &AppHandle,
        pool: Option<&SqlitePool>,
        active_run: &mut Option<ActiveRun>,
        outcome: RunOutcome,
        stats: &FlightStats,
        detail: Option<&str>,
    ) -> Result<Option<String>, String> {
        let Some(mut run) = active_run.take() else { return Ok(None); };
        let forced_interruption = self
            .inner
            .force_interrupt_current
            .swap(false, Ordering::SeqCst);
        let outcome = if forced_interruption {
            RunOutcome::Interrupted
        } else {
            outcome
        };
        let detail = if forced_interruption {
            Some("stop timed out after 5 seconds while draining queued data")
        } else {
            detail
        };
        let directory = run.recorder.directory().display().to_string();
        let file_result = run.recorder.finish(stats, outcome.as_str(), detail);
        if file_result.is_err() {
            run.file_available = false;
        }
        let db_result = if let Some(pool) = pool {
            sqlx::query(
                "UPDATE test_runs
                 SET ended_unix_ms = ?1, status = ?2, error_detail = ?3
                 WHERE test_run_id = ?4",
            )
            .bind(unix_ms() as i64)
            .bind(outcome.as_str())
            .bind(detail)
            .bind(&run.test_run_id)
            .execute(pool)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
        } else {
            Err("SQLite is unavailable".to_string())
        };
        if db_result.is_err() {
            run.database_available = false;
        }
        let error = file_result.err().or_else(|| db_result.err());
        self.update_sink_status(app_handle, &run, error.clone());
        if !run.file_available && !run.database_available {
            Err(error.unwrap_or_else(|| "both persistent storage sinks failed".to_string()))
        } else {
            Ok(Some(directory))
        }
    }

    fn update_sink_status(
        &self,
        app_handle: &AppHandle,
        run: &ActiveRun,
        error: Option<String>,
    ) {
        let phase = if self.inner.dropped_writes.load(Ordering::Relaxed) > 0 {
            StoragePhase::Failed
        } else { match (run.file_available, run.database_available) {
            (true, true) => StoragePhase::Healthy,
            (false, false) => StoragePhase::Failed,
            _ => StoragePhase::Degraded,
        }};
        let error = if phase == StoragePhase::Healthy {
            None
        } else {
            error.or_else(|| self.status().last_error)
        };
        self.set_status(
            app_handle,
            phase,
            error,
            available_space(&self.inner.app_dir).ok(),
            run.file_available || run.database_available,
        );
    }

    fn set_status(
        &self,
        app_handle: &AppHandle,
        phase: StoragePhase,
        error: Option<String>,
        available_bytes: Option<u64>,
        write_succeeded: bool,
    ) {
        let mut status = self
            .inner
            .status
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        status.phase = phase;
        status.available_bytes = available_bytes;
        status.queue_depth = self.inner.queue_depth.load(Ordering::Relaxed);
        status.dropped_writes = self.inner.dropped_writes.load(Ordering::Relaxed);
        status.last_error = error;
        if write_succeeded {
            status.last_write_unix_ms = Some(unix_ms());
        }
        let snapshot = status.clone();
        drop(status);
        let _ = app_handle.emit("storage-status", &snapshot);
    }
}

fn disk_phase(
    free: Option<u64>,
    current_phase: StoragePhase,
    current_error: Option<String>,
) -> (StoragePhase, Option<String>) {
    if free.is_some_and(|bytes| bytes < STORAGE_HARD_LIMIT_BYTES) {
        return (
            StoragePhase::Failed,
            Some("available disk space is below the hard limit".to_string()),
        );
    }
    if free.is_some_and(|bytes| bytes < STORAGE_WARNING_BYTES) {
        return (
            if current_phase == StoragePhase::Failed {
                StoragePhase::Failed
            } else {
                StoragePhase::Degraded
            },
            Some("available disk space is below the warning threshold".to_string()),
        );
    }
    let disk_error = current_error
        .as_deref()
        .is_some_and(|error| error.contains("disk space"));
    if disk_error {
        (StoragePhase::Healthy, None)
    } else {
        (current_phase, current_error)
    }
}

async fn initialize_database(app_dir: &Path) -> Result<SqlitePool, String> {
    std::fs::create_dir_all(app_dir)
        .map_err(|error| format!("failed to create app data directory: {error}"))?;
    let db_path = app_dir.join("telemetry.db");
    let options = SqliteConnectOptions::new()
        .filename(&db_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(2))
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await
        .map_err(|error| format!("failed to connect to SQLite: {error}"))?;
    migrate_database(&pool).await?;
    sqlx::query(
        "UPDATE test_runs
         SET status = 'interrupted', ended_unix_ms = ?1,
             error_detail = 'application restarted before the run was finalized'
         WHERE status = 'recording'",
    )
    .bind(unix_ms() as i64)
    .execute(&pool)
    .await
    .map_err(|error| format!("failed to recover unfinished runs: {error}"))?;
    Ok(pool)
}

async fn migrate_database(pool: &SqlitePool) -> Result<(), String> {
    let mut transaction = pool
        .begin()
        .await
        .map_err(|error| format!("failed to begin database migration: {error}"))?;
    let previous_version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&mut *transaction)
        .await
        .map_err(|error| error.to_string())?;
    let telemetry_exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'telemetry'",
    )
    .fetch_one(&mut *transaction)
    .await
    .map_err(|error| error.to_string())?;
    let mut migrate_legacy = false;
    if telemetry_exists > 0 {
        let columns = sqlx::query("PRAGMA table_info(telemetry)")
            .fetch_all(&mut *transaction)
            .await
            .map_err(|error| error.to_string())?;
        migrate_legacy = !columns.iter().any(|row| {
            row.try_get::<String, _>("name")
                .is_ok_and(|name| name == "test_run_id")
        });
        if migrate_legacy {
            let legacy_exists: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'table' AND name = 'telemetry_legacy'",
            )
            .fetch_one(&mut *transaction)
            .await
            .map_err(|error| error.to_string())?;
            if legacy_exists > 0 {
                return Err("telemetry and telemetry_legacy both contain legacy schemas".to_string());
            }
            sqlx::query("ALTER TABLE telemetry RENAME TO telemetry_legacy")
                .execute(&mut *transaction)
                .await
                .map_err(|error| format!("failed to preserve legacy telemetry: {error}"))?;
        }
    }
    transaction
        .commit()
        .await
        .map_err(|error| format!("failed to preserve legacy database: {error}"))?;

    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|error| format!("SQLx database migration failed: {error}"))?;

    let legacy_exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master
         WHERE type = 'table' AND name = 'telemetry_legacy'",
    )
    .fetch_one(pool)
    .await
    .map_err(|error| error.to_string())?;
    if legacy_exists > 0 && (migrate_legacy || previous_version < 1) {
        let mut copy = pool
            .begin()
            .await
            .map_err(|error| format!("failed to begin legacy telemetry copy: {error}"))?;
        sqlx::query(
            "INSERT INTO telemetry (
                test_run_id, received_at, x_acceleration, y_acceleration, z_acceleration,
                x_angular_velocity, y_angular_velocity, z_angular_velocity,
                longitude, latitude, altitude, ground_speed, vertical_velocity,
                air_pressure, temperature
             )
             SELECT NULL, received_at, x_acceleration, y_acceleration, z_acceleration,
                x_angular_velocity, y_angular_velocity, z_angular_velocity,
                longitude, latitude, altitude, ground_speed, vertical_velocity,
                air_pressure, temperature
             FROM telemetry_legacy",
        )
        .execute(&mut *copy)
        .await
        .map_err(|error| format!("failed to copy legacy telemetry: {error}"))?;
        sqlx::query("PRAGMA user_version = 1")
            .execute(&mut *copy)
            .await
            .map_err(|error| error.to_string())?;
        copy.commit()
            .await
            .map_err(|error| format!("failed to commit legacy telemetry copy: {error}"))?;
    } else if previous_version < 1 {
        sqlx::query("PRAGMA user_version = 1")
            .execute(pool)
            .await
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

async fn insert_telemetry(
    pool: &SqlitePool,
    test_run_id: &str,
    payload: &TelemetryPayload,
    stats: &FlightStats,
    received_unix_ms: u64,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO telemetry (
            test_run_id, received_at, received_unix_ms, protocol_version,
            airborne_session_id, frame_seq, uptime_ms, restart_reason,
            timer_state, deploy_state, sensor_flags, remaining_s,
            last_ack_command_id, last_ack_result,
            x_acceleration, y_acceleration, z_acceleration,
            x_angular_velocity, y_angular_velocity, z_angular_velocity,
            longitude, latitude, altitude, ground_speed, vertical_velocity,
            air_pressure, temperature, lost_packets, duplicate_packets,
            crc_errors, restart_count
         ) VALUES (
            ?1, datetime('now'), ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
            ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19,
            ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30
         )",
    )
    .bind(test_run_id)
    .bind(received_unix_ms as i64)
    .bind(payload.protocol_version)
    .bind(payload.session_id)
    .bind(payload.frame_seq)
    .bind(payload.uptime_ms)
    .bind(payload.restart_reason)
    .bind(payload.timer_state)
    .bind(payload.deploy_state)
    .bind(payload.sensor_flags)
    .bind(payload.remaining_s)
    .bind(payload.last_ack_command_id)
    .bind(payload.last_ack_result)
    .bind(payload.x_acceleration)
    .bind(payload.y_acceleration)
    .bind(payload.z_acceleration)
    .bind(payload.x_angular_velocity)
    .bind(payload.y_angular_velocity)
    .bind(payload.z_angular_velocity)
    .bind(payload.longitude)
    .bind(payload.latitude)
    .bind(payload.altitude)
    .bind(payload.ground_speed)
    .bind(payload.vertical_velocity)
    .bind(payload.air_pressure)
    .bind(payload.temperature)
    .bind(stats.lost_packets as i64)
    .bind(stats.duplicate_packets as i64)
    .bind(stats.crc_errors as i64)
    .bind(stats.restart_count as i64)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|error| error.to_string())
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(windows)]
fn available_space(path: &Path) -> Result<u64, String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    let mut wide = path.as_os_str().encode_wide().collect::<Vec<_>>();
    wide.push(0);
    let mut available = 0_u64;
    let result = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut available,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error().to_string())
    } else {
        Ok(available)
    }
}

#[cfg(not(windows))]
fn available_space(_path: &Path) -> Result<u64, String> {
    Ok(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_limits_are_ordered_and_queue_is_bounded() {
        assert!(STORAGE_WARNING_BYTES > STORAGE_HARD_LIMIT_BYTES);
        assert_eq!(STORAGE_QUEUE_CAPACITY, 4096);
    }

    #[test]
    fn disk_status_recovers_only_disk_related_failures() {
        assert_eq!(
            disk_phase(
                Some(STORAGE_WARNING_BYTES + 1),
                StoragePhase::Failed,
                Some("available disk space is below the hard limit".to_string()),
            ),
            (StoragePhase::Healthy, None),
        );
        assert_eq!(
            disk_phase(
                Some(STORAGE_WARNING_BYTES + 1),
                StoragePhase::Failed,
                Some("SQLite write failed".to_string()),
            )
            .0,
            StoragePhase::Failed,
        );
    }

    #[test]
    fn stop_timeout_latches_an_interrupted_finish() {
        let (state, _receiver) = StorageState::new(std::env::temp_dir());
        state.force_interrupt_current();
        assert!(state
            .inner
            .force_interrupt_current
            .swap(false, Ordering::SeqCst));
    }

    #[tokio::test]
    async fn migration_preserves_legacy_rows_and_is_idempotent() {
        let root = std::env::temp_dir().join(format!(
            "ground-station-migration-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).expect("temporary migration directory");
        let db_path = root.join("telemetry.db");
        let options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("legacy database");
        sqlx::query(
            "CREATE TABLE telemetry (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                received_at TEXT NOT NULL DEFAULT (datetime('now')),
                x_acceleration REAL NOT NULL, y_acceleration REAL NOT NULL,
                z_acceleration REAL NOT NULL, x_angular_velocity REAL NOT NULL,
                y_angular_velocity REAL NOT NULL, z_angular_velocity REAL NOT NULL,
                longitude REAL NOT NULL, latitude REAL NOT NULL, altitude REAL NOT NULL,
                ground_speed REAL NOT NULL, vertical_velocity REAL NOT NULL,
                air_pressure REAL NOT NULL, temperature REAL NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .expect("legacy schema");
        sqlx::query(
            "INSERT INTO telemetry (
                x_acceleration, y_acceleration, z_acceleration,
                x_angular_velocity, y_angular_velocity, z_angular_velocity,
                longitude, latitude, altitude, ground_speed, vertical_velocity,
                air_pressure, temperature
             ) VALUES (1,2,3,4,5,6,121,24,100,7,8,1013,25)",
        )
        .execute(&pool)
        .await
        .expect("legacy row");

        migrate_database(&pool).await.expect("first migration");
        migrate_database(&pool).await.expect("repeat migration");

        let legacy_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM telemetry_legacy")
            .fetch_one(&pool)
            .await
            .expect("legacy count");
        let migrated_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM telemetry WHERE test_run_id IS NULL",
        )
        .fetch_one(&pool)
        .await
        .expect("migrated count");
        let migration_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
            .fetch_one(&pool)
            .await
            .expect("SQLx migration count");
        assert_eq!(legacy_count, 1);
        assert_eq!(migrated_count, 1);
        assert_eq!(migration_count, 1);

        pool.close().await;
        std::fs::remove_dir_all(root).expect("remove temporary migration directory");
    }

    #[tokio::test]
    async fn migration_accepts_released_v0_1_1_checksum() {
        const RELEASED_CHECKSUM: &[u8] = &[
            0x01, 0xC4, 0xAA, 0x5F, 0xA0, 0x6F, 0xE6, 0x55, 0xD2, 0xC6, 0x83, 0xBE,
            0xE6, 0x27, 0x1F, 0x2C, 0x95, 0x08, 0x98, 0xFF, 0xB9, 0x55, 0xD1, 0x55,
            0x3F, 0x98, 0xAE, 0x95, 0xD0, 0x01, 0x77, 0x8E, 0xB4, 0x6C, 0xC8, 0x51,
            0x83, 0x2E, 0x42, 0x1F, 0x7F, 0xD1, 0x8D, 0x5E, 0x87, 0x31, 0x61, 0x91,
        ];
        let root = std::env::temp_dir().join(format!(
            "ground-station-released-migration-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).expect("temporary migration directory");
        let options = SqliteConnectOptions::new()
            .filename(root.join("telemetry.db"))
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("released database");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("create current schema");
        sqlx::query("UPDATE _sqlx_migrations SET checksum = ?1 WHERE version = 1")
            .bind(RELEASED_CHECKSUM)
            .execute(&pool)
            .await
            .expect("record released migration checksum");

        migrate_database(&pool)
            .await
            .expect("released database remains compatible");
        let migration_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
            .fetch_one(&pool)
            .await
            .expect("migration count");
        assert_eq!(migration_count, 1);

        pool.close().await;
        std::fs::remove_dir_all(root).expect("remove temporary migration directory");
    }

    #[tokio::test]
    #[ignore = "requires GS_MIGRATION_FIXTURE_DB pointing to a SQLite backup"]
    async fn migration_preserves_external_database_fixture() {
        let source = std::path::PathBuf::from(
            std::env::var("GS_MIGRATION_FIXTURE_DB")
                .expect("GS_MIGRATION_FIXTURE_DB must point to a SQLite backup"),
        );
        let root = std::env::temp_dir().join(format!(
            "ground-station-external-migration-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).expect("temporary migration directory");
        let db_path = root.join("telemetry.db");
        std::fs::copy(&source, &db_path).expect("copy migration fixture");

        let options = SqliteConnectOptions::new().filename(&db_path);
        let before_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("open migration fixture");
        let telemetry_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM telemetry")
            .fetch_one(&before_pool)
            .await
            .expect("telemetry count before migration");
        let runs_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_runs")
            .fetch_one(&before_pool)
            .await
            .expect("test run count before migration");
        before_pool.close().await;

        let pool = initialize_database(&root)
            .await
            .expect("external database fixture remains compatible");
        let telemetry_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM telemetry")
            .fetch_one(&pool)
            .await
            .expect("telemetry count after migration");
        let runs_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_runs")
            .fetch_one(&pool)
            .await
            .expect("test run count after migration");
        assert_eq!(telemetry_after, telemetry_before);
        assert_eq!(runs_after, runs_before);
        println!(
            "preserved {telemetry_after} telemetry rows and {runs_after} test runs"
        );

        pool.close().await;
        drop(pool);
        let _ = std::fs::remove_dir_all(root);
    }
}
