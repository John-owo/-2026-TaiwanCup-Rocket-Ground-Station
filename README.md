# 2026 Taiwan Cup — 五限可能地面站監控程式

> **2026 台灣盃火箭競賽 — 五限可能地面站監控系統**

一套基於 **Tauri v2 + Svelte 5 + Rust** 的桌面應用程式，用於即時接收並視覺化火箭遙測資料（加速度、陀螺儀、GPS、氣壓、溫度等），提供地面站操作人員完整的飛行狀態監控介面。

---

## 📸 功能總覽

| 功能 | 說明 |
|------|------|
| 🔌 序列埠連線管理 | 掃描並選擇 COM Port 與 Baud Rate，設定會保留但啟動時不自動連線 |
| 📡 即時遙測資料接收 | 透過序列埠持續讀取火箭下傳的二進位封包，含 CRC-16 驗證 |
| 📊 即時圖表 | 高度、垂直速度兩種時間序列圖表即時繪製 |
| 🧭 MPU6050 姿態儀 | 陀螺儀積分搭配加速度門控融合，可在 UI 調整感測器軸向 |
| 🗺️ GPS 即時地圖 | Leaflet + OpenStreetMap 顯示 M8N 位置、飛行軌跡與自動跟隨 |
| 📋 遙測數值面板 | 13 項感測器數值分類顯示（IMU / GPS / 環境） |
| 📶 狀態列 | 連線狀態、封包統計、CRC 錯誤率、接收頻率、連線時間 |
| 💾 資料庫記錄 | 遙測資料自動儲存至 SQLite，可查詢歷史紀錄 |
| ⏱️ 雙向飛行控制 | Protocol v1/v2 `SET_TIMER`／`FORCE_RELEASE`、依遙測版本回送、ACK 配對與重啟 timer 同步 |
| 🗂️ 場次檔案 | 每場建立 `flight_data.csv`、`system.log`、`session_summary.json` 並逐筆 flush |
| 📉 通訊統計 | 分開計算遺失、重複、CRC 錯誤、失聯區間、最長失聯與重啟次數 |

---

## ✅ 最新 Protocol v2 與雙向 ACK 實作

目前 portable 版本同時支援舊版 Protocol v1 與正式 Protocol v2；地面站會先等待有效 telemetry/session，再依收到的版本選擇上行 frame，不會在尚未取得 session 時發送指令。

- v2 telemetry 由 94 B 壓縮為 63 B，仍保留全部 13 個感測欄位。線上使用 big-endian 定點整數；Rust parser 會還原成既有的 m/s²、°/s、度、m、m/s、hPa 與 °C，UI／CSV／SQLite 的對外單位不變。
- v2 frame 為 14 B header + payload + CRC-16/CCITT-FALSE：`SET_TIMER` 20 B、`FORCE_RELEASE` 16 B、ACK 23 B。v2 ACK 不另配置 sequence；command ID、session ID、ACK result 與 telemetry 的 last-ACK 共同完成配對。
- 地面站在每包完整 telemetry 後 150–299 ms 只送一次；ACK 遺失時等下一包 telemetry 再重送。同一邏輯指令維持 command ID，每次傳送的 attempts 寫入 command status／場次 Log。`FORCE_RELEASE` 永遠優先，timer 只保留最新值。
- 收到新 session 時，舊 ACK 不會改變狀態；最新 timer 會以新 command ID 重建，`FORCE_RELEASE` 不跨 session 自動重放。收到 telemetry last-ACK 也能在獨立 ACK 遺失時停止重送。
- 正式空中 telemetry 週期為 1800 ms，地面站失聯門檻為 4500 ms。實測 1800 ms 三輪共 60/60 timer 首次 ACK，telemetry 遺失／重複／CRC 都是 0；ESP32 #2 USB 必須拔除並使用外部電源才能進行 RF 台架測試。

