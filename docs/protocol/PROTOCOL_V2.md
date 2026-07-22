# TASA RTC 2026 LoRa 雙向封包協定 v2

狀態：Protocol v1 相容升版草案
日期：2026-07-17

## 1. 目的與相容性

v2 將 94-byte v1 遙測改為 63-byte 定點數遙測，仍保留全部 13 個感測欄位、session、遙測序號、timer／deploy、最近 ACK 與 CRC。每包可獨立解碼，不使用跨包差分壓縮。

地面站必須同時解析 v1／v2，並依最近一包有效遙測的版本送出相同版本的指令。v2 空中端只接受 v2 指令；更新順序必須是先更新地面站、確認 v1 相容，再更新空中端。

## 2. 基本規則與 common header

- 多位元組整數使用 big-endian。
- CRC 維持 CRC-16/CCITT-FALSE，涵蓋 `version` 到 payload 最後一 byte，不含 magic 與 CRC。
- 保留 flag／status bit 必須為 0。

| Offset | Bytes | 欄位 | 規則 |
| ---: | ---: | --- | --- |
| 0 | 2 | magic | `A5 5A` |
| 2 | 1 | version | `02` |
| 3 | 1 | frame_type | v1 相同 type 值 |
| 4 | 1 | flags | bit 0 `RETRANSMISSION`，其餘保留 |
| 5 | 1 | payload_length | 0–96 |
| 6 | 4 | session_id | 空中端開機 session，非 0 |
| 10 | 4 | message_id | telemetry sequence 或 command ID，非 0 |
| 14 | N | payload | 依 type |
| 14+N | 2 | crc16 | big-endian |

| Type | Payload | 完整 frame | `message_id` |
| --- | ---: | ---: | --- |
| TELEMETRY `0x01` | 47 B | 63 B | telemetry sequence |
| SET_TIMER `0x10` | 4 B | 20 B | command ID |
| FORCE_RELEASE `0x11` | 0 B | 16 B | command ID |
| ACK `0x7F` | 7 B | 23 B | 被回覆的 command ID |

## 3. TELEMETRY payload

| Offset | Bytes | 欄位 | 編碼 |
| ---: | ---: | --- | --- |
| 0 | 4 | uptime_ms | uint32 |
| 4 | 1 | restart_reason | v1 列舉 |
| 5 | 1 | status | 見下方 bit 定義 |
| 6 | 4 | remaining_s | uint32 |
| 10 | 4 | last_ack_command_id | uint32；無則 0 |
| 14 | 1 | last_ack_result | v1 列舉；無則 `FF` |
| 15 | 6 | accel XYZ | 3×int16，0.01 m/s² |
| 21 | 6 | gyro XYZ | 3×int16，0.1 deg/s |
| 27 | 8 | longitude／latitude | 2×int32，degrees × 10⁶ |
| 35 | 4 | relative_altitude | int32，cm |
| 39 | 2 | ground_speed | uint16，cm/s |
| 41 | 2 | vertical_velocity | int16，0.1 m/s |
| 43 | 2 | pressure | uint16，0.1 hPa |
| 45 | 2 | temperature | int16，0.01 °C |

`status`：bits 0–1 timer state、bit 2 deploy、bit 3 MPU valid、bit 4 BMP valid、bit 5 GPS location fresh、bit 6 GPS speed fresh、bit 7 保留。

定點轉換採四捨五入且中點遠離零。若同一感測器群組任一值為 NaN、Infinity 或超出編碼量程，該群組有效 bit 清除且該組所有欄位傳 0，禁止飽和、回繞或仍標示有效。

## 4. 指令與 ACK

SET_TIMER payload 維持 4-byte `duration_s`；FORCE_RELEASE 無 payload。ACK payload 為 `acked_type(1)、result(1)、status(1)、remaining_s(4)`，status 只允許 timer／deploy bits，sensor 與保留 bits 固定 0。

v1 的 stale、duplicate、conflict、session、DEPLOYED 鎖定與 ACK result 規則全部沿用。重送維持相同 `message_id`，不再配置每次實體重送的獨立 frame sequence；地面站以本地 `attempts` 記錄實際傳送次數。
