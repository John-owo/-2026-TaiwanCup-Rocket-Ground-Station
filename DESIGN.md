---
name: 地面站 Ground Station
description: 火箭 LoRa 遙測接收、記錄與安全上行指令的單一操作畫面，以航空圖色階繪製。
colors:
  chart-paper: "#edf0ea"
  chart-paper-shade: "#e3e8e1"
  sheet: "#f6f8f4"
  field: "#ffffff"
  navy-ink: "#0f1b2d"
  ink-secondary: "#435168"
  ink-tertiary: "#5f6c80"
  hairline: "#c6cec5"
  hairline-strong: "#9eaaa0"
  chart-blue-rail: "#1b4f8f"
  chart-blue-deep: "#143d70"
  rail-ink: "#eef3fa"
  rail-ink-secondary: "#c3d3e8"
  rail-rule: "rgba(238, 243, 250, .22)"
  aeronautical-magenta: "#a3166f"
  aeronautical-magenta-soft: "rgba(163, 22, 111, .12)"
  live-green: "#1d7444"
  live-on-rail: "#7ee0a6"
  caution-amber: "#8f5200"
  caution-amber-soft: "rgba(143, 82, 0, .1)"
  danger-red: "#b3261e"
  danger-red-soft: "rgba(179, 38, 30, .09)"
  tint-lowland: "#cfe0bb"
  tint-plain: "#e2e6a8"
  tint-upland: "#ecd391"
  tint-hill: "#e2b27c"
  tint-ridge: "#cd916c"
  tint-summit: "#b47566"
typography:
  display:
    fontFamily: "Barlow, Microsoft JhengHei UI, Microsoft JhengHei, Noto Sans TC, sans-serif"
    fontSize: "clamp(72px, 8.6vw, 132px)"
    fontWeight: 600
    lineHeight: 0.86
    letterSpacing: "-.035em"
    fontFeature: "tnum"
  headline:
    fontFamily: "Microsoft JhengHei UI, Microsoft JhengHei, Noto Sans TC, Barlow, sans-serif"
    fontSize: "21px"
    fontWeight: 700
    lineHeight: 1.28
    letterSpacing: "-.005em"
  title:
    fontFamily: "Microsoft JhengHei UI, Microsoft JhengHei, Noto Sans TC, Barlow, sans-serif"
    fontSize: "15px"
    fontWeight: 700
    lineHeight: 1.2
  reading:
    fontFamily: "Barlow, Microsoft JhengHei UI, Microsoft JhengHei, Noto Sans TC, sans-serif"
    fontSize: "clamp(20px, 1.9vw, 27px)"
    fontWeight: 600
    lineHeight: 1
    fontFeature: "tnum"
  body:
    fontFamily: "Microsoft JhengHei UI, Microsoft JhengHei, Noto Sans TC, Barlow, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.45
    fontFeature: "tnum"
  label:
    fontFamily: "Microsoft JhengHei UI, Microsoft JhengHei, Noto Sans TC, Barlow, sans-serif"
    fontSize: "12.5px"
    fontWeight: 400
    lineHeight: 1.45
  identifier:
    fontFamily: "Barlow, Microsoft JhengHei UI, Microsoft JhengHei, Noto Sans TC, sans-serif"
    fontSize: "15px"
    fontWeight: 700
    letterSpacing: ".06em"
rounded:
  sm: "4px"
  lg: "6px"
spacing:
  xs: "6px"
  sm: "8px"
  md: "12px"
  lg: "20px"
  panel-inline: "clamp(18px, 2vw, 28px)"
