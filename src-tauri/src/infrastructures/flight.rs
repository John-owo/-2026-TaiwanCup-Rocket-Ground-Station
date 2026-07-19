use crate::models::response::TelemetryPayload;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

// Formal 1800 ms v2 telemetry period × 2.5, rounded up to 100 ms.
const LINK_LOSS_THRESHOLD_MS: u64 = 4_500;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlightSessionMetadata {
    pub purpose: String,
    pub initial_battery_voltage: f32,
    pub location: String,
    pub operator: String,
    pub notes: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FlightStats {
    pub telemetry_packets: u64,
    pub expected_packets: u64,
    pub lost_packets: u64,
    pub duplicate_packets: u64,
    pub crc_errors: u64,
    pub link_outages: u64,
    pub max_link_loss_ms: u64,
    pub restart_count: u64,
}

#[derive(Default)]
pub struct FlightStatsTracker {
    current_session_id: Option<u32>,
    current_seen: HashSet<u32>,
    current_first_seq: Option<u32>,
    current_max_seq: Option<u32>,
    completed_expected: u64,
    completed_unique: u64,
    duplicate_packets: u64,
    crc_errors: u64,
    restart_count: u64,
    last_packet_ms: Option<u64>,
    active_outage: bool,
    link_outages: u64,
    max_link_loss_ms: u64,
}

impl FlightStatsTracker {
    pub fn observe_telemetry(&mut self, session_id: u32, frame_seq: u32, now_ms: u64) {
        if self.current_session_id != Some(session_id) {
            if self.current_session_id.is_some() {
                self.finish_current_session();
                self.restart_count += 1;
            }
            self.current_session_id = Some(session_id);
            self.current_seen.clear();
            self.current_first_seq = None;
            self.current_max_seq = None;
        }

        if !self.current_seen.insert(frame_seq) {
            self.duplicate_packets += 1;
        } else {
            self.current_first_seq = Some(self.current_first_seq.map_or(frame_seq, |value| value.min(frame_seq)));
            self.current_max_seq = Some(self.current_max_seq.map_or(frame_seq, |value| value.max(frame_seq)));
        }

        if let Some(last_packet_ms) = self.last_packet_ms {
            let gap = now_ms.saturating_sub(last_packet_ms);
            if gap > LINK_LOSS_THRESHOLD_MS {
                if !self.active_outage {
                    self.link_outages += 1;
                }
                self.max_link_loss_ms = self.max_link_loss_ms.max(gap);
            }
        }
        self.active_outage = false;
        self.last_packet_ms = Some(now_ms);
    }

    pub fn tick(&mut self, now_ms: u64) {
        let Some(last_packet_ms) = self.last_packet_ms else {
            return;
        };
        let gap = now_ms.saturating_sub(last_packet_ms);
        if gap > LINK_LOSS_THRESHOLD_MS {
            if !self.active_outage {
                self.active_outage = true;
                self.link_outages += 1;
            }
            self.max_link_loss_ms = self.max_link_loss_ms.max(gap);
        }
    }

    pub fn record_crc_error(&mut self) {
        self.crc_errors += 1;
    }

    pub fn snapshot(&self) -> FlightStats {
        let current_expected = current_span(self.current_first_seq, self.current_max_seq);
        let current_unique = self.current_seen.len() as u64;
        let expected_packets = self.completed_expected + current_expected;
        let telemetry_packets = self.completed_unique + current_unique;
        FlightStats {
            telemetry_packets,
            expected_packets,
            lost_packets: expected_packets.saturating_sub(telemetry_packets),
            duplicate_packets: self.duplicate_packets,
            crc_errors: self.crc_errors,
            link_outages: self.link_outages,
            max_link_loss_ms: self.max_link_loss_ms,
            restart_count: self.restart_count,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    fn finish_current_session(&mut self) {
        self.completed_expected += current_span(self.current_first_seq, self.current_max_seq);
        self.completed_unique += self.current_seen.len() as u64;
    }
}

fn current_span(first: Option<u32>, last: Option<u32>) -> u64 {
    match (first, last) {
        (Some(first), Some(last)) if last >= first => u64::from(last - first) + 1,
        _ => 0,
    }
}

pub struct FlightRecorder {
    test_run_id: String,
    directory: PathBuf,
    csv: BufWriter<File>,
    log: BufWriter<File>,
    metadata: FlightSessionMetadata,
}

impl FlightRecorder {
    pub fn start(
        root: &Path,
        test_run_id: String,
        metadata: FlightSessionMetadata,
    ) -> Result<Self, String> {
        metadata.validate()?;
        let timestamp = unix_ms();
        let short_id = test_run_id.chars().take(8).collect::<String>();
        let location = sanitize_component(&metadata.location);
        let sessions_root = root.join("flight_sessions");
        fs::create_dir_all(&sessions_root).map_err(|error| error.to_string())?;
        let directory = sessions_root.join(format!("{timestamp}_{short_id}_{location}"));
        fs::create_dir(&directory).map_err(|error| error.to_string())?;
        let result = (|| {
            let csv_file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(directory.join("flight_data.csv"))
                .map_err(|error| error.to_string())?;
            let log_file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(directory.join("system.log"))
                .map_err(|error| error.to_string())?;
            let mut recorder = Self {
                test_run_id,
                directory: directory.clone(),
                csv: BufWriter::new(csv_file),
                log: BufWriter::new(log_file),
                metadata,
            };
            recorder.write_csv_header()?;
            recorder.log_event("INFO", "flight session started")?;
            recorder.write_summary(&FlightStats::default(), "recording", None)?;
            Ok(recorder)
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(&directory);
        }
        result
    }

    pub fn cleanup_failed_start(&self) -> Result<(), String> {
        fs::remove_dir_all(&self.directory).map_err(|error| error.to_string())
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    pub fn record_telemetry(
        &mut self,
        payload: &TelemetryPayload,
        stats: &FlightStats,
    ) -> Result<(), String> {
        writeln!(
            self.csv,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            unix_ms(), payload.protocol_version, payload.session_id, payload.frame_seq,
            payload.uptime_ms, payload.restart_reason, payload.timer_state, payload.deploy_state,
            payload.sensor_flags, payload.remaining_s, payload.last_ack_command_id,
            payload.last_ack_result, payload.x_acceleration, payload.y_acceleration,
            payload.z_acceleration, payload.x_angular_velocity, payload.y_angular_velocity,
            payload.z_angular_velocity, payload.longitude, payload.latitude, payload.altitude,
            payload.ground_speed, payload.vertical_velocity, payload.air_pressure,
            payload.temperature, stats.lost_packets, stats.duplicate_packets, stats.crc_errors,
            stats.restart_count,
        )
        .map_err(|error| error.to_string())?;
        self.csv.flush().map_err(|error| error.to_string())?;
        self.write_summary(stats, "recording", None)
    }

    pub fn log_event(&mut self, level: &str, message: &str) -> Result<(), String> {
        writeln!(self.log, "{}\t{}\t{}", unix_ms(), level, message.replace(['\r', '\n'], " "))
            .map_err(|error| error.to_string())?;
        self.log.flush().map_err(|error| error.to_string())
    }

    pub fn finish(
        &mut self,
        stats: &FlightStats,
        status: &str,
        detail: Option<&str>,
    ) -> Result<(), String> {
        self.log_event(
            if status == "completed" { "INFO" } else { "ERROR" },
            detail.unwrap_or("flight session stopped"),
        )?;
        self.write_summary(stats, status, detail)
    }

    fn write_csv_header(&mut self) -> Result<(), String> {
        writeln!(
            self.csv,
            "received_unix_ms,protocol_version,session_id,frame_seq,uptime_ms,restart_reason,timer_state,deploy_state,sensor_flags,remaining_s,last_ack_command_id,last_ack_result,accel_x_mps2,accel_y_mps2,accel_z_mps2,gyro_x_dps,gyro_y_dps,gyro_z_dps,longitude_deg,latitude_deg,baro_altitude_m,ground_speed_mps,vertical_velocity_mps,pressure_hpa,temperature_c,lost_packets,duplicate_packets,crc_errors,restart_count"
        )
        .map_err(|error| error.to_string())?;
        self.csv.flush().map_err(|error| error.to_string())
    }

    fn write_summary(
        &self,
        stats: &FlightStats,
        status: &str,
        detail: Option<&str>,
    ) -> Result<(), String> {
        let summary = serde_json::json!({
            "updatedUnixMs": unix_ms(),
            "completed": status == "completed",
            "status": status,
            "testRunId": self.test_run_id,
            "detail": detail,
            "metadata": self.metadata,
            "stats": stats,
        });
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(self.directory.join("session_summary.json"))
            .map_err(|error| error.to_string())?;
        serde_json::to_writer_pretty(&mut file, &summary).map_err(|error| error.to_string())?;
        file.flush().map_err(|error| error.to_string())
    }
}

impl FlightSessionMetadata {
    pub fn validate(&self) -> Result<(), String> {
        if !self.initial_battery_voltage.is_finite() || self.initial_battery_voltage <= 0.0 {
            return Err("initial battery voltage must be a positive finite value".to_string());
        }
        if self.purpose.trim().is_empty()
            || self.location.trim().is_empty()
            || self.operator.trim().is_empty()
        {
            return Err("purpose, location and operator are required".to_string());
        }
        Ok(())
    }
}

fn sanitize_component(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .take(40)
        .collect();
    if sanitized.is_empty() {
        "flight".to_string()
    } else {
        sanitized
    }
}

fn unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_requires_purpose_operator_location_and_positive_voltage() {
        let valid = FlightSessionMetadata {
            purpose: "timer test".to_string(),
            initial_battery_voltage: 8.2,
            location: "lab".to_string(),
            operator: "tester".to_string(),
            notes: String::new(),
        };
        assert!(valid.validate().is_ok());
        for invalid in [
            FlightSessionMetadata { purpose: " ".to_string(), ..valid.clone() },
            FlightSessionMetadata { operator: "".to_string(), ..valid.clone() },
            FlightSessionMetadata { location: "".to_string(), ..valid.clone() },
            FlightSessionMetadata { initial_battery_voltage: 0.0, ..valid.clone() },
            FlightSessionMetadata { initial_battery_voltage: f32::NAN, ..valid.clone() },
        ] {
            assert!(invalid.validate().is_err());
        }
    }

    #[test]
    fn statistics_separate_missing_duplicate_crc_link_loss_and_restart() {
        let mut tracker = FlightStatsTracker::default();
        tracker.observe_telemetry(1, 10, 0);
        tracker.observe_telemetry(1, 12, 1_000);
        tracker.observe_telemetry(1, 12, 1_100);
        tracker.record_crc_error();
        tracker.tick(7_000);
        tracker.observe_telemetry(2, 1, 7_500);
        let stats = tracker.snapshot();
        assert_eq!(stats.telemetry_packets, 3);
        assert_eq!(stats.expected_packets, 4);
        assert_eq!(stats.lost_packets, 1);
        assert_eq!(stats.duplicate_packets, 1);
        assert_eq!(stats.crc_errors, 1);
        assert_eq!(stats.link_outages, 1);
        assert_eq!(stats.max_link_loss_ms, 6_400);
        assert_eq!(stats.restart_count, 1);
    }

    #[test]
    fn recorder_flushes_csv_log_and_summary_for_mid_run_readback() {
        let root = std::env::temp_dir().join(format!("tasa-flight-recorder-{}", unix_ms()));
        let metadata = FlightSessionMetadata {
            purpose: "storage test".to_string(),
            initial_battery_voltage: 8.2,
            location: "lab".to_string(),
            operator: "tester".to_string(),
            notes: "automated".to_string(),
        };
        let mut recorder = FlightRecorder::start(
            &root,
            uuid::Uuid::new_v4().to_string(),
            metadata,
        )
        .unwrap();
        let directory = recorder.directory().to_path_buf();
        recorder
            .record_telemetry(&telemetry(), &FlightStats { telemetry_packets: 1, ..Default::default() })
            .unwrap();
        recorder.log_event("WARN", "CRC error test").unwrap();

        let csv = fs::read_to_string(directory.join("flight_data.csv")).unwrap();
        let log = fs::read_to_string(directory.join("system.log")).unwrap();
        let summary = fs::read_to_string(directory.join("session_summary.json")).unwrap();
        assert!(csv.contains("frame_seq"));
        assert!(csv.lines().count() >= 2);
        assert!(log.contains("CRC error test"));
        assert!(summary.contains("initialBatteryVoltage"));
        assert!(summary.contains("telemetryPackets"));
        drop(recorder);
        fs::remove_dir_all(root).unwrap();
    }

    fn telemetry() -> TelemetryPayload {
        TelemetryPayload {
            protocol_version: 1,
            session_id: 1,
            frame_seq: 1,
            uptime_ms: 1,
            restart_reason: 1,
            timer_state: 0,
            deploy_state: 0,
            sensor_flags: 0,
            remaining_s: 0,
            last_ack_command_id: 0,
            last_ack_result: 0xFF,
            x_acceleration: 0.0,
            y_acceleration: 0.0,
            z_acceleration: 0.0,
            x_angular_velocity: 0.0,
            y_angular_velocity: 0.0,
            z_angular_velocity: 0.0,
            longitude: 0.0,
            latitude: 0.0,
            altitude: 0.0,
            ground_speed: 0.0,
            vertical_velocity: 0.0,
            air_pressure: 0.0,
            temperature: 0.0,
        }
    }
}
