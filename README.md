# 2026 Taiwan Cup — 五限可能地面站監控程式

> **2026 台灣盃火箭競賽 — 五限可能地面站監控系統**

一套基於 **Tauri v2 + Svelte 5 + Rust** 的桌面應用程式，用於即時接收並視覺化火箭遙測資料（加速度、陀螺儀、GPS、氣壓、溫度等），提供地面站操作人員完整的飛行狀態監控介面。

---

## 📸 功能總覽

| 功能 | 說明 |
|------|------|
| 🔌 序列埠連線管理 | 「開始監控」先強制填寫測試資料，確認後才原子地開啟 COM、建立場次並開始接收 |
| 📡 即時遙測資料接收 | 透過序列埠持續讀取火箭下傳的二進位封包，含 CRC-16 驗證 |
| 📊 即時圖表 | 以高度軌跡為主視覺，並在同一面板顯示垂直速度、地速、氣壓與溫度 |
| 🧭 MPU6050 姿態儀 | 陀螺儀積分搭配加速度門控融合，可在 UI 調整感測器軸向 |
| 🗺️ GPS 即時地圖 | Leaflet + OpenStreetMap 顯示 M8N 位置、飛行軌跡與自動跟隨 |
| 📋 遙測數值面板 | 13 項感測器數值分類顯示（IMU / GPS / 環境） |
| 📶 狀態列 | 待命／等待資料／接收中／失聯、封包與解析失敗統計、獨立 CRC 錯誤、近期接收頻率、連線時間 |
| 💾 可靠資料記錄 | SQLite 與場次檔案由 4096 筆 FIFO writer 依序寫入，顯示正常／降級／失敗、磁碟空間與遺失寫入 |
| ⏱️ 雙向飛行控制 | Protocol v1/v2 `SET_TIMER`／`FORCE_RELEASE`、依遙測版本回送、ACK 配對與重啟 timer 同步 |
| 🗂️ 自動場次檔案 | 每次確認監控後自動建立 UUID 場次與 `flight_data.csv`、`system.log`、`session_summary.json`，不再另按「開始新場次」 |
| 📉 通訊統計 | 分開計算遺失、重複、CRC 錯誤、失聯區間、最長失聯與重啟次數 |

---

## ✅ 最新 Protocol v2 與雙向 ACK 實作

目前 portable 版本同時支援舊版 Protocol v1 與正式 Protocol v2；地面站會先等待有效 telemetry/session，再依收到的版本選擇上行 frame，不會在尚未取得 session 時發送指令。

- v2 telemetry 由 94 B 壓縮為 63 B，仍保留全部 13 個感測欄位。線上使用 big-endian 定點整數；Rust parser 會還原成既有的 m/s²、°/s、度、m、m/s、hPa 與 °C，UI／CSV／SQLite 的對外單位不變。
- v2 frame 為 14 B header + payload + CRC-16/CCITT-FALSE：`SET_TIMER` 20 B、`FORCE_RELEASE` 16 B、ACK 23 B。v2 ACK 不另配置 sequence；command ID、session ID、ACK result 與 telemetry 的 last-ACK 共同完成配對。
- 地面站在每包完整 telemetry 後 150–299 ms 只送一次；ACK 遺失時等下一包 telemetry 再重送。同一邏輯指令維持 command ID，每次傳送的 attempts 寫入 command status／場次 Log。`FORCE_RELEASE` 永遠優先，timer 只保留最新值。
- 收到新 session 時，舊 ACK 不會改變狀態；最新 timer 會以新 command ID 重建，`FORCE_RELEASE` 不跨 session 自動重放。收到 telemetry last-ACK 也能在獨立 ACK 遺失時停止重送。
- 正式空中 telemetry 週期為 1800 ms，地面站失聯門檻為 4500 ms。實測 1800 ms 三輪共 60/60 timer 首次 ACK，telemetry 遺失／重複／CRC 都是 0；ESP32 #2 USB 必須拔除並使用外部電源才能進行 RF 台架測試。