components:
  button-connect:
    backgroundColor: "{colors.rail-ink}"
    textColor: "{colors.chart-blue-rail}"
    typography: "{typography.title}"
    rounded: "{rounded.lg}"
    height: "46px"
  button-connect-active:
    backgroundColor: "transparent"
    textColor: "{colors.rail-ink}"
    rounded: "{rounded.lg}"
    height: "46px"
  button-dialog-primary:
    backgroundColor: "{colors.chart-blue-rail}"
    textColor: "{colors.rail-ink}"
    rounded: "{rounded.sm}"
    padding: "0 18px"
    height: "40px"
  button-dialog-primary-hover:
    backgroundColor: "{colors.chart-blue-deep}"
  button-ink:
    backgroundColor: "{colors.navy-ink}"
    textColor: "{colors.chart-paper}"
    rounded: "{rounded.sm}"
    padding: "0 12px"
    height: "38px"
  button-ink-hover:
    backgroundColor: "{colors.ink-secondary}"
  button-arm:
    backgroundColor: "transparent"
    textColor: "{colors.danger-red}"
    rounded: "{rounded.sm}"
    height: "50px"
  button-arm-unlocked:
    backgroundColor: "{colors.danger-red-soft}"
    textColor: "{colors.danger-red}"
  button-fire-ready:
    backgroundColor: "{colors.danger-red}"
    textColor: "{colors.field}"
    rounded: "{rounded.sm}"
    height: "50px"
  input-rail:
    backgroundColor: "{colors.chart-blue-deep}"
    textColor: "{colors.rail-ink}"
    rounded: "{rounded.sm}"
    padding: "0 10px"
    height: "36px"
  input-sheet:
    backgroundColor: "{colors.field}"
    textColor: "{colors.navy-ink}"
    rounded: "{rounded.sm}"
    padding: "9px 11px"
  link-block:
    backgroundColor: "{colors.chart-blue-deep}"
    textColor: "{colors.rail-ink}"
    rounded: "{rounded.lg}"
    padding: "14px 14px 12px"
  badge-safe:
    backgroundColor: "transparent"
    textColor: "{colors.live-green}"
    typography: "{typography.identifier}"
    rounded: "{rounded.sm}"
    padding: "5px 11px"
  badge-deployed:
    backgroundColor: "{colors.danger-red}"
    textColor: "{colors.field}"
  map-overlay:
    backgroundColor: "{colors.sheet}"
    textColor: "{colors.navy-ink}"
    rounded: "{rounded.sm}"
    padding: "8px 11px"
  status-bar:
    backgroundColor: "{colors.chart-paper-shade}"
    textColor: "{colors.ink-secondary}"
    typography: "{typography.label}"
    height: "34px"
---

# Design System: 地面站 Ground Station

## Overview

**Creative North Star: "航空圖色階（The Sectional Chart）"**

整個畫面像一張航空區域圖（sectional chart）那樣被閱讀：冷灰綠的圖紙底、海軍藍墨色、左緣一條實心的圖藍色側欄，資料區之間用髮絲線分隔而不是卡片。畫面上唯一響亮的墨色是火箭本身：高度曲線、地圖上的火箭標記與航跡、現在值標籤、姿態圖中的尾翼，全部是航空洋紅。高度用地形分層設色（綠、黃、黃褐、褐）表達，同一組色帶同時出現在高度階梯與高度剖面圖後方，洋紅指針隨高度移動。

密度是儀器級的：一個 1200×800 視窗內同時容納連線狀態、主高度讀數、剖面圖、姿態、13 欄遙測表、地圖與飛行控制，但只有少數數字是大的，診斷資訊安靜地退到底部狀態列。夜間版本以深海軍藍為底，角色完全相同、只換明度，兩者都要在戶外強光與暗處長時間可讀。

這個系統明確拒絕的，是類別預設的深色玻璃儀表板：統一的圓角卡片、英文 eyebrow 小標、青色光暈、等寬字型扮裝、膠囊形標籤與毛玻璃。

**Key Characteristics:**
- 圖紙底 + 海軍藍墨色 + 圖藍色左側欄，三者構成全部的面。
- 髮絲線（1px `hairline`）劃分區域；卡片不存在。
- 航空洋紅只屬於火箭。
- 六段地形分層色階是高度的唯一尺度，色階與剖面圖共用。
- 所有數字用 Barlow 等寬數字（tabular-nums），中文用微軟正黑體。
- 狀態永遠是「文字 + 形狀 + 顏色」三者並陳。

## Colors

一張冷調航空圖：低彩度的灰綠紙面與海軍藍墨，一條飽和的圖藍側欄，一種保留給火箭的洋紅，以及一組暖色地形分層色。所有顏色以 `app.css` 的 CSS 自訂屬性為唯一來源（`--paper`、`--rail`、`--rocket`、`--t0`…），日間為 `:root`，夜間為 `:root[data-theme="dark"]`；元件只引用變數，不寫死色值。

