// Browser-only demonstration feed. It runs only when the UI is opened outside the
// Tauri shell (vite dev / preview), so the real app never sees synthetic data.
// Everything it shows is labelled 示範資料 in the status bar.
import type { store as StoreType } from './stores.svelte';
import type {
  FlightSessionMetadata,
  FlightStats,
  TelemetryPayload,
  TestSessionStatus,
} from './types';

type AppStore = typeof StoreType;

const PERIOD_MS = 600;
const BOOT_UPTIME_MS = 182_400;
const LAUNCH_AT_S = 6;
const BURNOUT_AT_S = 9;
const APOGEE_AT_S = 21;
const APOGEE_M = 648;
const DESCENT_MPS = 6.5;
const PAD = { lat: 22.341702, lng: 120.895612 };
const SESSION_ID = 0xb9f0b7f6;

let appStore: AppStore | null = null;
let timer: ReturnType<typeof setInterval> | undefined;
let startedAt = 0;
let seq = 0;
let timerDeadlineS: number | null = 26;
let deployed = false;
let commandId = 0;
let lastAck = { id: 0, result: 0xff };
let status: TestSessionStatus = {
  phase: 'disconnected',
  testRunId: null,
  directory: null,
  purpose: null,
  detail: null,
};

function altitudeAt(t: number) {
  if (t < LAUNCH_AT_S) return { altitude: 0, vv: 0 };
  if (t < BURNOUT_AT_S) {
    const tb = t - LAUNCH_AT_S;
    return { altitude: 15 * tb * tb, vv: 30 * tb };
  }
  if (t < APOGEE_AT_S) {
    const burnoutAlt = 15 * (BURNOUT_AT_S - LAUNCH_AT_S) ** 2;
    const span = APOGEE_AT_S - BURNOUT_AT_S;
    const k = (t - BURNOUT_AT_S) / span;
    const eased = 1 - (1 - k) ** 2;
    return { altitude: burnoutAlt + (APOGEE_M - burnoutAlt) * eased, vv: (APOGEE_M - burnoutAlt) * 2 * (1 - k) / span };
  }
  const altitude = Math.max(0, APOGEE_M - (t - APOGEE_AT_S) * DESCENT_MPS);
  return { altitude, vv: altitude > 0 ? -DESCENT_MPS : 0 };
}

function wobble(t: number, scale: number) {
  return Math.sin(t * 1.7) * scale + Math.sin(t * 4.3) * scale * 0.4;
}

function frame(t = (Date.now() - startedAt) / 1000): TelemetryPayload {
  const { altitude, vv } = altitudeAt(t);
  const boosting = t >= LAUNCH_AT_S && t < BURNOUT_AT_S;
  const airborne = altitude > 0.5;
  if (timerDeadlineS !== null && t >= timerDeadlineS) {
    deployed = true;
  }
  const drift = airborne ? Math.max(0, t - LAUNCH_AT_S) : 0;
  seq += 1;
  return {
    protocolVersion: 2,
    sessionId: SESSION_ID,
    frameSeq: seq,
    uptimeMs: BOOT_UPTIME_MS + Math.round(t * 1000),
    restartReason: 1,
    timerState: deployed ? 2 : timerDeadlineS === null ? 0 : 1,
    deployState: deployed ? 1 : 0,
    sensorFlags: 0b0111_1000,
    remainingS: timerDeadlineS === null ? 0 : Math.max(0, Math.ceil(timerDeadlineS - t)),
    lastAckCommandId: lastAck.id,
    lastAckResult: lastAck.result,
    xAcceleration: wobble(t, airborne ? 1.4 : 0.05),
    yAcceleration: wobble(t + 2, airborne ? 1.1 : 0.05),
    zAcceleration: boosting ? 39.8 + wobble(t, 3) : 9.81 + wobble(t + 1, airborne ? 0.6 : 0.03),
    xAngularVelocity: wobble(t, airborne ? 6 : 0.2),
    yAngularVelocity: wobble(t + 3, airborne ? 4 : 0.2),
    zAngularVelocity: airborne ? 28 + wobble(t, 9) : wobble(t, 0.2),
    longitude: PAD.lng + drift * 0.000031 + wobble(t, 0.0000006),
    latitude: PAD.lat + drift * 0.000042 + wobble(t + 5, 0.0000006),
    altitude: altitude + wobble(t, 0.25),
    groundSpeed: airborne ? 5.8 + wobble(t, 0.6) : 0,
    verticalVelocity: vv,
    airPressure: 1009.6 - altitude * 0.1145,
    temperature: 28.4 - altitude * 0.0065 + wobble(t, 0.1),
  };
}

