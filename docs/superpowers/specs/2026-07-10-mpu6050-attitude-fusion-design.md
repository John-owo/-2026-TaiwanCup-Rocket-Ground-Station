# MPU6050 姿態解算修正設計

## 目標

修正目前姿態顯示的三個根因：角速度單位契約不一致、同一筆遙測可能被重複積分，以及純陀螺儀積分造成的 Roll/Pitch 漂移。修正後，地面站應能只靠現有的 MPU6050 提供穩定的相對姿態；BMP280 與 NEO-6M 維持各自的高度與定位用途，不錯誤混入姿態融合。使用者可以直接在 UI 調整感測器軸向，並永久保存軸向、COM Port 與 Baud Rate。監控介面的遙測與操作名稱統一改為中文。NEO-6M 的有效經緯度透過 Leaflet 與 OpenStreetMap 顯示即時位置和飛行軌跡。

## 已確認的限制

- 遙測封包只包含三軸加速度與三軸角速度，沒有磁力計資料。
- MPU6050 的實際安裝方向目前未知。
- 封包文件、Rust 型別註解與遙測畫面均把角速度定義為 `deg/s`。
- 沒有磁力計時，Yaw 只能是可歸零的相對角度，無法成為絕對航向。
- GPS 的移動方向是地面航跡，不等於火箭本體朝向，因此不拿來校正 Yaw。
- BMP280 的氣壓高度不提供姿態資訊。

## 根因

1. `attitude.js` 把封包角速度視為 `rad/s`，再次乘以 `180 / pi`；若封包遵守既有的 `deg/s` 契約，姿態變化會被放大約 57.3 倍。
2. `AttitudeIndicator.svelte` 的 reactive effect 同時讀取並寫入 Roll、Pitch、Yaw，更新時機沒有明確綁定到新的遙測封包，因此不能保證每筆資料只積分一次。
3. 目前 Roll、Pitch、Yaw 都只做角速度乘時間的 Euler 積分，沒有用 MPU6050 加速度計提供的重力方向抑制 Roll/Pitch 漂移。

## 設計決策

### 1. 明確的輸入單位

地面站以現有封包契約為準，三軸角速度一律解讀為 `deg/s`。姿態函式不再隱含執行弧度轉角度。函式名稱、參數名稱、註解、測試與畫面單位都使用 `degreesPerSecond` 或 `deg/s`，避免未來再次混淆。

不使用根據數值大小自動猜測單位的做法，因為慢速的 `deg/s` 與快速的 `rad/s` 數值範圍可能重疊，飛行中會產生不可預測的誤判。

### 2. 可由 UI 調整的感測器軸映射

姿態函式接收一份集中式軸向設定，將感測器的 X/Y/Z 向量映射成火箭本體的 X/Y/Z 向量。每個本體軸可指定來源軸與正負號，例如：

```js
const SENSOR_TO_BODY_AXIS = {
  x: { source: 'x', sign: 1 },
  y: { source: 'y', sign: 1 },
  z: { source: 'z', sign: 1 },
};
```

預設維持目前程式的 X=Roll、Y=Pitch、Z=Yaw。`ConnectionPanel` 增加可收合的「姿態軸向設定」區塊，依序顯示火箭 X、Y、Z 三列；每列包含來源軸選擇器與方向切換。設定變更後立即套用，不需要修改程式或重新啟動。

三個火箭本體軸必須分別使用不同的感測器來源軸，形成 X/Y/Z 的有效排列。UI 不允許儲存重複來源軸；若使用者把某列改成已被占用的來源軸，介面會交換兩列的來源軸，使映射始終有效。方向值只能是 `+1` 或 `-1`。

同一份映射必須同時套用到加速度與角速度，避免兩種感測器資料使用不同座標系。

軸向設定變更時，姿態估算器立即歸零並重設時間基準，避免新舊座標系的角度混合。UI 提供「恢復預設軸向」操作，將映射回復為 X→X、Y→Y、Z→Z 且方向均為正。

### 3. 每個封包只更新一次

遙測 store 新增單調遞增的 `telemetryRevision`。每次 `updateTelemetry` 收到新封包便增加一次，數值相同的連續封包也會被視為兩筆獨立樣本。

`AttitudeIndicator` 只以 `telemetryRevision` 和最新 telemetry 觸發更新。姿態估算器在普通 JavaScript closure 中保存上一個姿態與時間，不把顯示用的 reactive Roll/Pitch/Yaw 當成 effect 的輸入，因此輸出狀態變更不會再次觸發積分。

時間差使用封包抵達時的 monotonic clock。第一筆封包只建立時間基準；時間差小於或等於零時不積分。若封包間隔超過 250 ms，視為中斷或重新連線，只更新時間基準，不用舊角速度跨越整段空窗積分。