### Primary
- **圖藍側欄 Chart-Blue Rail**（`--rail`；夜間 #12335e）：左側欄整面底色，也是對話框主按鈕與日間焦點框的顏色。它擁有畫面左緣，不出現在中央資料區的面上。
- **深圖藍 Chart-Blue Deep**（`--rail-deep`；夜間 #0c264a）：側欄內的凹陷面：連線狀態區塊、側欄輸入框底色、主按鈕 hover。
- **側欄墨 Rail Ink / Rail Ink Secondary**（`--rail-ink`、`--rail-ink-2`）：側欄上的主文字與次要文字、標籤；`--rail-ink` 也是「開始監控」按鈕的實心底。
- **側欄分隔線 Rail Rule**（`--rail-rule`）：側欄內的分節線與輸入框邊框。

### Secondary
- **航空洋紅 Aeronautical Magenta**（`--rocket`；夜間 #f06cc0）：火箭高度曲線（3px）、剖面圖現在值圓點與標籤、地圖火箭標記與航跡、高度階梯指針、姿態圖尾翼、剖面圖標題中的最高值。`--rocket-soft` 是它的淡化版本。

### Tertiary
- **地形分層色 Hypsometric Tints**（`--t0` 低地綠 → `--t5` 峰頂褐，共六段）：高度階梯的六格，以及剖面圖背後的六條水平色帶（剖面圖上以 `--tint-opacity` 日間 .42、夜間 .55 疊印）。夜間色階為同色相的暗化版本（#2c4a37 → #5e2f33）。

### Neutral
- **圖紙 Chart Paper**（`--paper`；夜間 #0d1522）：應用底色、中央資料區、地圖底色、計時器凹槽底。
- **圖紙暗面 Chart Paper Shade**（`--paper-2`；夜間 #0a111c）：底部狀態列與地圖容器；FORCE RELEASE 鎖定時斜紋的一色。
- **圖頁 Sheet**（`--sheet`；夜間 #131e2e）：飛行控制區、浮在地圖上的標籤與工具、對話框。
- **欄位 Field**（`--field`；夜間 #0a111c）：圖紙區內的輸入框底色。
- **海軍藍墨 Navy Ink**（`--ink`；夜間 #dce5ee）：主文字與所有數值；也是「覆蓋倒數」實心按鈕與地圖工具啟用狀態的底色。
- **次要墨 / 三級墨**（`--ink-2`、`--ink-3`）：欄位名稱與單位、軸刻度、說明文字。
- **髮絲線 Hairline / Strong**（`--rule`、`--rule-strong`）：區域分隔、表格列線；`--rule-strong` 用於輸入框、色階外框、剖面圖零線與捲軸。

### 狀態色
- **接收綠 Live Green**（`--live`；側欄上用 `--live-on-rail`）：連線接收中、SAFE 徽章、上升趨勢、定位有效、儲存正常。
- **警示琥珀 Caution Amber**（`--warn`、`--warn-soft`）：下降趨勢、數值偏高、儲存降級、未記錄監控。
- **危險紅 Danger Red**（`--danger`、`--danger-soft`）：解除安全鎖、FORCE RELEASE 就緒、DEPLOYED、數值超限、錯誤。

### Named Rules
**The Rocket-Only Magenta Rule.** 航空洋紅只畫火箭本身與它的資料（曲線、標記、航跡、指針、現在值、最高值）。任何介面控制、狀態或裝飾都不得使用 `--rocket`。

**The Rail Owns the Left Edge Rule.** 圖藍色只以整面側欄的形式存在於左緣；中央與右欄不出現圖藍色塊，唯一例外是對話框主按鈕與焦點框。

**The Six Band Rule.** 高度尺度永遠是六段色帶，上限由 `niceAltitudeCeiling` 取能被 6 整除的整數（如 60、180、900），讓每一條色帶邊界都是整數。高度階梯與剖面圖共用同一組 `--t0`…`--t5` 與同一個上限。

## Typography

