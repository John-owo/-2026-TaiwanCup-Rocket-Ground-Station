import {
  DEFAULT_SETTINGS,
  loadSettings,
  saveSettings,
  setAxisSign,
  swapAxisSource,
} from './settings.js';
import type {
  AirborneSessionChange,
  AppSettings,
  AxisMapping,
  AxisSign,
  PacketStats,
  SensorAxis,
  SerialError,
  TelemetryPayload,
  CommandStatus,
  FlightStats,
  StorageStatus,
  TestSessionStatus,
  TestStartRequest,
} from './types';

const MAX_HISTORY = 200;

function createDefaultTelemetry(): TelemetryPayload {
  return {
    protocolVersion: 1,
    sessionId: 0,
    frameSeq: 0,
    uptimeMs: 0,
    restartReason: 0,
    timerState: 0,
    deployState: 0,
    sensorFlags: 0,
    remainingS: 0,
    lastAckCommandId: 0,
    lastAckResult: 0xFF,
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
  let airborneSessionChange = $state<AirborneSessionChange | null>(null);
  let airborneRestartCount = $state(0);
  let commandStatus = $state<CommandStatus | null>(null);
  let flightStats = $state<FlightStats>({
    telemetryPackets: 0,
    expectedPackets: 0,
    lostPackets: 0,
    duplicatePackets: 0,
    crcErrors: 0,
    linkOutages: 0,
    maxLinkLossMs: 0,
    restartCount: 0,
  });
  let lastPacketAt = $state<number | null>(null);
  let flightSessionDirectory = $state('');
  let testSessionStatus = $state<TestSessionStatus>({
    phase: 'disconnected',
    testRunId: null,
    directory: null,
    purpose: null,
    detail: null,
  });
  let storageStatus = $state<StorageStatus>({
    phase: 'initializing',
    dataPath: '',
    availableBytes: null,
    queueDepth: 0,
    queueCapacity: 4096,
    lastWriteUnixMs: null,
    lastError: null,
    droppedWrites: 0,
  });
  let testStartRequest = $state<TestStartRequest | null>(null);
  let nextTestStartRequestId = 1;

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
    get airborneSessionChange() { return airborneSessionChange; },
    get airborneRestartCount() { return airborneRestartCount; },
    get commandStatus() { return commandStatus; },
    get flightStats() { return flightStats; },
    get lastPacketAt() { return lastPacketAt; },
    get flightSessionDirectory() { return flightSessionDirectory; },
    get testSessionStatus() { return testSessionStatus; },
    get storageStatus() { return storageStatus; },
    get testStartRequest() { return testStartRequest; },

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
      lastPacketAt = Date.now();
    },

    updateAirborneSession(change: AirborneSessionChange) {
      airborneSessionChange = { ...change };
      if (change.previousSessionId !== null) airborneRestartCount += 1;
    },

    updateStats(newStats: PacketStats) {
      stats = { ...newStats };
    },

    updateCommandStatus(status: CommandStatus) {
      commandStatus = { ...status };
    },

    updateFlightStats(nextStats: FlightStats) {
      flightStats = { ...nextStats };
    },

    setFlightSessionDirectory(directory: string) {
      flightSessionDirectory = directory;
    },

    requestTestStart(path: string, baudRate: number) {
      testStartRequest = { id: nextTestStartRequestId++, path, baudRate };
    },

    cancelTestStart() {
      testStartRequest = null;
    },

    updateTestSessionStatus(status: TestSessionStatus) {
      testSessionStatus = { ...status };
      flightSessionDirectory = status.directory ?? flightSessionDirectory;
      const nextConnected = status.phase === 'recording'
        || status.phase === 'monitoring_unrecorded'
        || status.phase === 'finishing';
      if (nextConnected && !connected) {
        stats = { totalPackets: 0, failedPackets: 0, packetsPerSecond: 0 };
        lastPacketAt = null;
        commandStatus = null;
        airborneSessionChange = null;
      }
      connected = nextConnected;
      if (connected) testStartRequest = null;
    },

    updateStorageStatus(status: StorageStatus) {
      storageStatus = { ...status };
    },

    addError(error: SerialError) {
      errors = [...errors, error].slice(-50);
    },

    setConnected(val: boolean) {
      connected = val;
      if (!val) {
        stats = { totalPackets: 0, failedPackets: 0, packetsPerSecond: 0 };
        airborneSessionChange = null;
        airborneRestartCount = 0;
        commandStatus = null;
        lastPacketAt = null;
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
      airborneSessionChange = null;
      airborneRestartCount = 0;
      commandStatus = null;
      flightStats = {
        telemetryPackets: 0,
        expectedPackets: 0,
        lostPackets: 0,
        duplicatePackets: 0,
        crcErrors: 0,
        linkOutages: 0,
        maxLinkLossMs: 0,
        restartCount: 0,
      };
      lastPacketAt = null;
      flightSessionDirectory = '';
      testSessionStatus = {
        phase: 'disconnected',
        testRunId: null,
        directory: null,
        purpose: null,
        detail: null,
      };
      testStartRequest = null;
    },
  };
}

export const store = createStore();
