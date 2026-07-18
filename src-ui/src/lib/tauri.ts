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
} from './types';
import type { store as StoreType } from './stores.svelte';

export async function startMonitoring(path: string, baudRate: number): Promise<void> {
  await invoke('start_monitoring', { path, baudRate });
}

export async function listSerialPorts(): Promise<string[]> {
  return await invoke<string[]>('list_serial_ports');
}

export async function stopMonitoring(): Promise<void> {
  await invoke('stop_monitoring');
}

export async function getTelemetryHistory(limit: number): Promise<DbTelemetry[]> {
  return await invoke<DbTelemetry[]>('get_telemetry_history', { limit });
}

export async function setTimer(durationS: number): Promise<void> {
  await invoke('set_timer', { durationS });
}

export async function forceRelease(): Promise<void> {
  await invoke('force_release');
}

export async function startFlightSession(metadata: FlightSessionMetadata): Promise<string> {
  return await invoke<string>('start_flight_session', { metadata });
}

export async function stopFlightSession(): Promise<string> {
  return await invoke<string>('stop_flight_session');
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

  const unlistenError = await listen<SerialError>('serial-error', (event) => {
    appStore.addError(event.payload);
    appStore.setConnected(false);
  });
  unlisteners.push(unlistenError);

  return unlisteners;
}