**Display Font:** Barlow（自架於 `@fontsource/barlow`，400/500/600/700，僅 latin 子集），後援為微軟正黑體
**Body Font:** Microsoft JhengHei UI / Microsoft JhengHei（後援 Noto Sans TC、Barlow）
**Label/Mono Font:** 無等寬字型；識別碼與協定字串同樣用 Barlow

**Character:** Barlow 是 DIN 系的道路與圖面字體，給數字一種量測儀器的硬朗與清晰；微軟正黑體承擔所有中文，平實不搶戲。兩者以角色切分：`--font-num` 給數字，`--font-zh` 給文字，全域開啟 `tabular-nums` 讓跳動的數值不抖動。

### Hierarchy
- **Display**（600，`clamp(72px, 8.6vw, 132px)`，行高 .86，字距 -.035em）：只用於主高度讀數這一個數字。視窗高度 ≤820px 時縮為 `clamp(64px, 7.4vw, 104px)`。單位「m」以 600、`clamp(20px, 1.8vw, 28px)` 跟在旁邊。
- **Headline**（700，21px，行高 1.28）：側欄中的目前場次名稱，可換行、`text-wrap: balance`。對話框標題同層級（700，22px）。
- **Title**（700，15px）：每個面板的中文標題（高度剖面、估算姿態、全部遙測、飛行控制、GPS 位置）。主高度區的「相對高度」用 500 字重、`--ink-2`，讓數字當主角。
- **Reading**（600，`clamp(20px, 1.9vw, 27px)`，行高 1）：右側四個次要讀數；趨勢列數值（20px）、姿態角度（20px）、倒數秒數（34px）屬同一家族。
- **Body**（400，14px，行高 1.45）：全域預設文字。遙測表數值 14.5px/500。
- **Label**（400，12–13.5px）：欄位名稱、單位、軸刻度（11–12px）、說明與狀態列（12.5px）。單位以 `<small>` 用 `--ink-3` 降一級。
- **Identifier**（Barlow 700，15px，字距 .06em）：SAFE / DEPLOYED 徽章與 FORCE RELEASE，這些是協定與安全識別字，保留英文大寫。

### Named Rules
**The Barlow Numerals Rule.** 每一個數字（讀數、刻度、封包數、COM 埠、場次 ID）都套 `.num` 或 `--font-num`；中文標籤一律用 `--font-zh`。不使用任何等寬字型。

**The Chinese Label Rule.** 面板標題與欄位名稱是中文、句中大小寫；英文只出現在協定識別字（COM、CRC、SET_TIMER、FORCE RELEASE、RUN ID）。

## Layout

三欄固定殼層，填滿 `100dvh`、本身不捲動：左側欄 `clamp(216px, 17vw, 264px)`（跨兩列直到底），中央 `minmax(0, 1fr)`，右欄 `clamp(320px, 26vw, 400px)`；底部狀態列從第二欄跨到最後。

- **中央**：上為主讀數列（高度階梯 40px + 刻度 + 大數字，右側 `clamp(200px, 22vw, 300px)` 的四格髮絲線分隔欄），中為高度剖面圖（至少 230px，吃掉剩餘高度），下為姿態（`minmax(270px, .42fr)`）與 13 欄遙測表並排。
- **右欄**：地圖（至少 200px，吃掉剩餘高度），下接飛行控制區。
- **內距**：面板左右內距用 `clamp(18px, 2vw, 28px)` 或 `clamp(16px, 1.6vw, 22px)`，上方約 14px；側欄 `22px 20px 18px`、分節間距 20px。元素間距集中在 6 / 8 / 12 / 20px。
- **斷點**：≤1360px 遙測表改兩欄、姿態欄收窄；≤1180px 主讀數與次要讀數上下堆疊（次要讀數改橫排四格），姿態與表格堆疊；≤900px 整體改單欄、頁面可捲動，地圖固定 340px；≤560px 次要讀數兩欄、表格單欄。視窗高度 ≤820px 時縮小大數字、階梯與側欄間距，確保 1200×800 不捲動。

### Named Rules
**The Hairline-Not-Card Rule.** 區域之間用 1px `--rule` 線分隔，面板沒有自己的底色框、圓角或陰影。只有語意上「凹陷」的元件（計時器槽、側欄連線區塊）才有面。

