<script lang="ts">
  import { store } from '@/lib/stores.svelte';
  import { getStorageStatus, getTestSessionStatus, setupEventListeners } from '@/lib/tauri';
  import type { UnlistenFn } from '@tauri-apps/api/event';

  import ConnectionPanel from '@/components/ConnectionPanel.svelte';
  import FlightControlPanel from '@/components/FlightControlPanel.svelte';
  import TelemetryGrid from '@/components/TelemetryGrid.svelte';
  import TelemetryCharts from '@/components/TelemetryCharts.svelte';
  import TelemetryTable from '@/components/TelemetryTable.svelte';
  import GpsMap from '@/components/GpsMap.svelte';
  import AttitudeIndicator from '@/components/AttitudeIndicator.svelte';
  import StatusBar from '@/components/StatusBar.svelte';
  import TestSessionDialog from '@/components/TestSessionDialog.svelte';

  $effect(() => {
    let unlisteners: UnlistenFn[] = [];

    setupEventListeners(store).then((fns) => {
      unlisteners = fns;
      void Promise.all([getStorageStatus(), getTestSessionStatus()]).then(([storageStatus, sessionStatus]) => {
        store.updateStorageStatus(storageStatus);
        store.updateTestSessionStatus(sessionStatus);
      }).catch((error) => {
        store.addError({
          errorType: 'INITIALIZATION_ERROR',
          detail: error?.detail ?? error?.message ?? String(error),
        });
      });
    });

    return () => {
      unlisteners.forEach((fn) => fn());
    };
  });
</script>

<div class="app-layout">
  <aside class="sidebar-left" aria-label="場次與連線">
    <ConnectionPanel />
  </aside>

  <main class="center-area">
    <TelemetryGrid />
    <TelemetryCharts />
    <div class="lower-row">
      <AttitudeIndicator />
      <TelemetryTable />
    </div>
  </main>

  <aside class="sidebar-right" aria-label="定位與飛行控制">
    <GpsMap />
    <FlightControlPanel />
  </aside>

  <StatusBar />
  <TestSessionDialog />
</div>

<style>
  .app-layout {
    display: grid;
    grid-template-columns: clamp(216px, 17vw, 264px) minmax(0, 1fr) clamp(320px, 26vw, 400px);
    grid-template-rows: minmax(0, 1fr) auto;
    height: 100dvh;
    overflow: hidden;
    background: var(--paper);
  }

  .sidebar-left {
    grid-row: 1 / 3;
    min-height: 0;
    z-index: 2;
  }

  .center-area {
    display: grid;
    grid-template-rows: auto minmax(230px, 1fr) auto;
    min-width: 0;
    min-height: 0;
    overflow-y: auto;
    border-right: 1px solid var(--rule);
  }

  .lower-row {
    display: grid;
    grid-template-columns: minmax(270px, .42fr) minmax(0, 1fr);
    min-height: 0;
    border-top: 1px solid var(--rule);
  }

  .sidebar-right {
    display: grid;
    grid-template-rows: minmax(200px, 1fr) auto;
    min-width: 0;
    min-height: 0;
  }

  @media (max-width: 1360px) {
    .lower-row { grid-template-columns: minmax(230px, .34fr) minmax(0, 1fr); }
  }

  @media (max-width: 1180px) {
    .lower-row { grid-template-columns: 1fr; }
  }

  @media (max-width: 900px) {
    .app-layout {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: auto;
      height: auto;
      overflow: visible;
    }
    .sidebar-left { grid-row: auto; }
    .center-area { border-right: 0; overflow: visible; }
    .sidebar-right { grid-template-rows: 340px auto; border-top: 1px solid var(--rule); }
  }
</style>
