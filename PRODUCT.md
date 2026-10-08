# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

(Tauri v2 desktop shell on Windows; the UI is a Svelte 5 web view, so web conventions apply.)

## Users

- **Primary:** one or two team operators at the laptop during rocket bench tests and launch day. Their job is to start a recorded test session, confirm the radio link is alive, watch altitude, speed, attitude and GPS in real time, and issue the two uplink commands (`SET_TIMER`, `FORCE_RELEASE`) safely.
- **Secondary:** teammates standing behind the operator, and competition judges or an audience when the screen is projected or captured for a poster. Key flight numbers must read from a distance.

## Product Purpose

Receive, verify, display and record telemetry from the rocket's E22 LoRa downlink, and schedule safety-gated uplink commands in the half-duplex window. Success: the operator always knows whether the link is live, what the rocket is doing, whether the data is being saved, and whether the deploy mechanism is safe or armed — and never fires FORCE RELEASE by accident.

## Operating Context

- Used everywhere: indoors, in a tent, and outdoors in direct sunlight on a laptop. Glare and long sessions are both real.
- Session workflow: choose COM port and baud → fill mandatory test metadata dialog → monitor → optional timer/force commands → stop. Storage health and session identity are always visible.
- Telemetry arrives every ~1.8 s (Protocol v2); link loss threshold 4.5 s.
- Current phase (2026-10): preliminary judging, where the interface is shown on a poster and to judges. It must look good there while staying a working operator tool.

## Capabilities and Constraints

- Panels: connection/session setup, primary flight data (relative altitude, vertical speed, pressure, temperature, total acceleration, ground speed), altitude history chart, full 13-field telemetry grid, attitude estimate (roll/pitch/relative yaw from MPU6050, no magnetometer), Leaflet GPS map with track, flight-control (timer, session, deploy state, two-step FORCE RELEASE), status bar (link four-state, packets, parse failures, CRC, rate, uptime).
- Safety rules are product truth: FORCE stays locked unless live telemetry arrived within 4.5 s and the session is unchanged; sensors never trigger deployment; yaw is relative, never true north.
- Map tiles need internet; the rest must work offline.
- UI language: Traditional Chinese, with protocol/technical identifiers (COM, CRC, SET_TIMER, FORCE RELEASE, session IDs) kept as-is.

## Brand Commitments

None binding. The team name will change, so the UI must not hard-code a team name or emblem as identity; the current 5 SPACE emblem is not required.

## Evidence on Hand

- `docs/images/ground-station-day.png` / `ground-station-night.png`: current design, captured in browser demo mode (synthetic data).
- The previous AI-generated design (anti-reference only) is `docs/images/main-screen.png` at commit `d5431fb`.
- No real flight dataset is committed; demo values must be labelled as such and never presented as real flight results.

## Product Principles

1. Link truth first: live, waiting, lost and stale states are never ambiguous.
2. Safety controls are deliberate: arming and firing are separate, visible, and impossible to hit by accident.
3. Read from across the room: the few numbers that matter are large; diagnostics are quiet until something is wrong.
4. Works in sunlight and in the dark.

## Accessibility & Inclusion

State is never conveyed by colour alone (text + shape accompany colour). High contrast is needed for outdoor glare; respect reduced motion.