## Elevation & Depth

系統以平面與色調分層為主：圖紙、圖頁、側欄三個面直接相鄰，靠髮絲線與明度差區分層次。陰影只用在「浮在地圖之上」的東西與模態對話框，它們真的蓋在另一層內容上面。

### Shadow Vocabulary
- **浮貼 Pop**（`--shadow-pop`：日間 `0 6px 18px rgba(15, 27, 45, .14)`，夜間 `0 8px 22px rgba(0, 0, 0, .45)`）：地圖上的 GPS 標籤、地圖工具按鈕、發射點距離/方位列、縮放控制、錯誤提示。
- **對話框 Modal**（`box-shadow: 0 24px 60px rgba(15, 27, 45, .28)`，背幕 `rgba(15, 27, 45, .55)`）：測試場次對話框。
- **標記 Marker**（`box-shadow: 0 2px 8px rgba(15, 27, 45, .35)`）：地圖上的火箭圓點標記。

### Named Rules
**The Float-Over-Map Rule.** 只有疊在地圖或整個畫面之上的元素可以有陰影；固定在殼層格線中的面板永遠是平的。

## Shapes

小而克制的圓角：一般元件 4px（`--radius`），較大的實心塊（側欄連線區塊、開始監控按鈕、對話框）6px（`--radius-lg`）。高度階梯是 3px 外框的直條，內部六格只在頭尾圓 2px。圓形只用於具有圖面意義的符號：連線狀態點、火箭與發射點標記、姿態圓盤。

鎖定中的 FORCE RELEASE 以 -45° 斜紋（`--paper-2` / `--rule`，8px 一格）表示「此區封閉」，這是圖面上禁航區的語彙。展開箭頭是 CSS 畫的 1.5px 折角，圖示全部是內嵌 1.6–1.8px 線條 SVG。

## Components

### Buttons
儀器面板上的開關：扁平、實心或描邊，沒有漸層與光暈。
- **Shape:** 4px 圓角；側欄主按鈕 6px。
- **開始監控（側欄主按鈕）:** `--rail-ink` 實心底、`--rail` 文字、15px/700、高 46px；hover 轉為純白。監控中改為 1.5px `--rail-ink` 描邊的透明按鈕「停止監控」。
- **墨色按鈕（覆蓋倒數、地圖工具啟用）:** `--ink` 底、`--paper` 文字，hover `--ink-2`。
- **對話框主按鈕:** `--rail` 底、`--rail-ink` 文字、高 40px、左右 18px，hover `--rail-deep`；取消為 `--rule-strong` 描邊透明。
- **文字按鈕（重新掃描、姿態歸零、恢復設定）:** 底線樣式，底線色為分隔線色，hover 時底線變為文字色。需二次確認的動作（恢復所有設定）第一次點擊後轉為警示色並改字「再次點擊確認」，3 秒後還原。
- **Hover / Focus:** 顏色過渡 160ms `--ease-out`；焦點框 2px `--rail` 外框、offset 2px（夜間改 `--rail-ink-2`）。停用為 .45–.6 不透明度。

### 兩段式釋放（Signature）
- **解除安全鎖:** 1.5px `--danger` 描邊、紅字、鎖頭線條圖示，高 50px；解鎖後底色 `--danger-soft`、圖示改為開鎖、字改「重新上鎖」。無即時遙測時描邊與文字退為 `--rule-strong` / `--ink-3`。
- **FORCE RELEASE:** 鎖定時為斜紋封閉區、`--ink-3` 文字；只有解鎖且即時遙測正常時才轉為 `--danger` 實心白字。兩鈕以 1 : 1.25 並排，下方一行說明目前的閘門條件。

### 狀態徽章
- **SAFE:** 1.5px `--live` 描邊、綠字、盾牌線條圖示、Barlow 700 15px 字距 .06em、4px 圓角。
- **DEPLOYED:** 同形，`--danger` 實心白字、電源線條圖示。