### 4. MPU6050 六軸互補融合

每筆有效封包先用角速度預測姿態：

```text
rollGyro  = roll  + gyroX * dt
pitchGyro = pitch + gyroY * dt
yaw       = yaw   + gyroZ * dt
```

加速度計只在總加速度接近 1g 時提供重力方向校正。預設有效範圍為 `0.85g` 至 `1.15g`；火箭推進、高動態、失重或明顯振動造成總加速度超出範圍時，該筆資料退回純陀螺儀積分。

有效時，以標準重力向量公式得到 Roll/Pitch 觀測值：

```text
accelRoll  = atan2(accelY, accelZ)
accelPitch = atan2(-accelX, sqrt(accelY^2 + accelZ^2))
```

Roll/Pitch 使用互補濾波融合陀螺儀預測與加速度觀測。濾波時間常數固定為 0.5 秒，每筆權重使用 `alpha = tau / (tau + dt)` 計算，保持不同封包頻率下大致相同的反應。角度融合使用最短角度差，避免在 `-180/180` 邊界跳動。

Yaw 只積分 Z 軸角速度並正規化到 `0..360`。畫面名稱改為 `RELATIVE HEADING`，避免把它誤解為真北航向。

### 5. 歸零與失效處理

姿態卡提供 `ZERO` 操作，將目前 Roll、Pitch、Yaw 歸零並重設估算器時間基準。這供安裝方向測試、發射前校正與重新連線後使用。

若任何必要輸入不是有限數值，該封包不更新姿態。若加速度無效但角速度有效，仍可進行純陀螺儀積分；若角速度無效，整筆姿態更新跳過。UI 不顯示 `NaN` 或 `Infinity`。

### 6. 設定持久化

前端新增集中式設定模組，以 versioned local storage key `rocket-ground-station.settings.v1` 保存：

- COM Port 字串。
- Baud Rate 數值。
- 三軸來源與各軸正負號。

程式啟動時先載入並驗證設定，再提供給連線面板與姿態估算器。設定在 UI 變更後立即保存。程式重新開啟時恢復上次選擇，但不自動建立序列埠連線，避免在硬體環境改變時產生意外連線。

COM Port 可保留目前不在掃描清單中的值；介面必須顯示該值不可用並要求重新選擇，不能靜默改連其他序列埠。Baud Rate 僅接受連線面板支援的選項；無效值回復為 `115200`。

設定載入時逐欄驗證，不信任 local storage 內容。JSON 損壞、版本不支援、軸向重複、來源軸不明或 sign 不是 `+1/-1` 時，對應欄位回復安全預設值。提供「恢復所有設定」操作，回復預設軸向、預設 Baud Rate 與空白 COM Port，但不自動連線。

### 7. 中文介面

所有一般使用者可見的監控欄位、狀態與操作名稱改為繁體中文，包括：

- 加速度、角速度、高度、地面速度、垂直速度、氣壓、溫度、經度與緯度。
- 姿態、相對航向、角速度、歸零、恢復預設軸向。
- 連線、斷線、連線狀態、封包統計與錯誤狀態。

標準工程縮寫與單位維持原樣，例如 COM、Baud、GPS、IMU、CRC、`m/s`、`m/s²`、`deg/s`、`hPa`。資料結構欄位與程式內部識別字不為了顯示文字而改名；中文化集中在 UI label，避免破壞 Tauri 事件與序列化契約。

### 8. Leaflet 與 OpenStreetMap GPS 即時地圖

桌面版在右側欄最上方新增「GPS 即時位置」卡片，使用 Leaflet 顯示 OpenStreetMap 標準圖磚；現有火箭姿態、人工地平儀、相對航向與角速度卡片依序排列在地圖下方。地圖卡預設寬度跟隨右側欄，高度 280 px，確保啟動後不需捲動即可看到即時位置。地圖不需要 API Key 或計費帳戶。圖磚 URL 固定使用官方要求的 `https://tile.openstreetmap.org/{z}/{x}/{y}.png`，畫面右下角保留清楚可見的 `© OpenStreetMap contributors` attribution。

窄螢幕版不強制維持三欄：GPS 地圖移到遙測數據與圖表下方、姿態卡片上方，寬度改為可用內容寬度，高度仍至少 280 px。版面尺寸變更後呼叫 Leaflet `invalidateSize()`，避免地圖出現灰區或圖磚錯位。

程式只請求使用者目前可見視窗所需的圖磚，遵守 HTTP cache header，不實作背景預抓、整區下載或離線地圖。圖磚供應者被封裝在獨立設定中，不把 URL 散落在元件內，未來可切換到其他 OSM 圖磚服務而不必重寫 GPS 邏輯。