最新已驗證發行檔：[GroundStation_0.1.0_Portable_2026-07-17_220122.exe](artifacts/GroundStation_0.1.0_Portable_2026-07-17_220122.exe)。Protocol v1/v2 共用 vectors 位於工作區根目錄 `protocol/`。

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
│  Models            │  Glassmorphism Dark Theme       │
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
| `.gitignore` | Git 忽略規則：忽略 `target/`、`node_modules/`、`dist/`、lock 檔等 |
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
| `src/lib.rs` | **核心應用邏輯**。負責：<br>• 註冊 `tauri_plugin_log` 日誌外掛<br>• 管理共享狀態 (`SerialState`)<br>• 註冊 Tauri 指令：`start_monitoring`、`stop_monitoring`、`get_telemetry_history`<br>• `init_database()` 函式：在 App Data 目錄建立 `telemetry.db` SQLite 資料庫，建立 `telemetry` 資料表（13 個感測器欄位 + id + received_at），失敗時改用記憶體 DB |

#### `src/commands/` — Tauri IPC 指令層

| 檔案 | 說明 |
|------|------|
| `mod.rs` | 模組宣告，匯出 `serial` 模組 |
| `serial.rs` | **序列埠指令處理器**。啟動／停止 Protocol v1/v2 接收迴圈、列舉序列埠、設定 timer／強制釋放及查詢歷史資料。 |

#### `src/services/` — 服務層（Trait 定義 / 抽象介面）

| 檔案 | 說明 |
|------|------|
| `mod.rs` | 模組宣告，匯出 `serial` 和 `notify` 模組 |
| `serial.rs` | **序列埠抽象層**。定義 v1/v2 `Parser`、`Receiver` 與完整 frame `Decoder`。 |
| `notify.rs` | **通知服務** (`NotificationCenter`)。封裝 Tauri 的事件發送 API：<br>• `broadcast_error(error)` — 發送 `"serial-error"` 事件<br>• `update_telemetry(payload)` — 發送 `"update-telemetry"` 事件<br>• `update_stats(total, failed)` — 發送 `"packet-stats"` 事件 |

#### `src/infrastructures/` — 基礎設施層（具體實作）

| 檔案 | 說明 |
|------|------|
| `mod.rs` | 模組宣告，匯出 `serial` 模組 |
| `serial/crc.rs` | **CRC-16/CCITT-False 校驗碼計算**。函式 `crc16_ccitt(bit_stream) → u16`，多項式 `0x1021`、初始值 `0xFFFF`、MSB-first。用於驗證封包完整性 |
| `serial/parser.rs` | **Protocol v1/v2 stream parser**。搜尋 `A5 5A`，依 version 驗證長度、保留 bits 與 CRC；v1 解 float32，v2 解 fixed-point 並還原相同物理單位。測試同時載入 v1/v2 golden vectors。 |
| `serial/receiver.rs` | **序列埠接收器** (`SerialReceiver`)。連接 COM port，執行主接收迴圈：<br>• `get_connection(path, baud_rate)` — 使用 tokio_serial 開啟非同步序列埠<br>• `receive_task()` — 逐 byte 餵入 Parser；遙測成功時發送前端事件並背景存入 SQLite；解析失敗時累計錯誤<br>• session ID 第一次出現或改變時發送 `airborne-session-changed`<br>• `emit_stats()` — 發送封包統計事件<br>• `save_to_database()` — 在背景 `tokio::spawn` 中 INSERT 到 SQLite（不阻塞接收迴圈） |

#### `src/models/` — 資料模型

| 檔案 | 說明 |
|------|------|
| `mod.rs` | 模組宣告，匯出 `response` 模組 |
| `response.rs` | **資料傳輸型別**：<br>• `TelemetryPayload` — Protocol v1/v2 version／session／sequence／uptime／restart／timer／deploy／ACK metadata + 13 個已還原感測欄位<br>• `AirborneSessionChanged` — session 第一次出現或改變事件<br>• `InvokeError` — 錯誤列舉<br>• `DbTelemetry` — 13 個感測欄位歷史記錄 |

#### `src/state/` — 狀態管理