### Inputs / Fields
- **側欄:** 高 36px、`--rail-deep` 底、`--rail-rule` 邊框、4px 圓角、Barlow 500 14px；focus 時邊框轉 `--rail-ink-2`。
- **圖紙區 / 對話框:** `--field` 底、`--rule-strong` 邊框、4px 圓角；對話框 focus 為 `--rail` 邊框加 3px `color-mix(in srgb, var(--rail) 18%, transparent)` 光環；倒數輸入 focus 為 `--ink` 邊框。
- **Error / Disabled:** 錯誤以 `--danger` 文字與 `--danger-soft` 底的細框訊息呈現；警告同構用 `--warn`。停用 .45–.6 不透明度。

### 連線狀態（側欄）
`--rail-deep` 凹陷區塊、6px 圓角，大字 20px/700 狀態名稱前有 12px 圓點：接收中為實心綠點、等待中為虛線空心圈、待命為空心圈、失聯時整塊轉為深紅底。下方 `dt/dd` 兩欄列出最後封包、接收頻率、遺失/CRC、記錄與儲存。底部狀態列以 9px 圓點重複同一套形狀語彙。

### 高度色階（Signature）
40px 寬的直立六格色階，`--rule-strong` 外框；左側刻度 Barlow 500 11px。洋紅指針是 3px 橫線加右側三角箭頭，以 500ms `--ease-out` 隨高度移動。旁邊是 Display 級高度數字、單位，以及帶上升/下降箭頭的垂直速度（綠/琥珀）、本場最高、空中端時間三項趨勢讀數。

### 高度剖面圖
同一組六條色帶以 `--tint-opacity` 鋪在圖背後，`--rule-strong` 零線，3px 洋紅曲線（`vector-effect: non-scaling-stroke`），現在值為 11px 洋紅圓點（2px 圖紙色描邊）與右側洋紅標籤。橫軸是空中端時間（相對秒數至「現在」），縱軸刻度與色帶邊界對齊。

### 地圖
OSM 圖磚經 `--map-filter` 去色、偏綠並以 `mix-blend-mode: multiply`（夜間 `screen`）融入圖紙，讓底圖退成灰階地形，只留洋紅航跡與火箭標記。所有疊加物是 `--sheet` 底、`--rule` 邊框、4px 圓角、`--shadow-pop`。發射點標記為墨色雙圈。

### 遙測表
三欄（窄時兩欄、一欄）的 `dt/dd` 列，每列底部 1px `--rule`；欄名 `--ink-2` 13px，數值 14.5px/500，單位 `--ink-3` 11.5px 固定寬度。偏高與超限時數值轉色加粗，並在欄名後附加「偏高」「超限」文字。

### Navigation
本產品是單一畫面，沒有導覽列；左側欄承擔產品標記、場次身份、連線與設定，日/夜配色切換固定在側欄底部。

## Do's and Don'ts

### Do:
- **Do** 所有顏色都透過 `app.css` 的角色變數取用，並同時檢查日間與 `data-theme="dark"` 兩種值。
- **Do** 用 1px `--rule` 髮絲線劃分新區域，讓區域直接坐在圖紙上。
- **Do** 每個數字套 `.num`（Barlow、tabular-nums），單位用 `--ink-3` 小一級跟在後面。
- **Do** 狀態同時用文字、形狀（實心/空心/虛線圈、箭頭、斜紋）與顏色表達。
- **Do** 任何新的高度視覺都沿用六段 `--t0`…`--t5` 色帶與能被 6 整除的上限。
- **Do** 危險動作維持兩段式：先解鎖（描邊），再發射（實心紅）。
- **Do** 圖示用內嵌線條 SVG（1.6–1.8px，`currentColor`）。

### Don't:
- **Don't** 把 `--rocket` 洋紅用在任何不是火箭或火箭資料的東西上。
- **Don't** 做統一圓角卡片、毛玻璃、青色光暈或深色玻璃儀表板。
- **Don't** 使用膠囊形（全圓角）標籤或按鈕；圓角上限是 6px。
- **Don't** 加英文 eyebrow 小標或面板上方的大寫 kicker；面板標題就是中文標題。
- **Don't** 用等寬字型扮演「科技感」。
- **Don't** 給殼層格線內的面板加陰影；陰影只屬於浮在地圖或畫面之上的元素。
- **Don't** 在中央或右欄放大面積的圖藍色塊。