地圖初始中心設為台灣中心附近 `(23.7, 121.0)`、縮放層級 7。第一筆有效 GPS 座標到達後建立火箭位置標記、移至該位置並切換到縮放層級 16。後續 GPS 更新只移動既有標記與延伸軌跡，不重新建立 Leaflet map 或 tile layer。

有效座標必須是有限數值，緯度位於 `-90..90`、經度位於 `-180..180`。由於目前封包沒有 GPS fix、衛星數或 HDOP 欄位，`(0, 0)` 視為 NEO-6M 尚未定位，不建立標記或軌跡。無效座標不清除上一個有效位置，但畫面標示「等待有效定位」。

為降低 GPS 靜止飄移與記憶體使用，只有與上一個軌跡點相距至少 2 公尺的有效座標才加入 polyline；火箭標記仍會使用每一筆有效座標更新。軌跡最多保留 5,000 點，超過時移除最舊點。距離使用可單元測試的 Haversine 函式計算。

地圖提供：

- 「自動跟隨」切換，預設開啟；開啟時每筆有效位置更新都將地圖中心移到火箭。
- 「定位火箭」操作，立即移到最後一筆有效位置並重新開啟自動跟隨。
- 「清除軌跡」操作，只清除 polyline，不清除目前位置標記或遙測資料。
- 經度、緯度、GPS 地面速度、最後有效定位時間與目前定位狀態。

使用者手動拖曳地圖時，自動跟隨關閉，避免畫面立刻跳回火箭；縮放不自動關閉跟隨。自動跟隨與目前地圖視窗只屬於本次執行狀態，不寫入永久設定。

地圖載入失敗、斷網或圖磚請求失敗時，卡片顯示中文提示「地圖載入失敗，GPS 數值仍持續更新」。經緯度、地速與其他遙測不得依賴地圖成功載入。網路恢復後 Leaflet 可依正常圖磚請求重新顯示，不重新建立序列埠連線。

Tauri CSP 只開放 OpenStreetMap 圖磚所需的 HTTPS 圖片與連線來源，不加入 Google Maps、Places、Street View 或其他不使用的網域。Leaflet CSS 與 marker 圖示打包進應用程式，不依賴 CDN。

## 元件與檔案責任

- `src-ui/src/lib/attitude.js`
  - 接收並套用 UI 提供的軸向映射。
  - `deg/s` 角速度積分。
  - 加速度有效性判斷。
  - Roll/Pitch 互補濾波。
  - Yaw 正規化。
  - 可重設、以封包為單位更新的姿態估算器。
- `src-ui/src/lib/attitude.test.mjs`
  - 純函式與估算器回歸測試。
- `src-ui/src/lib/settings.js`
  - 定義預設設定、版本化 local storage key、欄位驗證、載入、保存與重設。
- `src-ui/src/lib/settings.test.mjs`
  - 以記憶體 storage 物件測試設定驗證與持久化，不依賴瀏覽器或 mock 內部實作。
- `src-ui/src/lib/gps-map.js`
  - GPS 座標驗證、Haversine 距離、2 公尺軌跡門檻與 5,000 點上限。
- `src-ui/src/lib/gps-map.test.mjs`
  - 測試有效座標、`(0, 0)` 未定位、距離門檻、軌跡上限與無效數值處理。
- `src-ui/src/lib/stores.svelte.ts`
  - 維護 `telemetryRevision`，保證相同數值的連續封包仍有獨立事件身份。
  - 保存已驗證的應用程式設定，提供 UI 與姿態元件共享。
- `src-ui/src/components/AttitudeIndicator.svelte`
  - 每個 revision 呼叫估算器一次。
  - 顯示融合後姿態與原始 `deg/s` 角速度。
  - 顯示相對航向並提供 `ZERO` 操作。
- `src-ui/src/components/ConnectionPanel.svelte`
  - 載入並保存 COM Port 與 Baud Rate。
  - 提供可收合的姿態軸向設定、方向切換與恢復預設操作。
- `src-ui/src/components/TelemetryGrid.svelte`、`TelemetryCharts.svelte`、`StatusBar.svelte` 與 `App.svelte`
  - 將一般使用者可見的監控欄位、狀態與操作名稱改為繁體中文。
- `src-ui/src/components/GpsMap.svelte`
  - Leaflet map、OpenStreetMap tile layer、火箭標記、軌跡與跟隨控制的生命週期。
  - 接收最新 telemetry 與 revision，不把地圖物件放進 reactive store。
- `src-ui/src/App.svelte` 與 `app.css`
  - 桌面版將 GPS 地圖放在右側欄最上方，姿態卡排列在下方。
  - 窄螢幕版將地圖移到遙測區下方並在尺寸變更後通知 Leaflet 重算。
- `src-ui/package.json`
  - 加入 `leaflet` 與 TypeScript 型別依賴。
- `src-tauri/tauri.conf.json`
  - 僅加入 OpenStreetMap 圖磚所需的 CSP 網域。
