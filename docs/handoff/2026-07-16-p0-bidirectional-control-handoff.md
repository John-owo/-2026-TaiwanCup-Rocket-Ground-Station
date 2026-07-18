# Ground Station P0 Handoff — 2026-07-16

## Current position

- Repository branch: `feature/ground-station-upgrade`
- Original workspace baseline: commit `67b813803fb8c2248142b790f4a07742bf48a9da`
- The working tree contains pre-existing and current uncommitted changes. Do not reset, discard, stage or commit them without first reviewing ownership and scope.
- Protocol source of truth: workspace `../protocol/PROTOCOL_V1.md`
- Shared vectors: workspace `../protocol/test_vectors_v1.json`
- Airborne counterpart: workspace `../staged_avionics_tests/06_protocol_v1_telemetry/`

The 2026-07-13 handoff remains a historical record of monitoring, settings and attitude work. This file describes the newer Protocol v1 bidirectional-control, flight-session and statistics work.

## Implemented in this workspace

### Rust backend

- Protocol v1 parser now accepts both TELEMETRY and ACK and resynchronizes after bad frames.
- `CommandManager` encodes Protocol v1 SET_TIMER／FORCE_RELEASE frames and retries every 100 ms.
- Retries keep the same command ID, use a new frame sequence and set the retransmission flag.
- ACK must match current session, command ID and command type; old or duplicate ACK cannot corrupt pending state.
- FORCE_RELEASE has priority over timer, and only the latest timer remains pending.
- On airborne session change, pending FORCE is cancelled for safety and the latest timer is rebuilt with a new command ID.
- One serial task owns split read／write halves, so telemetry, ACK and command transmissions share the same port safely.
- Flight sessions create `flight_data.csv`, `system.log` and `session_summary.json` under the application data directory and flush each telemetry／event write.
- Statistics separate expected, lost, duplicate, CRC errors, link outages, maximum link loss and restart count.

Key files:

- `src-tauri/src/infrastructures/serial/command.rs`
- `src-tauri/src/infrastructures/serial/parser.rs`
- `src-tauri/src/infrastructures/serial/receiver.rs`
- `src-tauri/src/infrastructures/flight.rs`
- `src-tauri/src/commands/serial.rs`
- `src-tauri/src/state/serial_state.rs`

### Svelte frontend

- Timer setting／overwrite control.
- Safety lock plus one-click FORCE RELEASE that automatically relocks.
- Command sending／ACK／failure state and attempt count.
- Airborne session, remaining time, deploy state and last-packet time.
- Flight-session metadata: starting battery voltage, location, operator and notes.
- Separate telemetry／loss／duplicate／CRC／link-loss／restart statistics.

Key files:

- `src-ui/src/components/FlightControlPanel.svelte`
- `src-ui/src/lib/tauri.ts`
- `src-ui/src/lib/stores.svelte.ts`
- `src-ui/src/lib/types.ts`
- `src-ui/src/lib/flight-control.test.mjs`

## Verified on 2026-07-16

Run from the workspace root:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\verify_p0_workspace.ps1
```

Final result:

- Protocol v1 shared vectors: 7 passed, including duplicate semantics.
- Airborne static／production host checks: passed.
- Rust tests: 15 passed, 0 failed.
- `cargo check`: passed.
- Frontend tests: 33 passed, 0 failed.
- `svelte-check`: 0 errors, 0 warnings.
- Vite production build: passed.
- Tauri `build --no-bundle`: passed; output `src-tauri/target/release/app.exe`.
- Both ESP32 Stage 06 sketches compiled in the same final run.

## Packaged release

The verified current ground-station executable is fixed under `artifacts/`:

```text
artifacts/GroundStation_0.1.0_Portable_2026-07-16_125046.exe
```

- Size: 11,788,800 bytes
- SHA-256: `C5C37DD27D168B5C78CC110B9F555150B7B7349C5B7E6A4878C0AE50DF12FDF7`
- Manifest: `artifacts/GroundStation_0.1.0_Portable_2026-07-16_125046.json`
- Latest pointer: `artifacts/LATEST.txt`
- Rebuild and package after every ground-station source/configuration change with `.\package_ground_station.ps1`; do not distribute `src-tauri/target/release/app.exe` directly without the validation step.

Rust still reports five non-blocking pre-existing dead-code／unused warnings around `NotificationCenter`, `ErrorResponse` and `Parser::parse_to_payload`. They do not fail tests or builds; do not delete those APIs merely to silence warnings without checking whether another branch uses them.

## Manual tests still required

No E22／ESP32／servo result is recorded as passing by this handoff. Required order:

1. Read both E22 configurations and confirm M8N baud／NMEA.
2. Cold-boot SAFE／UNSET test with the real release mechanism removed.
3. Real 1 Hz telemetry and UI／Log field comparison.
4. Timer overwrite pressure, ACK pairing and latest-only behavior.
5. Lost-link countdown to one safe dummy-load release.
6. ESP32 #2 reboot and new-session timer resynchronization.
7. FORCE RELEASE safety lock, 10 cold-boot runs and controlled 20-duplicate delivery.
8. At least 30 minutes of bidirectional E22 bench operation.
9. Flight-folder readback and UI／CSV／Log statistics audit.

The authoritative procedure and pass criteria are in workspace `../P0_HARDWARE_TEST_PLAN.md`. Do not mark P0-01, P0-03, P0-08 or P0-11 complete before the corresponding evidence exists.

## Build notes

- The workspace verification script prepares the bundled Node／pnpm path before frontend and Tauri commands.
- Arduino CLI writes its inventory under the user Arduino data directory; a restricted sandbox may require explicit permission even though the sketch itself only writes build artifacts.
- Local Tauri CLI should be invoked from the repository root so it can discover `src-tauri/tauri.conf.json`.
- `src-tauri/tauri.workspace-build.json` is the workspace release override used by the root verification script.
- The UI remains relative-yaw only because MPU6050 has no magnetometer; this is unrelated to deployment control and must never be used as a release condition.

## Next handoff action

Execute the hardware plan in order and attach the resulting flight-session folder, E22 setting record, video／measurement evidence and PASS／FAIL notes. Update `PROJECT_WORK_LOG.md` only after each stated criterion is actually met.
