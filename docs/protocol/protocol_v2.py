"""Reference codec for the compact TASA RTC 2026 LoRa Protocol v2."""

from __future__ import annotations

from dataclasses import dataclass
from enum import IntEnum, IntFlag
import math
import struct
from typing import Any, Sequence


MAGIC = b"\xA5\x5A"
VERSION = 2
HEADER_LENGTH = 14
CRC_LENGTH = 2
MAX_PAYLOAD_LENGTH = 96

HEADER_STRUCT = struct.Struct(">2sBBBBII")
CRC_STRUCT = struct.Struct(">H")
TELEMETRY_STRUCT = struct.Struct(">IBBIIB3h3h3iHhHh")
SET_TIMER_STRUCT = struct.Struct(">I")
ACK_STRUCT = struct.Struct(">BBBI")

TELEMETRY_PAYLOAD_LENGTH = 47
TELEMETRY_FRAME_LENGTH = 63
SET_TIMER_FRAME_LENGTH = 20
FORCE_RELEASE_FRAME_LENGTH = 16
ACK_FRAME_LENGTH = 23


class FrameType(IntEnum):
    TELEMETRY = 0x01
    SET_TIMER = 0x10
    FORCE_RELEASE = 0x11
    ACK = 0x7F


class FrameFlags(IntFlag):
    NONE = 0x00
    RETRANSMISSION = 0x01


class SensorFlags(IntFlag):
    NONE = 0x00
    MPU6050_VALID = 0x01
    BMP280_VALID = 0x02
    GPS_LOCATION_FRESH = 0x04
    GPS_SPEED_FRESH = 0x08


class TimerState(IntEnum):
    UNSET = 0
    RUNNING = 1
    EXPIRED = 2


class DeployState(IntEnum):
    SAFE = 0
    DEPLOYED = 1


EXPECTED_PAYLOAD_LENGTHS = {
    FrameType.TELEMETRY: TELEMETRY_STRUCT.size,
    FrameType.SET_TIMER: SET_TIMER_STRUCT.size,
    FrameType.FORCE_RELEASE: 0,
    FrameType.ACK: ACK_STRUCT.size,
}

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


@dataclass(frozen=True)
class Frame:
    frame_type: int
    flags: int
    session_id: int
    message_id: int
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
    if not isinstance(value, int) or not 0 <= value < (1 << bits):
        raise ValueError(f"{name} must be uint{bits}")


def _round_away_from_zero(value: float) -> int:
    return math.floor(value + 0.5) if value >= 0 else math.ceil(value - 0.5)


def _quantize(value: float, scale: float, minimum: int, maximum: int) -> int | None:
    if not math.isfinite(value):
        return None
    quantized = _round_away_from_zero(value * scale)
    return quantized if minimum <= quantized <= maximum else None


def _pack_status(timer_state: int, deploy_state: int, sensor_flags: int) -> int:
    _require_uint("timer_state", timer_state, 8)
    _require_uint("deploy_state", deploy_state, 8)
    _require_uint("sensor_flags", sensor_flags, 8)
    if timer_state > int(TimerState.EXPIRED):
        raise ValueError("timer_state must be UNSET, RUNNING, or EXPIRED")
    if deploy_state > int(DeployState.DEPLOYED):
        raise ValueError("deploy_state must be SAFE or DEPLOYED")
    if sensor_flags & 0xF0:
        raise ValueError("reserved sensor flag bits must be zero")
    return timer_state | (deploy_state << 2) | (sensor_flags << 3)


def _unpack_status(status: int) -> tuple[int, int, int]:
    if status & 0x80:
        raise FrameDecodeError("STATUS_ERROR", "reserved status bit is set")
    timer_state = status & 0x03
    deploy_state = (status >> 2) & 0x01
    sensor_flags = (status >> 3) & 0x0F
    if timer_state > int(TimerState.EXPIRED):
        raise FrameDecodeError("STATUS_ERROR", "reserved timer state is set")
    return timer_state, deploy_state, sensor_flags


