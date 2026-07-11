import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { TelemetryPayload, PacketStats, SerialError, DbTelemetry } from './types';
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

  const unlistenError = await listen<SerialError>('serial-error', (event) => {
    appStore.addError(event.payload);
    appStore.setConnected(false);
  });
  unlisteners.push(unlistenError);

  return unlisteners;
}
