"""Verify all Protocol v1 golden frames and malformed-frame vectors."""

from __future__ import annotations

import json
from pathlib import Path
import sys
from typing import Any

from protocol_v1 import (
    AckResult,
    Frame,
    FrameDecodeError,
    FrameFlags,
    FrameType,
    crc16_ccitt_false,
    decode_frame,
    decode_known_payload,
    encode_ack_payload,
    encode_frame,
    encode_set_timer_payload,
    encode_telemetry_payload,
)


VECTOR_PATH = Path(__file__).with_name("test_vectors_v1.json")


def build_payload(frame_type: FrameType, values: dict[str, Any]) -> bytes:
    if frame_type == FrameType.TELEMETRY:
        return encode_telemetry_payload(**values)
    if frame_type == FrameType.SET_TIMER:
        return encode_set_timer_payload(**values)
    if frame_type == FrameType.FORCE_RELEASE:
        if values:
            raise AssertionError("FORCE_RELEASE payload must be empty")
        return b""
    if frame_type == FrameType.ACK:
        return encode_ack_payload(**values)
    raise AssertionError(f"unsupported test frame type {frame_type.name}")


def build_frame(values: dict[str, Any]) -> Frame:
    frame_type = FrameType[values["frame_type"]]
    return Frame(
        frame_type=int(frame_type),
        flags=values["flags"],
        session_id=values["session_id"],
        frame_seq=values["frame_seq"],
        command_id=values["command_id"],
        payload=build_payload(frame_type, values["payload"]),
    )


def verify_duplicate(base: Frame, retry: Frame) -> None:
    if retry.session_id != base.session_id:
        raise AssertionError("duplicate retry changed session_id")
    if retry.command_id != base.command_id:
        raise AssertionError("duplicate retry changed command_id")
    if retry.frame_type != base.frame_type or retry.payload != base.payload:
        raise AssertionError("duplicate retry changed command type or payload")
    if retry.frame_seq == base.frame_seq:
        raise AssertionError("each physical retry must use a new frame_seq")
    if not retry.flags & int(FrameFlags.RETRANSMISSION):
        raise AssertionError("duplicate retry must set RETRANSMISSION")


def main() -> int:
    document = json.loads(VECTOR_PATH.read_text(encoding="utf-8"))
    if document["protocol_version"] != 1:
        raise AssertionError("test-vector protocol version must be 1")
    if crc16_ccitt_false(b"123456789") != 0x29B1:
        raise AssertionError("CRC-16/CCITT-FALSE self-test failed")

    decoded: dict[str, Frame] = {}
    failures: list[str] = []
    for vector in document["vectors"]:
        vector_id = vector["id"]
        raw = bytes.fromhex(vector["frame_hex"])
        expected = vector["expected"]
        try:
            if len(raw) != expected["length"]:
                raise AssertionError(f"length is {len(raw)}, expected {expected['length']}")

            expected_parse = expected["parse"]
            if expected_parse == "OK":
                parsed = decode_frame(raw)
                expected_frame = build_frame(vector["frame"])
                if parsed != expected_frame:
                    raise AssertionError(f"decoded frame differs: {parsed!r}")
                if encode_frame(expected_frame) != raw:
                    raise AssertionError("encoded bytes differ from golden frame")
                decode_known_payload(parsed)
                decoded[vector_id] = parsed
            else:
                try:
                    decode_frame(raw)
                except FrameDecodeError as error:
                    if error.code != expected_parse:
                        raise AssertionError(f"parse error is {error.code}, expected {expected_parse}") from error
                else:
                    raise AssertionError(f"expected parse error {expected_parse}")

            print(f"[PASS] {vector_id}")
        except Exception as error:  # Keep all vector failures visible in one run.
            failures.append(f"{vector_id}: {error}")
            print(f"[FAIL] {vector_id}: {error}")

    for vector in document["vectors"]:
        duplicate_of = vector.get("duplicate_of")
        if not duplicate_of or vector["id"] not in decoded or duplicate_of not in decoded:
            continue
        try:
            verify_duplicate(decoded[duplicate_of], decoded[vector["id"]])
            if vector["expected"].get("command_action") != "ACK_DUPLICATE_NO_EXECUTE":
                raise AssertionError("duplicate vector must require ACK_DUPLICATE_NO_EXECUTE")
            print(f"[PASS] {vector['id']} duplicate semantics")
        except Exception as error:
            failures.append(f"{vector['id']} duplicate semantics: {error}")
            print(f"[FAIL] {vector['id']} duplicate semantics: {error}")

    if failures:
        print("\nProtocol v1 verification failed:")
        for failure in failures:
            print(f"- {failure}")
        return 1

    print(f"\nProtocol v1 verification passed: {len(document['vectors'])} vectors")
    print(f"ACK duplicate result code: 0x{int(AckResult.DUPLICATE):02X}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
