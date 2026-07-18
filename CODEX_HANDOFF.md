# Codex Handoff

Read the current ground-station handoff before making changes:

```text
docs/handoff/2026-07-16-p0-bidirectional-control-handoff.md
```

Historical monitoring／attitude context remains in:

```text
docs/handoff/2026-07-13-codex-handoff.md
```

Continue from branch `feature/ground-station-upgrade`. The workspace contains pre-existing and current uncommitted changes; do not reset, discard, stage or commit them without reviewing scope.

The 2026-07-16 automated Protocol, Rust, frontend, Tauri, and ESP32 checks passed. Real E22, M8N, servo, and end-to-end UI results remain `NOT RUN`; follow workspace `../P0_HARDWARE_TEST_PLAN.md` and do not report them as passed until evidence is saved.

## Release packaging rule

After every ground-station source/configuration change, run `.\package_ground_station.ps1` from `ground_station/`. It must pass Rust tests/check, frontend tests/check/build and Tauri release build before it copies a versioned executable into `ground_station/artifacts/`. The script reads the release version from `src-tauri/tauri.conf.json`, writes a timestamped `.exe`, a validation manifest and `LATEST.txt`; do not distribute a manually copied or unverified `app.exe`.