def encode_frame(frame: Frame) -> bytes:
    _require_uint("frame_type", frame.frame_type, 8)
    _require_uint("flags", frame.flags, 8)
    _require_uint("session_id", frame.session_id, 32)
    _require_uint("message_id", frame.message_id, 32)
    if frame.session_id == 0:
        raise ValueError("session_id must be non-zero")
    if frame.flags & ~int(FrameFlags.RETRANSMISSION):
        raise ValueError("reserved flag bits must be zero")
    if len(frame.payload) > MAX_PAYLOAD_LENGTH:
        raise ValueError("payload exceeds protocol maximum")

    try:
        frame_type = FrameType(frame.frame_type)
    except ValueError:
        frame_type = None
    if frame_type is not None and len(frame.payload) != EXPECTED_PAYLOAD_LENGTHS[frame_type]:
        raise ValueError(f"{frame_type.name} payload length is invalid")
    if frame_type in (FrameType.TELEMETRY, FrameType.ACK) and frame.flags != 0:
        raise ValueError(f"{frame_type.name} flags must be zero")
    if frame.message_id == 0:
        raise ValueError("message_id must be non-zero")

    header = HEADER_STRUCT.pack(
        MAGIC, VERSION, frame.frame_type, frame.flags, len(frame.payload), frame.session_id, frame.message_id
    )
    crc = crc16_ccitt_false(header[2:] + frame.payload)
    return header + frame.payload + CRC_STRUCT.pack(crc)


def decode_frame(data: bytes) -> Frame:
    if len(data) < HEADER_LENGTH:
        raise FrameDecodeError("TRUNCATED", "frame is shorter than the v2 header")
    magic, version, frame_type, flags, payload_length, session_id, message_id = HEADER_STRUCT.unpack_from(data)
    if magic != MAGIC:
        raise FrameDecodeError("MAGIC_ERROR", "sync magic does not match")
    if version != VERSION:
        raise FrameDecodeError("VERSION_ERROR", f"unsupported protocol version {version}")
    if flags & ~int(FrameFlags.RETRANSMISSION):
        raise FrameDecodeError("FLAGS_ERROR", "reserved flag bits must be zero")
    if payload_length > MAX_PAYLOAD_LENGTH:
        raise FrameDecodeError("PAYLOAD_LENGTH_ERROR", "payload exceeds protocol maximum")
    expected_total = HEADER_LENGTH + payload_length + CRC_LENGTH
    if len(data) < expected_total:
        raise FrameDecodeError("TRUNCATED", f"expected {expected_total} bytes, received {len(data)}")
    if len(data) > expected_total:
        raise FrameDecodeError("TRAILING_BYTES", f"expected {expected_total} bytes, received {len(data)}")
    if session_id == 0:
        raise FrameDecodeError("SESSION_ERROR", "session_id must be non-zero")
    if message_id == 0:
        raise FrameDecodeError("MESSAGE_ID_ERROR", "message_id must be non-zero")

    try:
        known_type = FrameType(frame_type)
    except ValueError:
        known_type = None
    if known_type is not None and payload_length != EXPECTED_PAYLOAD_LENGTHS[known_type]:
        raise FrameDecodeError("PAYLOAD_LENGTH_ERROR", f"{known_type.name} payload length is invalid")
    if known_type in (FrameType.TELEMETRY, FrameType.ACK) and flags != 0:
        raise FrameDecodeError("FLAGS_ERROR", f"{known_type.name} flags must be zero")

    received_crc = CRC_STRUCT.unpack_from(data, expected_total - CRC_LENGTH)[0]
    calculated_crc = crc16_ccitt_false(data[2 : expected_total - CRC_LENGTH])
    if received_crc != calculated_crc:
        raise FrameDecodeError(
            "CRC_ERROR", f"received 0x{received_crc:04X}, calculated 0x{calculated_crc:04X}"
        )
    return Frame(frame_type, flags, session_id, message_id, data[HEADER_LENGTH:-CRC_LENGTH])


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
    _require_uint("remaining_s", remaining_s, 32)
    _require_uint("last_ack_command_id", last_ack_command_id, 32)
    _require_uint("last_ack_result", last_ack_result, 8)
    _require_uint("sensor_flags", sensor_flags, 8)
    if sensor_flags & 0xF0:
        raise ValueError("reserved sensor flag bits must be zero")
    if len(sensors) != 13:
        raise ValueError("sensors must contain 13 values")

    effective_flags = sensor_flags
    encoded = [0] * 13
    groups = (
        (int(SensorFlags.MPU6050_VALID), range(0, 6), ((100, -32768, 32767),) * 3 + ((10, -32768, 32767),) * 3),
        (int(SensorFlags.GPS_LOCATION_FRESH), range(6, 8), ((1_000_000, -2147483648, 2147483647),) * 2),
        (int(SensorFlags.BMP280_VALID), (8, 10, 11, 12), ((100, -2147483648, 2147483647), (10, -32768, 32767), (10, 0, 65535), (100, -32768, 32767))),
        (int(SensorFlags.GPS_SPEED_FRESH), (9,), ((100, 0, 65535),)),
    )
    for flag, indices, formats in groups:
        if not effective_flags & flag:
            continue
        if flag == int(SensorFlags.GPS_LOCATION_FRESH) and not (
            math.isfinite(float(sensors[6]))
            and math.isfinite(float(sensors[7]))
            and -180.0 <= float(sensors[6]) <= 180.0
            and -90.0 <= float(sensors[7]) <= 90.0
        ):
            effective_flags &= ~flag
            continue
        values = [_quantize(float(sensors[index]), *spec) for index, spec in zip(indices, formats)]
        if any(value is None for value in values):
            effective_flags &= ~flag
            continue
        for index, value in zip(indices, values):
            encoded[index] = int(value)

    status = _pack_status(timer_state, deploy_state, effective_flags)
    return TELEMETRY_STRUCT.pack(
        uptime_ms,
        restart_reason,
        status,
        remaining_s,
        last_ack_command_id,
        last_ack_result,
        *encoded[:3],
        *encoded[3:6],
        *encoded[6:9],
        encoded[9],
        encoded[10],
        encoded[11],
        encoded[12],
    )