function flightStats(): FlightStats {
  return {
    telemetryPackets: seq,
    expectedPackets: seq + 2,
    lostPackets: 2,
    duplicatePackets: 0,
    crcErrors: 0,
    linkOutages: 0,
    maxLinkLossMs: 1_900,
    restartCount: 0,
  };
}

function tick() {
  if (!appStore) return;
  appStore.updateTelemetry(frame());
  appStore.updateStats({ totalPackets: seq, failedPackets: 0, packetsPerSecond: 1000 / PERIOD_MS });
  appStore.updateFlightStats(flightStats());
}

function begin(purpose: string, seedSeconds = 0) {
  startedAt = Date.now() - seedSeconds * 1000;
  seq = 0;
  deployed = false;
  timerDeadlineS = 26;
  status = {
    phase: 'recording',
    testRunId: '81012096-c4d5-40cd-a28f-6c2e05a8983b',
    directory: 'C:\\示範\\flight_sessions\\81012096',
    purpose,
    detail: null,
  };
  appStore?.updateTestSessionStatus(status);
  clearInterval(timer);
  // Replay the flight up to the requested moment so a still capture shows a full profile.
  for (let t = 0; t < seedSeconds; t += PERIOD_MS / 1000) {
    appStore?.updateTelemetry(frame(t));
  }
  tick();
  timer = setInterval(tick, PERIOD_MS);
}

export function startDemo(target: AppStore) {
  appStore = target;
  target.setDemoMode(true);
  target.updateStorageStatus({
    phase: 'healthy',
    dataPath: 'C:\\示範\\flight_sessions',
    availableBytes: 214_000_000_000,
    queueDepth: 0,
    queueCapacity: 4096,
    lastWriteUnixMs: Date.now(),
    lastError: null,
    droppedWrites: 0,
  });
  const params = new URLSearchParams(location.search);
  if (params.get('demo') === 'flight') {
    const seed = Math.max(0, Math.min(180, Number(params.get('at')) || 0));
    begin('示範飛行：半雙工遙測與定時開傘', seed);
  }
  return () => clearInterval(timer);
}

export const demoApi = {
  async listSerialPorts() {
    return ['COM9', 'COM4'];
  },
  async startTestMonitoring(_path: string, _baud: number, metadata: FlightSessionMetadata) {
    begin(metadata.purpose);
    return status;
  },
  async stopTestMonitoring() {
    clearInterval(timer);
    status = { ...status, phase: 'completed', detail: '操作員停止監控' };
    return status;
  },
  async getTestSessionStatus() {
    return status;
  },
  async setTimer(durationS: number) {
    const t = (Date.now() - startedAt) / 1000;
    timerDeadlineS = t + durationS;
    commandId += 1;
    lastAck = { id: commandId, result: 0 };
    appStore?.updateCommandStatus({
      commandId,
      commandType: 'SET_TIMER',
      status: 'acked',
      attempts: 1,
      result: 0,
      detail: '示範：空中端已確認新的倒數',
    });
  },
  async forceRelease() {
    deployed = true;
    commandId += 1;
    lastAck = { id: commandId, result: 0 };
    appStore?.updateCommandStatus({
      commandId,
      commandType: 'FORCE_RELEASE',
      status: 'acked',
      attempts: 1,
      result: 0,
      detail: '示範：空中端已確認強制釋放',
    });
  },
};
