CREATE TABLE IF NOT EXISTS test_runs (
    test_run_id TEXT PRIMARY KEY,
    started_unix_ms INTEGER NOT NULL,
    ended_unix_ms INTEGER,
    purpose TEXT NOT NULL,
    operator TEXT NOT NULL,
    location TEXT NOT NULL,
    initial_battery_voltage REAL NOT NULL,
    notes TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL CHECK(status IN ('recording','completed','interrupted','failed')),
    directory TEXT NOT NULL,
    error_detail TEXT
);

CREATE TABLE IF NOT EXISTS telemetry (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    test_run_id TEXT REFERENCES test_runs(test_run_id),
    received_at TEXT NOT NULL DEFAULT (datetime('now')),
    received_unix_ms INTEGER,
    protocol_version INTEGER,
    airborne_session_id INTEGER,
    frame_seq INTEGER,
    uptime_ms INTEGER,
    restart_reason INTEGER,
    timer_state INTEGER,
    deploy_state INTEGER,
    sensor_flags INTEGER,
    remaining_s INTEGER,
    last_ack_command_id INTEGER,
    last_ack_result INTEGER,
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
    temperature REAL NOT NULL,
    lost_packets INTEGER,
    duplicate_packets INTEGER,
    crc_errors INTEGER,
    restart_count INTEGER
);

CREATE INDEX IF NOT EXISTS idx_telemetry_test_run_received
ON telemetry(test_run_id, received_unix_ms);

CREATE TABLE IF NOT EXISTS test_run_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    test_run_id TEXT NOT NULL REFERENCES test_runs(test_run_id),
    created_unix_ms INTEGER NOT NULL,
    level TEXT NOT NULL,
    message TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_test_run_events_run_time
ON test_run_events(test_run_id, created_unix_ms);
