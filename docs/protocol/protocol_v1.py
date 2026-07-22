"""Reference codec for the TASA RTC 2026 LoRa Protocol v1.

This module intentionally uses only the Python standard library so the same
golden vectors can be checked on any development machine.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import IntEnum, IntFlag
import struct
from typing import Any, Sequence


MAGIC = b"\xA5\x5A"
VERSION = 1
HEADER_LENGTH = 20
CRC_LENGTH = 2
MAX_PAYLOAD_LENGTH = 96

HEADER_STRUCT = struct.Struct(">2sBBBBHIII")
CRC_STRUCT = struct.Struct(">H")
TELEMETRY_STRUCT = struct.Struct(">IBBBBIIB3x13f")
SET_TIMER_STRUCT = struct.Struct(">I")
ACK_STRUCT = struct.Struct(">BBBBI")

SENSOR_FIELD_NAMES = (
    "accel_x_mps2",
    "accel_y_mps2",
    "accel_z_mps2",
    "gyro_x_dps",
    "gyro_y_dps",
    "gyro_z_dps",
    "longitude_deg",
    "latitude_deg",
    "baro_altitude_m",
    "ground_speed_mps",
    "vertical_velocity_mps",
    "pressure_hpa",
    "temperature_c",
)


class FrameType(IntEnum):
    TELEMETRY = 0x01
    SET_TIMER = 0x10
    FORCE_RELEASE = 0x11
    ACK = 0x7F


class FrameFlags(IntFlag):
    NONE = 0x00
    RETRANSMISSION = 0x01


class RestartReason(IntEnum):
    UNKNOWN = 0x00
    POWER_ON = 0x01
    SOFTWARE = 0x02
    WATCHDOG = 0x03
    BROWNOUT = 0x04
    PANIC = 0x05
    USB_OR_JTAG = 0x06
    OTHER = 0xFF


class TimerState(IntEnum):
    UNSET = 0x00
    RUNNING = 0x01
    EXPIRED = 0x02


class DeployState(IntEnum):
    SAFE = 0x00
    DEPLOYED = 0x01


class SensorFlags(IntFlag):
    NONE = 0x00
    MPU6050_VALID = 0x01
    BMP280_VALID = 0x02
    GPS_LOCATION_FRESH = 0x04
    GPS_SPEED_FRESH = 0x08


class AckResult(IntEnum):
    EXECUTED = 0x00
    DUPLICATE = 0x01
    ALREADY_DEPLOYED = 0x02
    STALE_COMMAND = 0x03
    BAD_PAYLOAD = 0x10
    SESSION_MISMATCH = 0x11
    UNSUPPORTED_TYPE = 0x12
    INVALID_STATE = 0x13
    COMMAND_ID_CONFLICT = 0x14
    NONE = 0xFF


EXPECTED_PAYLOAD_LENGTHS = {
    FrameType.TELEMETRY: TELEMETRY_STRUCT.size,
    FrameType.SET_TIMER: SET_TIMER_STRUCT.size,
    FrameType.FORCE_RELEASE: 0,
    FrameType.ACK: ACK_STRUCT.size,
}

COMMAND_TYPES = {FrameType.SET_TIMER, FrameType.FORCE_RELEASE}


@dataclass(frozen=True)
class Frame:
    frame_type: int
    flags: int
    session_id: int
    frame_seq: int
    command_id: int
    payload: bytes


class FrameDecodeError(ValueError):
    def __init__(self, code: str, message: str):
        super().__init__(message)
        self.code = code


def crc16_ccitt_false(data: bytes) -> int:
    crc = 0xFFFF
    for value in data:
        crc ^= value << 8
        for _ in range(8):
            crc = ((crc << 1) ^ 0x1021) & 0xFFFF if crc & 0x8000 else (crc << 1) & 0xFFFF
    return crc


def _require_uint(name: str, value: int, bits: int) -> None:
    maximum = (1 << bits) - 1
    if not isinstance(value, int) or not 0 <= value <= maximum:
        raise ValueError(f"{name} must be uint{bits}")


def _validate_known_frame(frame: Frame, *, decode: bool) -> None:
    error_type = FrameDecodeError if decode else ValueError

    if frame.flags & ~int(FrameFlags.RETRANSMISSION):
        if decode:
            raise error_type("FLAGS_ERROR", "reserved flag bits must be zero")
        raise error_type("reserved flag bits must be zero")
    if frame.session_id == 0:
        if decode:
            raise error_type("SESSION_ERROR", "session_id must be non-zero")
        raise error_type("session_id must be non-zero")

    try:
        frame_type = FrameType(frame.frame_type)
    except ValueError:
        return

    expected_length = EXPECTED_PAYLOAD_LENGTHS[frame_type]
    if len(frame.payload) != expected_length:
        if decode:
            raise error_type(
                "PAYLOAD_LENGTH_ERROR",
                f"{frame_type.name} payload must be {expected_length} bytes",
            )
        raise error_type(f"{frame_type.name} payload must be {expected_length} bytes")

    if frame_type == FrameType.TELEMETRY:
        if frame.command_id != 0:
            if decode:
                raise error_type("COMMAND_ID_ERROR", "TELEMETRY command_id must be zero")
            raise error_type("TELEMETRY command_id must be zero")
        if frame.flags != int(FrameFlags.NONE):
            if decode:
                raise error_type("FLAGS_ERROR", "TELEMETRY flags must be zero")
            raise error_type("TELEMETRY flags must be zero")
    elif frame_type in COMMAND_TYPES or frame_type == FrameType.ACK:
        if frame.command_id == 0:
            if decode:
                raise error_type("COMMAND_ID_ERROR", f"{frame_type.name} command_id must be non-zero")
            raise error_type(f"{frame_type.name} command_id must be non-zero")
        if frame_type == FrameType.ACK and frame.flags != int(FrameFlags.NONE):
            if decode:
                raise error_type("FLAGS_ERROR", "ACK flags must be zero")
            raise error_type("ACK flags must be zero")


def encode_frame(frame: Frame) -> bytes:
    _require_uint("frame_type", frame.frame_type, 8)
    _require_uint("flags", frame.flags, 8)
    _require_uint("session_id", frame.session_id, 32)
    _require_uint("frame_seq", frame.frame_seq, 32)
    _require_uint("command_id", frame.command_id, 32)
    if not isinstance(frame.payload, bytes):
        raise ValueError("payload must be bytes")
    if len(frame.payload) > MAX_PAYLOAD_LENGTH:
        raise ValueError(f"payload exceeds {MAX_PAYLOAD_LENGTH} bytes")
    _validate_known_frame(frame, decode=False)

    header = HEADER_STRUCT.pack(
        MAGIC,
        VERSION,
        frame.frame_type,
        frame.flags,
        HEADER_LENGTH,
        len(frame.payload),
        frame.session_id,
        frame.frame_seq,
        frame.command_id,
    )
    crc = crc16_ccitt_false(header[2:] + frame.payload)
    return header + frame.payload + CRC_STRUCT.pack(crc)


def decode_frame(data: bytes) -> Frame:
    if len(data) < HEADER_LENGTH:
        raise FrameDecodeError("TRUNCATED", "frame is shorter than the common header")

    magic, version, frame_type, flags, header_length, payload_length, session_id, frame_seq, command_id = (
        HEADER_STRUCT.unpack_from(data)
    )
    if magic != MAGIC:
        raise FrameDecodeError("MAGIC_ERROR", "sync magic does not match")
    if version != VERSION:
        raise FrameDecodeError("VERSION_ERROR", f"unsupported protocol version {version}")
    if header_length != HEADER_LENGTH:
        raise FrameDecodeError("HEADER_LENGTH_ERROR", f"header_length must be {HEADER_LENGTH}")
    if payload_length > MAX_PAYLOAD_LENGTH:
        raise FrameDecodeError("PAYLOAD_LENGTH_ERROR", "payload exceeds protocol maximum")

    expected_total = HEADER_LENGTH + payload_length + CRC_LENGTH
    if len(data) < expected_total:
        raise FrameDecodeError("TRUNCATED", f"expected {expected_total} bytes, received {len(data)}")
    if len(data) > expected_total:
        raise FrameDecodeError("TRAILING_BYTES", f"expected {expected_total} bytes, received {len(data)}")

    payload = data[HEADER_LENGTH : HEADER_LENGTH + payload_length]
    received_crc = CRC_STRUCT.unpack_from(data, HEADER_LENGTH + payload_length)[0]
    calculated_crc = crc16_ccitt_false(data[2 : HEADER_LENGTH + payload_length])
    if received_crc != calculated_crc:
        raise FrameDecodeError(
            "CRC_ERROR",
            f"received 0x{received_crc:04X}, calculated 0x{calculated_crc:04X}",
        )

    frame = Frame(frame_type, flags, session_id, frame_seq, command_id, payload)
    _validate_known_frame(frame, decode=True)
    return frame


def encode_telemetry_payload(
    *,
    uptime_ms: int,
    restart_reason: int,
    timer_state: int,
    deploy_state: int,
    sensor_flags: int,
    remaining_s: int,
    last_ack_command_id: int,
    last_ack_result: int,
    sensors: Sequence[float],
) -> bytes:
    _require_uint("uptime_ms", uptime_ms, 32)
    _require_uint("restart_reason", restart_reason, 8)
    _require_uint("timer_state", timer_state, 8)
    _require_uint("deploy_state", deploy_state, 8)
    _require_uint("sensor_flags", sensor_flags, 8)
    _require_uint("remaining_s", remaining_s, 32)
    _require_uint("last_ack_command_id", last_ack_command_id, 32)
    _require_uint("last_ack_result", last_ack_result, 8)
    if len(sensors) != len(SENSOR_FIELD_NAMES):
        raise ValueError(f"sensors must contain {len(SENSOR_FIELD_NAMES)} float values")
    return TELEMETRY_STRUCT.pack(
        uptime_ms,
        restart_reason,
        timer_state,
        deploy_state,
        sensor_flags,
        remaining_s,
        last_ack_command_id,
        last_ack_result,
        *sensors,
    )


def encode_set_timer_payload(duration_s: int) -> bytes:
    _require_uint("duration_s", duration_s, 32)
    if duration_s == 0:
        raise ValueError("duration_s must be non-zero; use FORCE_RELEASE for immediate deployment")
    return SET_TIMER_STRUCT.pack(duration_s)


def encode_ack_payload(
    *,
    acked_type: int,
    result: int,
    timer_state: int,
    deploy_state: int,
    remaining_s: int,
) -> bytes:
    _require_uint("acked_type", acked_type, 8)
    _require_uint("result", result, 8)
    _require_uint("timer_state", timer_state, 8)
    _require_uint("deploy_state", deploy_state, 8)
    _require_uint("remaining_s", remaining_s, 32)
    return ACK_STRUCT.pack(acked_type, result, timer_state, deploy_state, remaining_s)


def decode_known_payload(frame: Frame) -> dict[str, Any]:
    frame_type = FrameType(frame.frame_type)
    if frame_type == FrameType.TELEMETRY:
        values = TELEMETRY_STRUCT.unpack(frame.payload)
        result: dict[str, Any] = {
            "uptime_ms": values[0],
            "restart_reason": values[1],
            "timer_state": values[2],
            "deploy_state": values[3],
            "sensor_flags": values[4],
            "remaining_s": values[5],
            "last_ack_command_id": values[6],
            "last_ack_result": values[7],
        }
        result.update(zip(SENSOR_FIELD_NAMES, values[8:]))
        return result
    if frame_type == FrameType.SET_TIMER:
        return {"duration_s": SET_TIMER_STRUCT.unpack(frame.payload)[0]}
    if frame_type == FrameType.FORCE_RELEASE:
        return {}
    if frame_type == FrameType.ACK:
        values = ACK_STRUCT.unpack(frame.payload)
        return {
            "acked_type": values[0],
            "result": values[1],
            "timer_state": values[2],
            "deploy_state": values[3],
            "remaining_s": values[4],
        }
    raise ValueError(f"unknown frame type 0x{frame.frame_type:02X}")