最新已驗證 portable metadata 為 [`GroundStation_0.1.0_Portable_2026-07-20_023341.json`](artifacts/GroundStation_0.1.0_Portable_2026-07-20_023341.json)，SHA-256 `176E755317C7FCD6AAAFEDE3FA4F940349F5630AD9389D9334550FC8F55094A8`。可執行檔不再提交到一般 Git 歷史，正式二進位由 [GitHub Releases](https://github.com/John-owo/-2026-TaiwanCup-Rocket-Ground-Station/releases) 發布。Protocol v1/v2 共用測試向量位於工作區根目錄 `protocol/`。

---

## 🏗️ 技術架構

```
┌──────────────────────────────────────────────────────┐
│                   Tauri v2 桌面應用                    │
├────────────────────┬─────────────────────────────────┤
│   Rust 後端         │      Svelte 5 前端              │
│   (src-tauri/)     │      (src-ui/)                  │
│                    │                                 │
│  Commands ←IPC→    │  Tauri API Wrappers             │
│  Services (Traits) │  Reactive Stores ($state)       │
│  Infrastructures   │  UI Components (7 個)            │
│  State (Mutex)     │  SVG Charts / Instruments       │
│  Models            │  Mission Console Dark Theme     │
└────────────────────┴─────────────────────────────────┘
         ↕ Serial Port (USB / Radio)
   ┌─────────────┐
   │  🚀 火箭     │
   │  (遙測下傳)   │
   └─────────────┘
```

### 後端分層架構

```
Commands (Tauri IPC 指令) → Services (Trait 定義 / 抽象層) → Infrastructures (實作 / I/O)
                                                            ↕
                                                      State (共享狀態)
                                                            ↕
                                                      Models (資料型別)
```

### 資料流

```
Serial Port (COM) → SerialReceiver.receive_task()
  → 逐 byte 讀取 → PacketParser.sink(byte)
    → 搜尋 A5 5A → 依 version 選擇 20-byte v1／14-byte v2 Header
      → CRC-16/CCITT-False 驗證（version 到 payload 結尾）
      → TelemetryDecoder 解碼 v1 float32 或 v2 fixed-point，還原相同 13 個物理量
        → TelemetryPayload
          → session 改變時發送 "airborne-session-changed"
          → 發送 "update-telemetry" 事件到前端
          → 背景儲存到 SQLite
          → 發送 "packet-stats" 封包統計事件
```

---

## 📁 專案結構與程式說明

### 根目錄

| 檔案 | 說明 |
|------|------|
| `.gitignore` | Git 忽略規則：忽略 `target/`、`node_modules/`、`dist/` 與本機 release `.exe`；`Cargo.lock`、`pnpm-lock.yaml` 及 release JSON metadata 必須追蹤 |
| `README.md` | 本說明文件 |

---

### `src-tauri/` — Rust 後端（Tauri 核心）

#### 設定檔

| 檔案 | 說明 |
|------|------|
| `Cargo.toml` | Rust 套件設定，相依套件：`tauri`、`tokio`、`tokio-serial`、`sqlx` (SQLite)、`serde` 等 |
| `tauri.conf.json` | Tauri 應用設定：應用名稱 `rocket-monitoring-system`、視窗大小 1200×800、前端路徑、開發伺服器 port 8000 |
| `build.rs` | Tauri 建置腳本，呼叫 `tauri_build::build()` |

#### 核心程式

| 檔案 | 說明 |
|------|------|
| `src/main.rs` | **應用程式進入點**。呼叫 `app_lib::run()` 啟動 Tauri 應用。Release 模式下隱藏 Windows 控制台視窗 |
| `src/lib.rs` | **核心應用邏輯**。同步註冊 `SerialState`／`StorageState`、在背景初始化持久化儲存、啟用 Release 日誌、註冊原子場次指令，並在程式退出時將進行中場次安全標記為 `interrupted`。SQLite 初始化失敗不會再靜默改用記憶體資料庫。 |

#### `src/commands/` — Tauri IPC 指令層

| 檔案 | 說明 |
|------|------|
| `mod.rs` | 模組宣告，匯出 `serial` 模組 |
| `serial.rs` | **序列埠與場次協調器**。`start_test_monitoring`／`stop_test_monitoring` 原子地協調 COM、UUID 場次、writer 與 receiver，另提供儲存／場次狀態、timer、強制釋放及依場次查詢歷史資料。 |

#### `src/services/` — 服務層（Trait 定義 / 抽象介面）

| 檔案 | 說明 |
|------|------|
| `mod.rs` | 模組宣告，匯出 `serial` 和 `notify` 模組 |
| `serial.rs` | **序列埠抽象層**。定義 v1/v2 `Parser`、`Receiver` 與完整 frame `Decoder`。 |
| `notify.rs` | **通知服務** (`NotificationCenter`)。封裝 Tauri 的事件發送 API：<br>• `broadcast_error(error)` — 發送 `"serial-error"` 事件<br>• `update_telemetry(payload)` — 發送 `"update-telemetry"` 事件<br>• `update_stats(total, failed, packets_per_second)` — 發送 `"packet-stats"` 事件 |

#### `src/infrastructures/` — 基礎設施層（具體實作）

| 檔案 | 說明 |
|------|------|
| `mod.rs` | 模組宣告，匯出 `serial` 模組 |
| `serial/crc.rs` | **CRC-16/CCITT-False 校驗碼計算**。函式 `crc16_ccitt(bit_stream) → u16`，多項式 `0x1021`、初始值 `0xFFFF`、MSB-first。用於驗證封包完整性 |
| `serial/parser.rs` | **Protocol v1/v2 stream parser**。搜尋 `A5 5A`，依 version 驗證長度、保留 bits 與 CRC；v1 解 float32，v2 解 fixed-point 並還原相同物理單位。測試同時載入 v1/v2 golden vectors。 |
| `serial/receiver.rs` | **序列埠接收器** (`SerialReceiver`)。COM 先開啟但要等場次建立成功才啟動；遙測與事件以非阻塞方式加入單一 FIFO writer，不再每包各自 `spawn`。queue 滿載會立即回報儲存失敗並累計遺失寫入，但不阻斷遙測與安全控制。 |

#### `src/models/` — 資料模型

| 檔案 | 說明 |
|------|------|
| `mod.rs` | 模組宣告，匯出 `response` 模組 |
| `response.rs` | **資料傳輸型別**：<br>• `TelemetryPayload` — Protocol v1/v2 version／session／sequence／uptime／restart／timer／deploy／ACK metadata + 13 個已還原感測欄位<br>• `AirborneSessionChanged` — session 第一次出現或改變事件<br>• `InvokeError` — 錯誤列舉<br>• `DbTelemetry` — 13 個感測欄位歷史記錄 |

#### `src/state/` — 狀態管理

| 檔案 | 說明 |
|------|------|
| `mod.rs` | 模組宣告，匯出 `SerialState` 與 `StorageState` |
| `serial_state.rs` | **執行緒安全序列／場次狀態**：保存 COM、取消權、指令 channel、統計與後端唯一的 `TestSessionStatus`。 |
| `storage_state.rs` | **可靠儲存 actor**：4096 筆 FIFO、SQLx migration、SQLite／CSV／Log 雙 sink、磁碟檢查、正常／降級／失敗狀態與啟動復原。 |

---

### `src-ui/` — Svelte 5 前端

#### 設定檔

| 檔案 | 說明 |
|------|------|
| `index.html` | HTML 進入點。語言設為 `zh-TW`，載入 Google Fonts (Inter + JetBrains Mono)，掛載 `main.ts` |
| `package.json` | Node.js 套件設定，相依：Svelte 5、Vite 8、Tauri API v2 |
| `vite.config.ts` | Vite 設定：Svelte 外掛、開發伺服器 port 8000 (strict)、路徑別名 `@` → `src/` |
| `svelte.config.js` | Svelte 設定：使用預設值 |
| `tsconfig*.json` | TypeScript 設定檔 |

#### 核心程式

| 檔案 | 說明 |
|------|------|
| `src/main.ts` | **前端進入點**。使用 Svelte 5 的 `mount()` 將 `App` 元件掛載到 `#app`，匯入全域 CSS |
| `src/app.css` | **全域設計系統 & 樣式表**。低彩度深色任務控制台主題，使用單一薄荷綠主色、琥珀與紅色狀態色、Bahnschrift／微軟正黑體及 Cascadia Code；包含一致的間距、圓角、焦點樣式與減少動態效果支援。 |
| `src/App.svelte` | **根元件**。三欄式監控版面：<br>• **頂部列** — 5 SPACE 隊徽、目前測試場次、儲存／記錄／連線狀態<br>• **左側邊欄** — `ConnectionPanel`<br>• **中央區域** — `TelemetryGrid` + `TelemetryCharts` + 精簡 `AttitudeIndicator`<br>• **右側邊欄** — 上方 `GpsMap`、下方 `FlightControlPanel`<br>• **底部** — `StatusBar` |

#### `src/lib/` — 共用程式庫

| 檔案 | 說明 |
|------|------|
| `types.ts` | **TypeScript 型別定義**：遙測、封包統計、資料庫記錄，以及持久化的軸向與連線設定 |
| `tauri.ts` | **Tauri IPC 橋接層**：原子開始／停止測試監控、查詢場次與儲存狀態、掃描序列埠、依場次查詢歷史資料與註冊事件 |
| `stores.svelte.ts` | **Svelte 5 響應式狀態管理**：遙測快照、每封包 revision、最多 200 筆圖表資料、連線狀態，以及 COM／Baud／軸向持久化設定 |
| `attitude.js` | **姿態估算器**：角速度以 `deg/s` 直接積分；總加速度接近 1g 時以互補濾波修正 Roll/Pitch，高動態時退回陀螺儀積分；Yaw 為相對航向 |
| `settings.js` | **設定驗證與保存**：使用 `rocket-ground-station.settings.v1` 保存 COM Port、Baud Rate 與三軸來源／方向 |
| `gps-map.js` | **GPS 純邏輯**：座標驗證、Haversine 距離、2 公尺軌跡門檻與 5,000 點上限 |

#### `src/components/` — UI 元件

| 檔案 | 說明 |
|------|------|
| `ConnectionPanel.svelte` | **連線與校正面板**（左側邊欄）：開始監控時只提出強制場次視窗；視窗確認後才由後端連線。另提供掃描／保存 COM、Baud 與姿態軸向設定。 |
| `TestSessionDialog.svelte` | **不可略過的測試資料視窗**：必填目的、操作者、地點、起始電壓；只記住操作者與地點。儲存失敗時必須明確承認才可僅監控、不記錄。 |
| `TelemetryGrid.svelte` | **遙測數值格狀面板**（中央上方）。13 項數值依類別顯示：<br>• **IMU 感測器**（青色）：加速度 X/Y/Z (m/s²)、角速度 X/Y/Z (°/s)<br>• **GPS / 導航**（綠色）：經度、緯度、高度 (m)、地速、垂直速度<br>• **環境**（橙色）：氣壓 (hPa)、溫度 (°C)<br>• 告警機制：超過 `warnThreshold` 橙色邊框、超過 `critThreshold` 紅色脈衝動畫<br>• 交錯 slide-up 入場動畫 |
| `TelemetryCharts.svelte` | **即時遙測圖表**（中央）：以純 SVG 繪製最近 100 筆相對高度面積線圖，旁列即時垂直速度、地速、氣壓與溫度 |
| `GpsMap.svelte` | **GPS 即時地圖**（右上）：Leaflet/OpenStreetMap 圖磚、火箭標記、最多 5,000 點軌跡、自動跟隨、定位火箭與清除軌跡。斷網時 GPS 數值與其他遙測仍持續更新 |
| `AttitudeIndicator.svelte` | **精簡姿態列**（中央底部）：保留火箭姿態示意、滾轉／俯仰／偏航與三軸角速度。每個新封包只更新一次；軸向變更或按下「姿態歸零」會重設姿態 |
| `StatusBar.svelte` | **底部狀態列**。水平分隔顯示：<br>• 📡 待命／等待資料／接收中／失聯四態（4500 ms 門檻）<br>• 📦 總封包數<br>• ⚠️ 解析失敗率與獨立 CRC 錯誤數<br>• ⏱ 10 秒短窗有效 telemetry 接收頻率 (Hz)<br>• 🕐 連線時間 (HH:MM:SS，每秒更新) |

---

## 🖥️ UI 配置

桌面版採三欄配置：左側為序列連線與軸向設定，中央以高度、關鍵遙測與精簡火箭姿態為主，右側固定 GPS 地圖與安全控制。開始監控時的測試資料表單使用置中的原生 modal，未確認前不會開啟 COM 或建立場次。

---

## 📡 遙測封包格式

正式空中端每 1.8 秒使用 Protocol v2 `TELEMETRY`（63 bytes），地面站仍可解析 94-byte v1；地面站失聯門檻為正式週期 2.5 倍的 4.5 秒：

```
┌─────────────────────┬─────────────────────────────┬─────────────┐
│ Common header       │ TELEMETRY payload             │ CRC         │
│ A5 5A + 12 bytes    │ 15B metadata + 32B fixed-point│ CRC-16 BE   │
│ 14 bytes            │ 47 bytes                      │ 2 bytes     │
└─────────────────────┴─────────────────────────────┴─────────────┘
```

v2 header 包含 version、type、payload length、session ID 與 `message_id`；telemetry 的 `message_id` 即 sequence。Payload 使用定點整數傳輸，Rust parser 會還原為下列既有單位：

| 索引 | 欄位 | 說明 | 單位 |
|------|------|------|------|
| 0 | x_acceleration | X 軸加速度 | m/s² |
| 1 | y_acceleration | Y 軸加速度 | m/s² |
| 2 | z_acceleration | Z 軸加速度 | m/s² |
| 3 | x_angular_velocity | X 軸角速度 | °/s |
| 4 | y_angular_velocity | Y 軸角速度 | °/s |
| 5 | z_angular_velocity | Z 軸角速度 | °/s |
| 6 | longitude | 經度 | ° |
| 7 | latitude | 緯度 | ° |
| 8 | altitude | 高度 | m |
| 9 | ground_speed | 地速 | m/s |
| 10 | vertical_velocity | 垂直速度 | m/s |
| 11 | air_pressure | 氣壓 | hPa |
| 12 | temperature | 溫度 | °C |

### CRC 驗證

- 演算法：**CRC-16/CCITT-False**
- 多項式：`0x1021`
- 初始值：`0xFFFF`
- 位元順序：MSB-first（非反射）
- 計算範圍：從 common header 的 `version`（offset 2）到 payload 最後一個 byte
- CRC 位元組順序：Big-Endian

完整欄位、定點尺度、結果碼與 golden vectors 以 [`../protocol/PROTOCOL_V2.md`](../protocol/PROTOCOL_V2.md) 為準；v1 相容格式仍見 [`../protocol/PROTOCOL_V1.md`](../protocol/PROTOCOL_V1.md)。

---

## 🎛️ 操作與限制

### P0 timer、強制釋放與場次紀錄

- 地面站只在有待執行指令時，於每包完整 telemetry 後 150–299 ms 內送一次；未收到 ACK 才等下一包重送。同一邏輯指令維持 command ID；v1 保留實體 frame sequence，v2 以本地 attempts 記錄重送。
- 新 timer 會淘汰尚未 ACK 的舊 timer；`FORCE_RELEASE` 永遠優先於 timer。
- 偵測到空中端 session 改變時，舊 ACK 不會改變狀態，最新 timer 會以新 command ID 自動重建。FORCE 指令基於安全考量不跨 session 自動重放。
- 強制釋放按鈕平時鎖定；只有最近 4500 ms 內收到有效空中端 telemetry/session 時才能解除，失聯、斷線或 session 變更立即重新上鎖。解除後單擊即送出並自動重新上鎖，不使用長按；後端會再次驗證 live session，且不會保存無 session 的 FORCE 等待日後重播。
- 按「開始監控」後必須先填寫測試目的、起始電池電壓、地點與操作者（備註選填）；取消不會開啟 COM 或建立資料夾。確認後自動建立場次，不需另按開始。只會記住操作者與地點。
- SQLite 位於 Tauri 顯示的應用程式資料路徑下之 `telemetry.db`；Windows 一般位於 `%APPDATA%\com.taiwancup.rocketground\`。每場檔案位於同一路徑的 `flight_sessions\<timestamp>_<UUID>_<location>\`，介面會顯示該場完整路徑。
- 儲存狀態為失敗時預設禁止正式開始；操作員勾選了解資料不會保存後才可進入僅監控模式。系統不使用記憶體 SQLite 冒充永久保存，也不會自動刪除舊資料。
- 剩餘空間低於 512 MiB 顯示警告／降級，低於 128 MiB 禁止新正式場次；啟動、開始場次前與每 30 秒重查。SQLite 單一 sink 失敗時仍以 CSV／Log 保存並標示降級，兩者都失敗或 queue 滿載時標示失敗。
- 操作員按中斷會排空 writer、完成摘要並標記 `completed`；序列異常、程式關閉或上次當機未收尾會標記 `interrupted`。空中端遙測暫時失聯不會自行結束場次。
- `flight_data.csv` 包含 Protocol v1/v2 metadata、13 個已還原感測欄位與當下統計；`system.log` 記錄連線、失聯、CRC、指令、ACK、重啟、DEPLOYED 與錯誤。

以上自動測試不取代 E22 半雙工、伺服與實際開傘機構的實機驗證。

### 發射前姿態校正

1. 讓 MPU6050 靜止，確認三軸角速度接近零。
2. 在姿態卡按「歸零」。
3. 每次只繞一個實體軸轉動，確認畫面主要變化的是預期的滾轉、俯仰或偏航。
4. 若軸向錯誤，在左側「姿態軸向設定」交換來源軸；若方向相反，切換正向／反向。
5. MPU6050 沒有磁力計，因此相對航向會隨時間漂移，不能當成真北航向。

### GPS 地圖

- M8N 經緯度有效後，右上地圖會顯示火箭位置並開始記錄軌跡。
- `(0, 0)`、非有限值或超出經緯度範圍的資料視為尚未定位。
- 地圖需要網路；圖磚失敗不會停止 GPS 數值、序列埠或其他遙測。
- 使用 OpenStreetMap 標準圖磚並保留 `© OpenStreetMap contributors` attribution。
- 不提供背景預抓、批次下載或離線圖磚，避免違反 OpenStreetMap 公共圖磚政策。

### 設定保存

- COM Port、Baud Rate 與姿態軸向會保存於本機。
- 程式重開後只恢復設定，不會自動連線。
- 若已保存的 COM Port 當下不存在，介面會保留該值並提示確認裝置。

---

## 🛠️ 環境需求

| 工具 | 最低版本 | 用途 |
|------|----------|------|
| Node.js | ≥ 18 | 前端建置 |
| pnpm | 最新版 | 套件管理 |
| Rust | ≥ 1.77.2 | 後端編譯 |
| Tauri CLI | v2 | 應用建置 |

### 安裝 Rust

前往 [rustup.rs](https://rustup.rs/) 下載安裝，或在終端機執行：

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 安裝 pnpm

```bash
npm install -g pnpm
```

### 安裝 Tauri CLI

```bash
pnpm add -g @tauri-apps/cli
```

---

## 🚀 開始開發

E22、M8N、伺服與 UI 的實機步驟見工作區 [`../P0_HARDWARE_TEST_PLAN.md`](../P0_HARDWARE_TEST_PLAN.md)。

### 1. 安裝前端相依套件

```bash
cd src-ui
pnpm install --frozen-lockfile
```

### 2. 啟動開發模式

從 `ground_station/` 執行：

```powershell
& ".\src-ui/node_modules/.bin/tauri.CMD" dev
```

這會同時啟動：
- **Vite 開發伺服器** (port 8000) — 前端熱重載
- **Tauri 原生視窗** — Rust 後端

### 3. 驗證與建置正式版本

前端驗證：

```powershell
cd .\src-ui
npm test
npm run check
npm run build
```

Rust 驗證：

```powershell
cd ..\src-tauri
cargo test --locked
cargo check --locked
```

建置目前工作區的 Windows release 殼時，從 `ground_station/` 執行：

```powershell
& ".\src-ui/node_modules/.bin/tauri.CMD" build --no-bundle --config .\src-tauri\tauri.workspace-build.json
```

2026-07-16 已驗證的 no-bundle 產物位於 `src-tauri/target/release/app.exe`。若需要 installer／bundle，另行執行正式 bundle build 並保存該次驗證結果，不要把 no-bundle 結果當成 installer 已驗證。

正式發行前需保存上述測試、型別檢查、前端 production build 與 Tauri release build 的通過結果。通過後，可將 `src-tauri/target/release/app.exe` 以 `artifacts/GroundStation_<version>_Portable_<YYYY-MM-DD_HHMMSS>.exe` 暫存在本機，建立同名 `.json` 驗證清單／SHA-256 並更新 `artifacts/LATEST.txt`。`artifacts/*.exe` 一律不提交 Git；正式版本須把 `.exe` 與驗證清單上傳到 GitHub Releases。版本庫只保留 JSON/checksum、`LATEST.txt` 與 release URL metadata，未通過驗證的本機產物不得標記或發布為最新版。既有 Git 歷史不在一般修正版中重寫。

---

## 📜 授權條款

本專案為 2026 台灣盃火箭競賽 五限可能 內部使用。
