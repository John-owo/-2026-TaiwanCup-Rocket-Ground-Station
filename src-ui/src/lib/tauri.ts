import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  AirborneSessionChange,
  TelemetryPayload,
  PacketStats,
  SerialError,
  DbTelemetry,
  CommandStatus,
  FlightStats,
  FlightSessionMetadata,
  StorageStatus,
  TestSessionStatus,
} from './types';
import type { store as StoreType } from './stores.svelte';

export async function startTestMonitoring(
  path: string,
  baudRate: number,
  metadata: FlightSessionMetadata,
  allowUnrecorded: boolean,
): Promise<TestSessionStatus> {
  return await invoke<TestSessionStatus>('start_test_monitoring', {
    path,
    baudRate,
    metadata,
    allowUnrecorded,
  });
}

export async function listSerialPorts(): Promise<string[]> {
  return await invoke<string[]>('list_serial_ports');
}

export async function stopTestMonitoring(): Promise<TestSessionStatus> {
  return await invoke<TestSessionStatus>('stop_test_monitoring');
}

export async function getTestSessionStatus(): Promise<TestSessionStatus> {
  return await invoke<TestSessionStatus>('get_test_session_status');
}

export async function getStorageStatus(): Promise<StorageStatus> {
  return await invoke<StorageStatus>('get_storage_status');
}

export async function getTelemetryHistory(
  limit: number,
  testRunId?: string,
): Promise<DbTelemetry[]> {
  return await invoke<DbTelemetry[]>('get_telemetry_history', {
    limit,
    testRunId: testRunId ?? null,
  });
}

export async function setTimer(durationS: number): Promise<void> {
  await invoke('set_timer', { durationS });
}

export async function forceRelease(): Promise<void> {
  await invoke('force_release');
}

export async function getFlightStats(): Promise<FlightStats> {
  return await invoke<FlightStats>('get_flight_stats');
}

export async function setupEventListeners(
  appStore: typeof StoreType
): Promise<UnlistenFn[]> {
  const unlisteners: UnlistenFn[] = [];

  const unlistenTelemetry = await listen<TelemetryPayload>('update-telemetry', (event) => {
    appStore.updateTelemetry(event.payload);
  });
  unlisteners.push(unlistenTelemetry);

  const unlistenStats = await listen<PacketStats>('packet-stats', (event) => {
    appStore.updateStats(event.payload);
  });
  unlisteners.push(unlistenStats);

  const unlistenSession = await listen<AirborneSessionChange>('airborne-session-changed', (event) => {
    appStore.updateAirborneSession(event.payload);
  });
  unlisteners.push(unlistenSession);

  const unlistenCommand = await listen<CommandStatus>('command-status', (event) => {
    appStore.updateCommandStatus(event.payload);
  });
  unlisteners.push(unlistenCommand);

  const unlistenFlightStats = await listen<FlightStats>('flight-stats', (event) => {
    appStore.updateFlightStats(event.payload);
  });
  unlisteners.push(unlistenFlightStats);

  const unlistenTestSession = await listen<TestSessionStatus>('test-session-status', (event) => {
    appStore.updateTestSessionStatus(event.payload);
  });
  unlisteners.push(unlistenTestSession);

  const unlistenStorage = await listen<StorageStatus>('storage-status', (event) => {
    appStore.updateStorageStatus(event.payload);
  });
  unlisteners.push(unlistenStorage);

  const unlistenError = await listen<SerialError>('serial-error', (event) => {
    appStore.addError(event.payload);
    appStore.setConnected(false);
  });
  unlisteners.push(unlistenError);

  return unlisteners;
}
