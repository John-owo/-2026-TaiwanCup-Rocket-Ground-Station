"""Verify Protocol v2 golden frames, quantization, and malformed inputs."""

from __future__ import annotations

import json
from pathlib import Path
import sys

from protocol_v2 import (
    Frame,
    FrameDecodeError,
    FrameFlags,
    FrameType,
    SensorFlags,
    crc16_ccitt_false,
    decode_frame,
    decode_known_payload,
    encode_ack_payload,
    encode_frame,
    encode_set_timer_payload,
    encode_telemetry_payload,
)


VECTOR_PATH = Path(__file__).with_name("test_vectors_v2.json")


def build_frame(values: dict) -> Frame:
    frame_type = FrameType[values["frame_type"]]
    payload_values = values["payload"]
    if frame_type == FrameType.TELEMETRY:
        payload = encode_telemetry_payload(**payload_values)
    elif frame_type == FrameType.SET_TIMER:
        payload = encode_set_timer_payload(**payload_values)
    elif frame_type == FrameType.FORCE_RELEASE:
        payload = b""
    else:
        payload = encode_ack_payload(**payload_values)
    return Frame(int(frame_type), values["flags"], values["session_id"], values["message_id"], payload)


def verify_quantization() -> None:
    payload = encode_telemetry_payload(
        uptime_ms=1,
        restart_reason=1,
        timer_state=0,
        deploy_state=0,
        sensor_flags=15,
        remaining_s=0,
        last_ack_command_id=0,
        last_ack_result=0xFF,
        sensors=[0.005, -0.005, 327.674, 3276.74, -3276.74, 0.05, 180.0, -90.0, 21474836.47, 655.354, -3276.74, 6553.54, -327.674],
    )
    decoded = decode_known_payload(Frame(1, 0, 1, 1, payload))
    assert decoded["accel_x_mps2"] == 0.01
    assert decoded["accel_y_mps2"] == -0.01
    assert decoded["gyro_z_dps"] == 0.1
    assert decoded["sensor_flags"] == 15

    invalid = encode_telemetry_payload(
        uptime_ms=1,
        restart_reason=1,
        timer_state=0,
        deploy_state=0,
        sensor_flags=15,
        remaining_s=0,
        last_ack_command_id=0,
        last_ack_result=0xFF,
        sensors=[float("nan"), 0, 0, 0, 0, 0, 181, 0, 0, -1, 0, -1, 0],
    )
    decoded_invalid = decode_known_payload(Frame(1, 0, 1, 1, invalid))
    assert decoded_invalid["sensor_flags"] == 0
    assert all(decoded_invalid[name] == 0 for name in (
        "accel_x_mps2", "gyro_x_dps", "longitude_deg", "baro_altitude_m", "ground_speed_mps"
    ))


def main() -> int:
    document = json.loads(VECTOR_PATH.read_text(encoding="utf-8"))
    assert document["protocol_version"] == 2
    assert crc16_ccitt_false(b"123456789") == 0x29B1
    decoded: dict[str, Frame] = {}
    failures: list[str] = []

    for vector in document["vectors"]:
        try:
            raw = bytes.fromhex(vector["frame_hex"])
            assert len(raw) == vector["expected"]["length"]
            if vector["expected"]["parse"] == "OK":
                expected = build_frame(vector["frame"])
                parsed = decode_frame(raw)
                assert parsed == expected
                assert encode_frame(expected) == raw
                decode_known_payload(parsed)
                decoded[vector["id"]] = parsed
            else:
                try:
                    decode_frame(raw)
                except FrameDecodeError as error:
                    assert error.code == vector["expected"]["parse"]
                else:
                    raise AssertionError("malformed frame unexpectedly parsed")
            print(f"[PASS] {vector['id']}")
        except Exception as error:
            failures.append(f"{vector['id']}: {error}")
            print(f"[FAIL] {vector['id']}: {error}")

    try:
        base = decoded["set_timer_nominal_v2"]
        retry = decoded["set_timer_retry_v2"]
        assert retry.message_id == base.message_id
        assert retry.payload == base.payload
        assert retry.flags == int(FrameFlags.RETRANSMISSION)
        verify_quantization()
        print("[PASS] duplicate semantics and fixed-point boundaries")
    except Exception as error:
        failures.append(f"semantics: {error}")

    if failures:
        print("\nProtocol v2 verification failed:")
        for failure in failures:
            print(f"- {failure}")
        return 1
    print(f"\nProtocol v2 verification passed: {len(document['vectors'])} vectors")
    return 0


if __name__ == "__main__":
    sys.exit(main())