| 檔案 | 說明 |
|------|------|
| `mod.rs` | 模組宣告，匯出並重新導出 `serial_state::*` |
| `serial_state.rs` | **執行緒安全共享狀態** (`SerialState`)：<br>• `path: Mutex<Option<String>>` — COM port 路徑<br>• `baud_rate: Mutex<Option<u32>>` — 鮑率<br>• `cancellation_token: Mutex<Option<CancellationToken>>` — 控制接收迴圈生命週期<br>• `verification_failed_count: Arc<Mutex<u32>>` — CRC 驗證失敗計數<br>• `total_packet_count: Arc<Mutex<u64>>` — 總封包計數<br>• `DbPool(pub sqlx::SqlitePool)` — SQLite 連線池 newtype 包裝 |

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
| `src/app.css` | **全域設計系統 & 樣式表**。深色航太風格主題：<br>• CSS 自訂屬性：深色背景 (`#0a0e1a`)、霓虹強調色（Cyan、Green、Orange、Red + dim/glow 變體）<br>• 字體：Inter (sans-serif) + JetBrains Mono (monospace)，8 級字體大小<br>• Glassmorphism 效果：半透明背景、模糊、陰影<br>• 動畫：`pulse`、`fade-in`、`slide-up`、`glow`、`scan-line` 等<br>• 自訂捲軸、Utility Classes |
| `src/App.svelte` | **根元件**。三欄式監控版面：<br>• **頂部列** — 「五限可能」隊名、地面站監控程式標題、連線狀態與封包統計<br>• **左側邊欄** — `ConnectionPanel`<br>• **中央區域** — `TelemetryGrid` + `TelemetryCharts`<br>• **右側邊欄** — 上方 `GpsMap`、下方 `AttitudeIndicator`<br>• **底部** — `StatusBar` |

#### `src/lib/` — 共用程式庫

| 檔案 | 說明 |
|------|------|
| `types.ts` | **TypeScript 型別定義**：遙測、封包統計、資料庫記錄，以及持久化的軸向與連線設定 |
| `tauri.ts` | **Tauri IPC 橋接層**：開始／停止監控、掃描序列埠、查詢歷史資料與註冊遙測事件 |
| `stores.svelte.ts` | **Svelte 5 響應式狀態管理**：遙測快照、每封包 revision、最多 200 筆圖表資料、連線狀態，以及 COM／Baud／軸向持久化設定 |
| `attitude.js` | **姿態估算器**：角速度以 `deg/s` 直接積分；總加速度接近 1g 時以互補濾波修正 Roll/Pitch，高動態時退回陀螺儀積分；Yaw 為相對航向 |
| `settings.js` | **設定驗證與保存**：使用 `rocket-ground-station.settings.v1` 保存 COM Port、Baud Rate 與三軸來源／方向 |
| `gps-map.js` | **GPS 純邏輯**：座標驗證、Haversine 距離、2 公尺軌跡門檻與 5,000 點上限 |

#### `src/components/` — UI 元件

| 檔案 | 說明 |
|------|------|
| `ConnectionPanel.svelte` | **連線與校正面板**（左側邊欄）：掃描／保存 COM Port、保存 Baud Rate、連線控制，以及可交換來源軸與反轉方向的姿態軸向設定。設定在程式重開後保留，但不會自動連線 |
| `TelemetryGrid.svelte` | **遙測數值格狀面板**（中央上方）。13 項數值依類別顯示：<br>• **IMU 感測器**（青色）：加速度 X/Y/Z (m/s²)、角速度 X/Y/Z (°/s)<br>• **GPS / 導航**（綠色）：經度、緯度、高度 (m)、地速、垂直速度<br>• **環境**（橙色）：氣壓 (hPa)、溫度 (°C)<br>• 告警機制：超過 `warnThreshold` 橙色邊框、超過 `critThreshold` 紅色脈衝動畫<br>• 交錯 slide-up 入場動畫 |
| `TelemetryCharts.svelte` | **即時遙測圖表**（中央下方）：以純 SVG 繪製相對高度與垂直速度，顯示最近 100 筆資料並自動縮放 Y 軸 |
| `GpsMap.svelte` | **GPS 即時地圖**（右上）：Leaflet/OpenStreetMap 圖磚、火箭標記、最多 5,000 點軌跡、自動跟隨、定位火箭與清除軌跡。斷網時 GPS 數值與其他遙測仍持續更新 |
| `AttitudeIndicator.svelte` | **姿態儀**（地圖下方）：火箭姿態、人工地平儀、相對航向與三軸角速度。每個新封包只更新一次；軸向變更或按下「歸零」會重設姿態 |
| `StatusBar.svelte` | **底部狀態列**。水平分隔顯示：<br>• 📡 接收狀態指示（脈衝綠點 + "接收中" / "離線"）<br>• 📦 總封包數<br>• ⚠️ CRC 錯誤率（綠/橙/紅 三級，< 5% / 5-10% / > 10%）<br>• ⏱ 接收頻率 (Hz)<br>• 🕐 連線時間 (HH:MM:SS，每秒更新) |

