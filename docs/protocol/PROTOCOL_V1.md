# TASA RTC 2026 LoRa 雙向封包協定 v1

狀態：P0-02 基準規格
日期：2026-07-15

## 1. 適用範圍

本協定定義 E22-900T22D 透明傳輸模式承載的空中端與地面站二進位封包。v1 支援：

- 空中端遙測 `TELEMETRY`
- 地面站覆蓋倒數 `SET_TIMER`
- 地面站強制釋放 `FORCE_RELEASE`
- 空中端指令回覆 `ACK`
- 空中端重啟辨識、CRC、指令冪等與舊指令防護

E22 半雙工傳輸時窗不在本文件定案，屬於 P0-03；本文件提供固定封包長度供 P0-03 計算 airtime。

CRC 只用來偵測傳輸損壞，不提供加密或身分驗證。E22 的頻道、位址與 NETID 仍須依 P0-01 確認；若日後要求密碼學驗證，必須升版協定。

## 2. 基本編碼規則

- 多位元組整數：network byte order（big-endian）。
- 浮點數：IEEE-754 binary32（float32），big-endian。
- `uint8`、`uint16`、`uint32`：分別為 1、2、4 bytes 的無號整數。
- v1 最大 payload 為 96 bytes，最大完整 frame 為 118 bytes。
- 保留欄位與保留 flag bit 發送時必須為 `0`；接收端遇到非零保留 bit 必須拒收。

## 3. Common header

所有封包都有固定 20-byte header，payload 後接 2-byte CRC。

| Offset | Bytes | 欄位 | v1 規則 |
| ---: | ---: | --- | --- |
| 0 | 2 | `magic` | 固定 `A5 5A` |
| 2 | 1 | `version` | 固定 `01` |
| 3 | 1 | `frame_type` | 見封包類型表 |
| 4 | 1 | `flags` | bit 0：`RETRANSMISSION`；bit 1–7 保留 |
| 5 | 1 | `header_length` | 固定 `14` hex，即 20 bytes |
| 6 | 2 | `payload_length` | payload byte 數，不含 header／CRC |
| 8 | 4 | `session_id` | 空中端本次開機 session，禁止為 0 |
| 12 | 4 | `frame_seq` | 依 frame type 分流的傳輸序號，規則見 8.2 |
| 16 | 4 | `command_id` | 指令及 ACK 的冪等 ID；遙測固定為 0 |
| 20 | N | `payload` | 依 `frame_type` 定義 |
| 20+N | 2 | `crc16` | CRC-16/CCITT-FALSE，big-endian |

完整封包長度：

```text
total_length = 20 + payload_length + 2
```

### 3.1 封包類型與固定長度

| 值 | 名稱 | 方向 | Payload | 完整 frame |
| ---: | --- | --- | ---: | ---: |
| `0x01` | `TELEMETRY` | 空中端 → 地面站 | 72 bytes | 94 bytes |
| `0x10` | `SET_TIMER` | 地面站 → 空中端 | 4 bytes | 26 bytes |
| `0x11` | `FORCE_RELEASE` | 地面站 → 空中端 | 0 bytes | 22 bytes |
| `0x7F` | `ACK` | 空中端 → 地面站 | 8 bytes | 30 bytes |

## 4. CRC

使用 CRC-16/CCITT-FALSE：

| 參數 | 值 |
| --- | --- |
| Width | 16 |
| Polynomial | `0x1021` |
| Initial value | `0xFFFF` |
| RefIn / RefOut | false / false |
| XorOut | `0x0000` |
| 檢查字串 `123456789` | `0x29B1` |

CRC 覆蓋從 `version`（offset 2）到 payload 最後一個 byte；不含 `magic` 與最後的 CRC 本身。

```text
crc_input = frame[2 : 20 + payload_length]
```

CRC 錯誤、截斷、無效 version 或不合理長度的 frame 不得執行，也不得回 ACK，因為 header 中的 `session_id`／`command_id` 不可信。

## 5. TELEMETRY payload（72 bytes）

