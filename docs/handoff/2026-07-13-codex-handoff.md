# Ground Station Codex Handoff - 2026-07-13

## Repository state

- Remote: `https://github.com/John-owo/-2026-TaiwanCup-Rocket-Ground-Station.git`
- Branch: `feature/ground-station-upgrade`
- Platform used for verification: Windows 11, PowerShell, Tauri 2, Svelte 5
- This handoff intentionally contains the current monitoring/settings/attitude work plus a portable Windows executable.
- No real-hardware result is recorded as passing unless explicitly listed below.

## Laptop continuation

```powershell
git clone --branch feature/ground-station-upgrade --single-branch `
  https://github.com/John-owo/-2026-TaiwanCup-Rocket-Ground-Station.git `
  "TASA-RTC-2026-ground-station"
cd "TASA-RTC-2026-ground-station\src-ui"
pnpm install
pnpm test
pnpm check
pnpm build
cd ..
```

Start the full Tauri application from the repository root, not from `src-ui`:

```powershell
& ".\src-ui\node_modules\.bin\tauri.CMD" dev
```

The portable test build is available without compiling:

```text
artifacts/GroundStation_0.1.0_Portable_2026-07-13.exe
```

SHA-256:

```text
CA32C200F1E7790C5E5F04BA9FF2104A9EE821A2E7217795FA959E195D32E7BD
```

The executable is unsigned, so Windows SmartScreen may require **More info -> Run anyway**.

## Work included in this handoff

### 1. Persistent ground-station UI settings

- COM port, baud rate, and attitude axis mapping are restored from validated local storage.
- Invalid/non-serializable axis mapping data falls back safely.
- Visible operational UI remains Traditional Chinese.
- The left connection panel has the intended stacking order above central content.

Relevant files:

- `src-ui/src/App.svelte`
- `src-ui/src/lib/settings.js`
- `src-ui/src/lib/settings.test.mjs`

### 2. Start monitoring repair

Confirmed failures in the previous flow:

- The monitoring button had been changed from a direct click handler to a window-level pointer-coordinate workaround.
- Rust returned success before the COM port was actually opened.
- Background serial errors entered the store but were not displayed by the connection panel.
- Monitoring token reservation and cleanup had concurrency gaps.

Implemented behavior:

- The button directly uses `onclick={handleConnect}`.
- `start_monitoring` atomically reserves one monitoring task and awaits the real serial-port open.
- Open failure rejects the invoke and releases state so the user can retry.
- Receive-loop cleanup does not delete a newer monitoring reservation.
- The connection panel displays the latest background serial error and clears stale errors on retry.

Relevant files:

- `src-tauri/src/commands/serial.rs`
- `src-ui/src/components/ConnectionPanel.svelte`
- `src-ui/src/lib/settings.test.mjs`

Automated Rust state tests cover atomic reservation, current-task cleanup, and old-task cleanup preserving a new reservation.

### 3. MPU6050 attitude display repair

Observed real telemetry cadence from the live `telemetry.db`:

```text
20 measured packet gaps
minimum: 488.0 ms
average: 524.8 ms
maximum: 990.7 ms
over the old 250 ms limit: 20/20
```

Actual IMU inputs were finite and changing. Replaying 21 real rows at the measured cadence through the old estimator produced zero non-zero attitude outputs. The estimator updated `lastTimestampMs` and rejected every packet because each normal gap exceeded 250 ms.

Fix:

- Increase `MAX_PACKET_GAP_MS` from 250 ms to 1500 ms.
- This accepts the observed 500-1000 ms cadence with margin.
- Multi-second interruptions still rebuild the time baseline and do not integrate downtime.
- Gyroscope units remain `deg/s`; parser, CRC, and firmware contracts were not changed.

Replaying the same 21 real IMU rows and measured timestamps after the fix produced 20 non-zero outputs. The first row remains zero by design because it establishes the time baseline.

Relevant files:

- `src-ui/src/lib/attitude.js`
- `src-ui/src/lib/attitude.test.mjs`

Regression coverage includes 100 ms, 500 ms, and 1000 ms packet intervals, multi-second interruptions, reset behavior, finite-value handling, and axis/sign mapping.

## Verification evidence

Latest frontend verification:

```text
pnpm test  -> PASS, 29 tests, 0 failures
pnpm check -> PASS, 0 errors, 0 warnings
pnpm build -> PASS, 132 modules transformed
```

Rust verification performed for the monitoring repair:

```text
cargo test  -> PASS, 3 tests, 0 failures
cargo check -> PASS, existing dead-code warnings only
```

`cargo fmt --check` is not green in the baseline repository. It reports pre-existing formatting differences in several untouched Rust files. Those unrelated files were deliberately not reformatted.

The release build completed with:

```text
Tauri build --no-bundle -> PASS
Executable size          -> 11.05 MiB
PE header                -> MZ
File version             -> 0.1.0
Authenticode             -> NotSigned
```

## Manual tests still required

The following remain **NOT RUN** against the final source/build:

- Blank COM input in the actual Tauri window.
- Invalid `COM999` open failure and immediate retry.
- Real unoccupied COM port connection.
- Occupied COM port failure.
- Disconnect and reconnect.
- Device removal/read-error recovery.
- Rotate the MPU6050 and confirm roll/pitch/yaw, rocket model, artificial horizon, and heading all update.
- Zero the attitude and confirm later packets move it again.
- Pause/reconnect telemetry and confirm no large attitude jump.
- Observe stationary drift with the real sensor.

## Build caveats

- The old `src-tauri/target` cache in this checkout previously had an incomplete third-party build-script directory. Use an independent target directory if it recurs:

```powershell
$env:CARGO_TARGET_DIR = Join-Path $env:TEMP "tasa-ground-station-target"
& ".\src-ui\node_modules\.bin\tauri.CMD" build --no-bundle
```

- Invoke the local Tauri CLI from the repository root. Running it from `src-ui` cannot discover the sibling `src-tauri/tauri.conf.json`.
- The application uses relative yaw because there is no magnetometer.
- Do not change gyro units from `deg/s` unless the firmware/telemetry contract is deliberately changed and verified.

## Recommended next Codex task

1. Run the portable executable or Tauri dev build with the real ground-station hardware.
2. Complete the manual matrices above without treating unrun cases as passing.
3. If attitude still does not move, capture `telemetryRevision`, estimator input/output, and `performance.now()` gaps in the running WebView before changing more code.
4. If monitoring opens but packet count stays zero, treat that as a separate LoRa/baud/packet-format issue; do not rewrite the attitude estimator or parser without evidence.