- `README.md`
  - 將姿態說明更新為 MPU6050 六軸融合、`deg/s` 契約與相對 Yaw 限制。
  - 記錄 Leaflet、OpenStreetMap attribution、聯網需求與禁止離線預抓圖磚。

## 測試策略

### 單位與積分

- `90 deg/s` 持續 1 秒得到 90 度，不再額外乘以 57.3。
- Roll 正確跨越 `-180/180`，Yaw 正確跨越 `0/360`。
- 零或負時間差不改變姿態。
- 大於 250 ms 的封包空窗不使用舊角速度補積分。

### 封包更新

- 相同內容的兩個封包會產生兩個 revision。
- 一個 revision 只讓估算器更新一次；顯示狀態改變不會自行產生新積分。

### 加速度融合

- 靜止且總加速度約 1g 時，Roll/Pitch 會朝重力觀測角收斂。
- 總加速度低於 `0.85g` 或高於 `1.15g` 時，只使用陀螺儀預測。
- 加速度包含非有限值時，不污染姿態輸出。

### 軸向設定

- 軸交換與正負號會同時套用到加速度與角速度。
- 預設映射維持現有 X=Roll、Y=Pitch、Z=Yaw 行為。
- UI 軸向設定不允許形成重複來源軸。
- 軸向變更會重設姿態與時間基準。

### 設定持久化

- 有效的 COM Port、Baud Rate 與軸向設定可保存並重新載入。
- 重新載入設定不會自動連線。
- 損壞的 JSON、未知版本、無效 Baud Rate 與無效軸向會安全回復預設值。
- 已保存但目前不存在的 COM Port 會保留並標示不可用，不會改連其他序列埠。
- 「恢復所有設定」會清除 COM Port、使用 `115200` 並恢復預設軸向。

### 中文介面

- 遙測表格與圖表不再顯示 `Altitude`、`Ground Speed`、`V. Velocity`、`Air Pressure`、`Temperature` 等英文欄位名稱。
- 姿態與連線操作使用繁體中文；工程縮寫和標準單位保持不變。

### GPS 地圖

- 桌面版地圖位於右上角且高度為 280 px，姿態卡位於其下方。
- 窄螢幕版地圖位於遙測區下方、姿態卡上方，調整尺寸後不出現灰區。
- 合法的台灣經緯度能建立並移動同一個火箭標記，不重建 Leaflet map。
- `NaN`、`Infinity`、超出範圍與 `(0, 0)` 不建立位置或軌跡。
- 少於 2 公尺的 GPS 飄移不加入軌跡，但仍更新目前位置標記。
- 軌跡超過 5,000 點時只保留最新 5,000 點。
- 手動拖曳會關閉自動跟隨；「定位火箭」會重新開啟跟隨並定位最後有效座標。
- 清除軌跡不會清除目前位置標記。
- 圖磚或網路失敗不會停止 GPS 數值、序列埠或其他遙測更新。
- 元件卸載時移除 Leaflet event listener 並銷毀 map instance，重新掛載不產生重複地圖。

### 完整驗證

- 執行前端單元測試。
- 執行 Svelte/TypeScript 靜態檢查。
- 執行前端正式建置。
- 在可聯網環境手動確認 OpenStreetMap 圖磚、attribution、火箭標記、軌跡與自動跟隨。
- 若既有依賴狀態阻擋靜態檢查或建置，必須明確回報實際錯誤，不得以單元測試通過代替完整成功聲明。

## 現場校正程序

1. 感測器靜置，確認角速度接近零，Roll/Pitch 不持續快速漂移。
2. 按 `ZERO`，只繞預期 Roll 軸轉動，確認只有 Roll 主要改變且方向正確。
3. 重複 Pitch 與 Yaw 測試。
4. 若軸錯誤或方向相反，直接在 UI 的「姿態軸向設定」調整來源軸或方向。
5. 將裝置靜置於不同傾角，確認 Roll/Pitch 能靠重力方向穩定，而相對 Yaw 不宣稱絕對北向。

## 不在本次範圍

- 從不可靠的數值範圍自動判斷 `rad/s` 或 `deg/s`。
- 使用 NEO-6M 地面航跡假裝成火箭本體 Yaw。
- 使用 BMP280 高度推導姿態。
- 在沒有磁力計的情況下提供絕對航向。
- 修改火箭端韌體或封包格式。
- 自動連線到上次使用的 COM Port。
- 翻譯 COM、Baud、GPS、IMU、CRC 或標準工程單位。
- Google Maps、Google Places、Street View、路線規劃或地址搜尋。
- OpenStreetMap 圖磚的背景預抓、批次下載或離線地圖。
- 在目前沒有 GPS fix、衛星數或 HDOP 欄位的封包中推測定位品質。