| Payload offset | Bytes | 欄位 | 單位／規則 |
| ---: | ---: | --- | --- |
| 0 | 4 | `uptime_ms` | 空中端開機後毫秒，uint32 自然回繞 |
| 4 | 1 | `restart_reason` | 正規化重啟原因 |
| 5 | 1 | `timer_state` | `UNSET`／`RUNNING`／`EXPIRED` |
| 6 | 1 | `deploy_state` | `SAFE`／`DEPLOYED` |
| 7 | 1 | `sensor_flags` | 感測資料有效旗標 |
| 8 | 4 | `remaining_s` | 剩餘整秒；`UNSET`／`EXPIRED` 時為 0 |
| 12 | 4 | `last_ack_command_id` | 本 session 最近一次已評估並回 ACK 的指令；無則 0 |
| 16 | 1 | `last_ack_result` | 最近 ACK 結果；無則 `0xFF` |
| 17 | 3 | `reserved` | 固定 0 |
| 20 | 4 | `accel_x_mps2` | MPU6050 X 加速度，m/s² |
| 24 | 4 | `accel_y_mps2` | MPU6050 Y 加速度，m/s² |
| 28 | 4 | `accel_z_mps2` | MPU6050 Z 加速度，m/s² |
| 32 | 4 | `gyro_x_dps` | MPU6050 X 角速度，deg/s |
| 36 | 4 | `gyro_y_dps` | MPU6050 Y 角速度，deg/s |
| 40 | 4 | `gyro_z_dps` | MPU6050 Z 角速度，deg/s |
| 44 | 4 | `longitude_deg` | GPS 經度，decimal degrees |
| 48 | 4 | `latitude_deg` | GPS 緯度，decimal degrees |
| 52 | 4 | `baro_altitude_m` | BMP280 經 5 秒平均零點與 IIR 低通後的相對高度，m |
| 56 | 4 | `ground_speed_mps` | GPS 地速，m/s |
| 60 | 4 | `vertical_velocity_mps` | 由濾波高度的 1 秒差分再低通導出的垂直速度，m/s |
| 64 | 4 | `pressure_hpa` | BMP280 氣壓，hPa |
| 68 | 4 | `temperature_c` | BMP280 溫度，°C |

### 5.1 狀態列舉

`restart_reason`：

| 值 | 名稱 |
| ---: | --- |
| `0x00` | `UNKNOWN` |
| `0x01` | `POWER_ON` |
| `0x02` | `SOFTWARE` |
| `0x03` | `WATCHDOG` |
| `0x04` | `BROWNOUT` |
| `0x05` | `PANIC` |
| `0x06` | `USB_OR_JTAG` |
| `0xFF` | `OTHER` |

`timer_state`：

| 值 | 名稱 | `remaining_s` |
| ---: | --- | --- |
| `0x00` | `UNSET` | 0 |
| `0x01` | `RUNNING` | 大於 0 |
| `0x02` | `EXPIRED` | 0 |

`deploy_state`：`0x00 SAFE`、`0x01 DEPLOYED`。一旦進入 `DEPLOYED`，同一 session 內不得回到 `SAFE`。

`sensor_flags`：

| Bit | Mask | 意義 |
| ---: | ---: | --- |
| 0 | `0x01` | MPU6050 數值有效 |
| 1 | `0x02` | BMP280 數值有效 |
| 2 | `0x04` | GPS location 有效且未過期 |
| 3 | `0x08` | GPS speed 有效且未過期 |
| 4–7 |  | 保留，固定 0 |

無效感測欄位發送 `+0.0`，接收端必須依 `sensor_flags` 判斷，不得把 0 當作有效值。感測器資料只能遙測，不得觸發開傘。

## 6. 指令 payload

### 6.1 SET_TIMER（4 bytes）

| Offset | Bytes | 欄位 | 規則 |
| ---: | ---: | --- | --- |
| 0 | 4 | `duration_s` | 新倒數整秒，必須大於 0 |

成功接收後直接覆蓋目前剩餘時間。`duration_s == 0` 回 `BAD_PAYLOAD`；立即釋放只能使用 `FORCE_RELEASE`。

### 6.2 FORCE_RELEASE（0 bytes）

沒有 payload。payload 非 0 bytes 時回 `BAD_PAYLOAD`。成功後進入 `DEPLOYED`，伺服動作最多執行一次。

## 7. ACK payload（8 bytes）

ACK common header 的 `session_id` 是空中端目前 session，`command_id` 必須複製被回覆的指令 ID。

| Offset | Bytes | 欄位 | 規則 |
| ---: | ---: | --- | --- |
| 0 | 1 | `acked_type` | 被回覆的指令 type |
| 1 | 1 | `result` | ACK 結果碼 |
| 2 | 1 | `timer_state` | 指令處理後狀態快照 |
| 3 | 1 | `deploy_state` | 指令處理後狀態快照 |
| 4 | 4 | `remaining_s` | 指令處理後剩餘秒數 |

`result`：

| 值 | 名稱 | 是否執行狀態變更 |
| ---: | --- | --- |
| `0x00` | `EXECUTED` | 是，且只執行一次 |
| `0x01` | `DUPLICATE` | 否；先前相同指令已成功執行 |
| `0x02` | `ALREADY_DEPLOYED` | 否；新 FORCE 指令到達時已釋放 |
| `0x03` | `STALE_COMMAND` | 否；比最近處理 ID 更舊 |
| `0x10` | `BAD_PAYLOAD` | 否 |
| `0x11` | `SESSION_MISMATCH` | 否；ACK header 回報目前 session |
| `0x12` | `UNSUPPORTED_TYPE` | 否 |
| `0x13` | `INVALID_STATE` | 否，例如已釋放後設定 timer |
| `0x14` | `COMMAND_ID_CONFLICT` | 否；同 ID 卻有不同 type／payload |
| `0xFF` | `NONE` | 只用於尚無 ACK 的遙測，不得作為 ACK frame 結果 |

