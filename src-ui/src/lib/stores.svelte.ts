import type { TelemetryPayload, PacketStats, SerialError } from './types';

const MAX_HISTORY = 200;

function createDefaultTelemetry(): TelemetryPayload {
  return {
    xAcceleration: 0,
    yAcceleration: 0,
    zAcceleration: 0,
    xAngularVelocity: 0,
    yAngularVelocity: 0,
    zAngularVelocity: 0,
    longitude: 0,
    latitude: 0,
    altitude: 0,
    groundSpeed: 0,
    verticalVelocity: 0,
    airPressure: 0,
    temperature: 0,
  };
}

function createStore() {
  let telemetry = $state<TelemetryPayload>(createDefaultTelemetry());
  let history = $state<TelemetryPayload[]>([]);
  let stats = $state<PacketStats>({ totalPackets: 0, failedPackets: 0, packetsPerSecond: 0 });
  let connected = $state(false);
  let errors = $state<SerialError[]>([]);

  return {
    get telemetry() { return telemetry; },
    get history() { return history; },
    get stats() { return stats; },
    get connected() { return connected; },
    get errors() { return errors; },

    updateTelemetry(payload: TelemetryPayload) {
      telemetry = { ...payload };
      history = [...history, { ...payload }].slice(-MAX_HISTORY);
    },

    updateStats(newStats: PacketStats) {
      stats = { ...newStats };
    },

    addError(error: SerialError) {
      errors = [...errors, error].slice(-50);
    },

    setConnected(val: boolean) {
      connected = val;
      if (!val) {
        stats = { totalPackets: 0, failedPackets: 0, packetsPerSecond: 0 };
      }
    },

    clearHistory() {
      history = [];
    },

    clearErrors() {
      errors = [];
    },

    reset() {
      telemetry = createDefaultTelemetry();
      history = [];
      stats = { totalPackets: 0, failedPackets: 0, packetsPerSecond: 0 };
      connected = false;
      errors = [];
    },
  };
}

export const store = createStore();
