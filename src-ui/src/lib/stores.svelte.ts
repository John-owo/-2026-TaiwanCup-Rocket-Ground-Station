import {
  DEFAULT_SETTINGS,
  loadSettings,
  saveSettings,
  setAxisSign,
  swapAxisSource,
} from './settings.js';
import type {
  AppSettings,
  AxisMapping,
  AxisSign,
  PacketStats,
  SensorAxis,
  SerialError,
  TelemetryPayload,
} from './types';

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
  const storage = typeof localStorage === 'undefined' ? undefined : localStorage;
  let telemetry = $state<TelemetryPayload>(createDefaultTelemetry());
  let telemetryRevision = $state(0);
  let history = $state<TelemetryPayload[]>([]);
  let stats = $state<PacketStats>({ totalPackets: 0, failedPackets: 0, packetsPerSecond: 0 });
  let connected = $state(false);
  let errors = $state<SerialError[]>([]);
  let settings = $state<AppSettings>(loadSettings(storage));
  let settingsRevision = $state(0);
  let axisMappingRevision = $state(0);

  function persistSettings(next: AppSettings, axisChanged = false) {
    settings = saveSettings(storage, next);
    settingsRevision += 1;
    if (axisChanged) axisMappingRevision += 1;
  }

  return {
    get telemetry() { return telemetry; },
    get telemetryRevision() { return telemetryRevision; },
    get history() { return history; },
    get stats() { return stats; },
    get connected() { return connected; },
    get errors() { return errors; },
    get settings() { return settings; },
    get settingsRevision() { return settingsRevision; },
    get axisMappingRevision() { return axisMappingRevision; },

    updateConnectionSettings(next: { portPath?: string; baudRate?: number }) {
      persistSettings({ ...settings, ...next });
    },

    updateAxisSource(bodyAxis: SensorAxis, source: SensorAxis) {
      persistSettings({
        ...settings,
        axisMapping: swapAxisSource(settings.axisMapping, bodyAxis, source),
      }, true);
    },

    updateAxisSign(bodyAxis: SensorAxis, sign: AxisSign) {
      persistSettings({
        ...settings,
        axisMapping: setAxisSign(settings.axisMapping, bodyAxis, sign),
      }, true);
    },

    resetAxisMapping() {
      persistSettings({
        ...settings,
        axisMapping: structuredClone(DEFAULT_SETTINGS.axisMapping) as AxisMapping,
      }, true);
    },

    resetSettings() {
      persistSettings(structuredClone(DEFAULT_SETTINGS), true);
    },

    updateTelemetry(payload: TelemetryPayload) {
      telemetry = { ...payload };
      history = [...history, { ...payload }].slice(-MAX_HISTORY);
      telemetryRevision += 1;
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
      telemetryRevision = 0;
      history = [];
      stats = { totalPackets: 0, failedPackets: 0, packetsPerSecond: 0 };
      connected = false;
      errors = [];
    },
  };
}

export const store = createStore();
