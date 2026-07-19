<script lang="ts">
  import { store } from '@/lib/stores.svelte';
  import { getStorageStatus, getTestSessionStatus, setupEventListeners } from '@/lib/tauri';
  import type { UnlistenFn } from '@tauri-apps/api/event';

  import ConnectionPanel from '@/components/ConnectionPanel.svelte';
  import FlightControlPanel from '@/components/FlightControlPanel.svelte';
  import TelemetryGrid from '@/components/TelemetryGrid.svelte';
  import TelemetryCharts from '@/components/TelemetryCharts.svelte';
  import GpsMap from '@/components/GpsMap.svelte';
  import AttitudeIndicator from '@/components/AttitudeIndicator.svelte';
  import StatusBar from '@/components/StatusBar.svelte';
  import TestSessionDialog from '@/components/TestSessionDialog.svelte';

  let connected = $derived(store.connected);
  let packets = $derived(store.stats.totalPackets);
  let storage = $derived(store.storageStatus);
  let session = $derived(store.testSessionStatus);
  let purpose = $derived(session.purpose || '尚未開始測試');
  let runLabel = $derived(session.testRunId ? session.testRunId.slice(0, 8).toUpperCase() : '--');

  const phaseLabels = {
    disconnected: '待命',
    starting: '啟動中',
    recording: '記錄中',
    monitoring_unrecorded: '僅監控',
    finishing: '結束中',
    completed: '已完成',
    interrupted: '未完成',
    failed: '啟動失敗',
  } as const;

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
  <header class="top-bar">
    <div class="brand">
      <img class="brand-mark" src="/assets/5-space-emblem.png" alt="5 SPACE 隊徽" />
      <div class="brand-copy">
        <strong>GROUND STATION</strong>
        <span>Rocket Telemetry Console</span>
      </div>
    </div>

    <div class="run-identity">
      <span>目前測試場次</span>
      <h1>{purpose}</h1>
      <small class="mono">RUN {runLabel}</small>
    </div>

    <div class="system-state" aria-label="系統狀態">
      <span class="data-chip mono">封包 {packets.toLocaleString()}</span>
      <span class:healthy={storage.phase === 'healthy'} class:failed={storage.phase === 'failed'} class="storage-chip">
        {storage.phase === 'healthy' ? '儲存正常' : storage.phase === 'degraded' ? '儲存降級' : storage.phase === 'failed' ? '儲存失敗' : '儲存初始化'}
      </span>
      <span class:active={session.phase === 'recording' || session.phase === 'monitoring_unrecorded'} class="run-chip">
        {phaseLabels[session.phase]}
      </span>
      <span class:online={connected} class="connection-chip">
        <i aria-hidden="true"></i>{connected ? '已連線' : '未連線'}
      </span>
    </div>
  </header>

  <div class="main-content">
    <aside class="sidebar-left" aria-label="連線與測試設定">
      <ConnectionPanel />
    </aside>

    <main class="center-area">
      <TelemetryGrid />
      <TelemetryCharts />
      <AttitudeIndicator />
    </main>

    <aside class="sidebar-right" aria-label="定位與安全控制">
      <GpsMap />
      <FlightControlPanel />
    </aside>
  </div>

  <StatusBar />
  <TestSessionDialog />
</div>

<style>
  .app-layout {
    position: relative;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    min-height: 100dvh;
    max-height: 100dvh;
    overflow: hidden;
    background: var(--bg-gradient);
  }

  .top-bar {
    min-height: 78px;
    display: grid;
    grid-template-columns: minmax(300px, .9fr) minmax(300px, 1.2fr) minmax(400px, 1fr);
    align-items: center;
    gap: 18px;
    padding: 0 22px;
    border-bottom: 1px solid var(--border-muted);
    background: rgba(6, 14, 19, .86);
    backdrop-filter: blur(18px);
    z-index: 10;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }

  .brand-mark {
    width: 122px;
    height: 42px;
    object-fit: contain;
    flex: 0 0 auto;
  }

  .brand-copy { min-width: 0; }
  .brand-copy strong {
    display: block;
    font-size: 12px;
    font-weight: 650;
    letter-spacing: .13em;
  }
  .brand-copy span { color: var(--text-tertiary); font-size: 10px; letter-spacing: .06em; }

  .run-identity {
    min-width: 0;
    padding-left: 18px;
    border-left: 1px solid var(--border);
  }
  .run-identity > span { color: var(--text-secondary); font-size: 10px; letter-spacing: .08em; }
  .run-identity h1 {
    margin: 3px 0 0;
    overflow: hidden;
    color: var(--text-primary);
    font-size: clamp(16px, 1.4vw, 21px);
    font-weight: 560;
    letter-spacing: -.02em;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .run-identity small { display: block; margin-top: 2px; color: var(--text-tertiary); font-size: 9px; }

  .system-state {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .data-chip,
  .storage-chip,
  .run-chip,
  .connection-chip {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    min-height: 30px;
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-full);
    background: rgba(14, 27, 35, .7);
    color: var(--text-secondary);
    font-size: 10px;
    white-space: nowrap;
  }

  .storage-chip { color: var(--accent-orange); }
  .storage-chip.healthy,
  .run-chip.active { color: var(--accent-cyan); border-color: rgba(115, 210, 182, .28); }
  .storage-chip.failed { color: var(--accent-red); border-color: rgba(229, 109, 121, .34); }
  .connection-chip { color: var(--accent-red); }
  .connection-chip.online { color: var(--accent-cyan); }
  .connection-chip i {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
  }

  .main-content {
    display: grid;
    grid-template-columns: 244px minmax(500px, 1fr) minmax(330px, 380px);
    gap: 16px;
    min-height: 0;
    padding: 16px 18px;
    overflow: hidden;
  }

  .sidebar-left,
  .sidebar-right,
  .center-area {
    min-width: 0;
    min-height: 0;
    overflow-y: auto;
    scrollbar-gutter: stable;
  }

  .sidebar-left {
    position: relative;
    z-index: 2;
  }

  .sidebar-right,
  .center-area {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  @media (max-width: 1240px) {
    .top-bar { grid-template-columns: minmax(280px, 1fr) minmax(280px, 1fr); }
    .system-state { grid-column: 1 / -1; justify-content: flex-start; padding-bottom: 12px; }
    .main-content { grid-template-columns: 230px minmax(0, 1fr); overflow-y: auto; }
    .sidebar-left,
    .sidebar-right,
    .center-area { overflow: visible; }
    .sidebar-right { grid-column: 1 / -1; display: grid; grid-template-columns: minmax(0, 1.2fr) minmax(330px, .8fr); }
  }

  @media (max-width: 780px) {
    .app-layout { max-height: none; overflow: visible; }
    .top-bar { display: flex; flex-wrap: wrap; padding: 13px 14px; }
    .brand-copy { display: none; }
    .run-identity { order: 3; width: 100%; padding: 10px 0 0; border-top: 1px solid var(--border-muted); border-left: 0; }
    .system-state { margin-left: auto; padding: 0; }
    .data-chip,
    .run-chip { display: none; }
    .main-content { grid-template-columns: 1fr; padding: 12px; overflow: visible; }
    .sidebar-right { grid-column: auto; grid-template-columns: 1fr; }
  }
</style>