def encode_set_timer_payload(duration_s: int) -> bytes:
    _require_uint("duration_s", duration_s, 32)
    if duration_s == 0:
        raise ValueError("duration_s must be non-zero")
    return SET_TIMER_STRUCT.pack(duration_s)


def encode_ack_payload(*, acked_type: int, result: int, timer_state: int, deploy_state: int, remaining_s: int) -> bytes:
    _require_uint("acked_type", acked_type, 8)
    _require_uint("result", result, 8)
    _require_uint("remaining_s", remaining_s, 32)
    return ACK_STRUCT.pack(acked_type, result, _pack_status(timer_state, deploy_state, 0), remaining_s)


def decode_known_payload(frame: Frame) -> dict[str, Any]:
    frame_type = FrameType(frame.frame_type)
    if frame_type == FrameType.SET_TIMER:
        return {"duration_s": SET_TIMER_STRUCT.unpack(frame.payload)[0]}
    if frame_type == FrameType.FORCE_RELEASE:
        return {}
    if frame_type == FrameType.ACK:
        acked_type, result, status, remaining_s = ACK_STRUCT.unpack(frame.payload)
        timer_state, deploy_state, sensor_flags = _unpack_status(status)
        if sensor_flags:
            raise FrameDecodeError("STATUS_ERROR", "ACK sensor flags must be zero")
        return {
            "acked_type": acked_type,
            "result": result,
            "timer_state": timer_state,
            "deploy_state": deploy_state,
            "remaining_s": remaining_s,
        }

    values = TELEMETRY_STRUCT.unpack(frame.payload)
    timer_state, deploy_state, sensor_flags = _unpack_status(values[2])
    raw = values[6:]
    sensors = (
        raw[0] / 100.0,
        raw[1] / 100.0,
        raw[2] / 100.0,
        raw[3] / 10.0,
        raw[4] / 10.0,
        raw[5] / 10.0,
        raw[6] / 1_000_000.0,
        raw[7] / 1_000_000.0,
        raw[8] / 100.0,
        raw[9] / 100.0,
        raw[10] / 10.0,
        raw[11] / 10.0,
        raw[12] / 100.0,
    )
    result: dict[str, Any] = {
        "uptime_ms": values[0],
        "restart_reason": values[1],
        "timer_state": timer_state,
        "deploy_state": deploy_state,
        "sensor_flags": sensor_flags,
        "remaining_s": values[3],
        "last_ack_command_id": values[4],
        "last_ack_result": values[5],
    }
    result.update(zip(SENSOR_FIELD_NAMES, sensors))
    return result


assert HEADER_STRUCT.size == HEADER_LENGTH
assert TELEMETRY_STRUCT.size == TELEMETRY_PAYLOAD_LENGTH
assert HEADER_LENGTH + TELEMETRY_PAYLOAD_LENGTH + CRC_LENGTH == TELEMETRY_FRAME_LENGTH