---

## 🖥️ UI 總覽

```
┌──────────────────────────────────────────────────────────┐
│ 五限可能 地面站監控程式                  [接收中]      │ ← 頂部列
├──────────┬──────────────────────────────┬────────────────┤
│ 串口連接  │     遙測數值面板              │  人工地平儀     │
│          │  ┌──────┬──────┬──────┐      │  (Attitude)    │
│ COM Port │  │Acc X │Acc Y │Acc Z │ ... │                │
│ Baud Rate│  └──────┴──────┴──────┘      │  航向指示器     │
│ [連接]    │                              │  (Compass)     │
│          ├──────────────────────────────┤                │
│          │  即時圖表                     │  角速度讀數     │
│          │  高度 📈  |  垂直速度 📈     │                │
├──────────┴──────────────────────────────┴────────────────┤
│ ● 接收中 | 封包: 1,234 | CRC: 0.1% | 10 Hz | 00:05:32  │ ← 狀態列
└──────────────────────────────────────────────────────────┘
```

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
- 強制釋放按鈕平時鎖定；解除安全鎖後單擊即送出並自動重新上鎖，不使用長按。
- 開始場次前輸入起始電池電壓、地點、操作者與備註。場次資料寫在應用程式資料目錄的 `flight_sessions/` 下；每筆 CSV 與事件 Log 都立即 flush，中途關閉後仍可讀取已完成寫入的資料。
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

目前 P0 雙向控制、場次紀錄與統計的交接狀態見 [`CODEX_HANDOFF.md`](CODEX_HANDOFF.md)；E22、M8N、伺服與 UI 的實機步驟見工作區 [`../P0_HARDWARE_TEST_PLAN.md`](../P0_HARDWARE_TEST_PLAN.md)。

### 1. 安裝前端相依套件

```bash
cd src-ui
pnpm install
```

### 2. 啟動開發模式

從 `ground_station/` 執行：

```powershell
& ".\src-ui\node_modules\.bin\tauri.CMD" dev
```

這會同時啟動：
- **Vite 開發伺服器** (port 8000) — 前端熱重載
- **Tauri 原生視窗** — Rust 後端

### 3. 建置正式版本

建議先在工作區根目錄執行完整驗證：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\verify_p0_workspace.ps1
```

只建置目前工作區的 Windows release 殼時，從 `ground_station/` 執行：

```powershell
& ".\src-ui\node_modules\.bin\tauri.CMD" build --no-bundle --config .\src-tauri\tauri.workspace-build.json
```

2026-07-16 已驗證的 no-bundle 產物位於 `src-tauri/target/release/app.exe`。若需要 installer／bundle，另行執行正式 bundle build 並保存該次驗證結果，不要把 no-bundle 結果當成 installer 已驗證。

### 固定發行打包流程

之後只要修改地面站程式、Rust、前端或 Tauri 設定，完成修改後必須先執行：

```powershell
& ".\package_ground_station.ps1"
```

腳本會依序執行 Rust tests／check、前端 tests／check／production build 及 Tauri release build；任一步失敗就不會產生新的發行檔。通過後會把帶版本號與時間的 `.exe`、驗證清單及 `LATEST.txt` 固定放在 `ground_station\artifacts\`。目前版本來源為 `src-tauri\tauri.conf.json` 的 `0.1.0`，不要手動複製未驗證的 `app.exe` 取代發行檔。

---

## 📜 授權條款

本專案為 2026 台灣盃火箭競賽 五限可能 內部使用。