地面站只接受 `session_id` 與目前空中端相同、`command_id` 與目前待送指令相同的 ACK。舊 session、舊 ID 或未知 ACK 只記錄，不得改變 UI 指令狀態。

## 8. Session、序號與防重複規則

### 8.1 session_id

- 空中端每次開機產生新的非零隨機 uint32 `session_id`。
- 同一次開機的 `TELEMETRY` 與 `ACK` 使用同一值。
- 地面站指令必須填入最近遙測觀察到的 session。
- 空中端重啟後 timer 回到 `UNSET`，不因舊 timer 自行釋放。
- 地面站看到新 session，必須清除舊 session ACK 判斷，並將最新 timer 設定列為最高優先新指令。

### 8.2 frame_seq

- `frame_seq` 依 frame type 使用獨立計數器，第一包建議為 1。
- `TELEMETRY.frame_seq` 只在產生新遙測時遞增；ACK 不得占用遙測序號，因此可直接用來計算掉包。
- `ACK.frame_seq` 使用獨立的空中端 ACK 計數器。
- 地面站指令使用獨立計數器；每 0.1 秒實際重送時取得新的 `frame_seq`，並設定 `RETRANSMISSION`。
- 各計數器回繞均按 uint32 處理。
- 掉包率只以同一 `session_id` 的 `TELEMETRY.frame_seq` 計算；ACK 與地面站指令不混入遙測掉包統計。

### 8.3 command_id

- 每個 air session 內，地面站對新邏輯指令配置嚴格遞增、非零的 `command_id`。
- 同一指令的所有重送保留相同 `command_id`、type 與 payload。
- 新 timer 值必須取得新 `command_id`；舊待送 timer 立即淘汰。
- 強制釋放取得比所有既有待送指令新的 ID，並擁有佇列最高優先權。
- v1 不允許同一 session 內 `command_id` 回繞；若耗盡則停止發指令並回報錯誤。
- 地面站程式重啟但空中端未重啟時，以下一個大於遙測 `last_ack_command_id` 的 ID 繼續。

空中端處理順序：

1. CRC、長度、version 或保留 bit 不合法：丟棄，不回 ACK。
2. `session_id` 不符：不改狀態，回 `SESSION_MISMATCH`，ACK header 使用目前 session。
3. ID 等於最近處理 ID且 type／payload 相同：不再執行；若原結果為 `EXECUTED` 則回 `DUPLICATE`，否則重送原本的 ACK 結果。
4. ID 等於最近處理 ID但 type／payload 不同：不執行，回 `COMMAND_ID_CONFLICT`。
5. ID 小於最近處理 ID：不執行，回 `STALE_COMMAND`。
6. ID 較新：驗證 payload／狀態、最多執行一次、記錄結果並回 ACK。

空中端須保存最近處理指令的 ID、type、payload fingerprint 與原始 ACK 結果，讓相同重送可得到確定回覆；發出 `DUPLICATE` 時不得覆蓋保存的原始執行結果。`last_ack_command_id`／`last_ack_result` 回報最近一個目前 session 實際送出的 ACK；`SESSION_MISMATCH` 不寫入這兩欄。

## 9. Stream parser 與錯誤恢復

E22 UART 資料可能分段到達，parser 必須保留未完成 bytes：

1. 搜尋 `A5 5A`。
2. 未滿 20 bytes 時等待後續資料。
3. 讀取 `header_length`／`payload_length`，拒絕非 v1 固定 header 或 payload 大於 96。
4. 未滿 `20 + payload_length + 2` 時等待。
5. 驗證 CRC，再驗證該 type 的固定 payload 長度及欄位規則。
6. 合法 frame 完整消耗；錯誤 frame 從候選 magic 的下一個 byte 重新搜尋，避免失去後續合法 frame。

未知的 v1 `frame_type` 若結構與 CRC 合法，空中端可依可信 header 回 `UNSUPPORTED_TYPE`；地面站收到未知 type 則記錄並忽略。

## 10. Golden test vectors

完整 hex 與語意欄位位於 [`test_vectors_v1.json`](./test_vectors_v1.json)。

| Vector | 預期 |
| --- | --- |
| `telemetry_nominal` | 94-byte 遙測正常解析 |
| `set_timer_nominal` | 30 秒 timer，執行一次 |
| `set_timer_retry_duplicate` | 同 command ID、不同 frame seq，僅回 `DUPLICATE` |
| `force_release_nominal` | 0-byte payload 正常解析 |
| `ack_executed_nominal` | `SET_TIMER` 的 `EXECUTED` ACK |
| `set_timer_bad_crc` | 最後一個 CRC bit 翻轉，回 `CRC_ERROR`，不 ACK |
| `set_timer_truncated` | 宣告 26 bytes、實際 23 bytes，回 `TRUNCATED`，不 ACK |

重跑驗證：

```powershell
python .\docs\protocol\verify_test_vectors.py
```

參考 encoder／decoder 位於 [`protocol_v1.py`](./protocol_v1.py)，只使用 Python standard library。ESP32 與地面站實作必須以同一批 golden vectors 交叉驗證。
