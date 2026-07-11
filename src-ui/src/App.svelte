<script lang="ts">
  import { store } from '@/lib/stores.svelte';
  import { setupEventListeners } from '@/lib/tauri';
  import type { UnlistenFn } from '@tauri-apps/api/event';

  import ConnectionPanel from '@/components/ConnectionPanel.svelte';
  import TelemetryGrid from '@/components/TelemetryGrid.svelte';
  import TelemetryCharts from '@/components/TelemetryCharts.svelte';
  import GpsMap from '@/components/GpsMap.svelte';
  import AttitudeIndicator from '@/components/AttitudeIndicator.svelte';
  import StatusBar from '@/components/StatusBar.svelte';

  let connected = $derived(store.connected);
  let packets = $derived(store.stats.totalPackets);

  $effect(() => {
    let unlisteners: UnlistenFn[] = [];

    setupEventListeners(store).then((fns) => {
      unlisteners = fns;
    });

    return () => {
      unlisteners.forEach((fn) => fn());
    };
  });
</script>

<div class="app-layout">
  <header class="top-bar">
    <div class="brand">
      <div class="logo">
        <svg width="28" height="28" viewBox="0 0 32 32" fill="none" aria-hidden="true">
          <path d="M16 2L14 12L16 8L18 12L16 2Z" fill="var(--accent-cyan)" opacity="0.9"/>
          <path d="M16 8L12 24L16 20L20 24L16 8Z" fill="var(--accent-cyan)"/>
          <path d="M10 24L16 30L22 24L20 24L16 28L12 24Z" fill="var(--accent-cyan)" opacity="0.5"/>
          <circle cx="16" cy="16" r="14" stroke="var(--accent-cyan)" stroke-width="0.5" opacity="0.3"/>
        </svg>
      </div>
      <div class="brand-text">
        <h1>五限可能 火箭地面站</h1>
        <span class="brand-sub">2026 台灣盃火箭監控 · 相對高度</span>
      </div>
    </div>

    <div class="top-bar-right">
      <div class="packet-chip mono">封包 {packets.toLocaleString()}</div>
      <div class="status-badge" class:online={connected}>
        <div class="badge-dot"></div>
        <span>{connected ? '已連線' : '未連線'}</span>
      </div>
    </div>
  </header>

  <div class="main-content">
    <aside class="sidebar-left">
      <ConnectionPanel />
    </aside>

    <main class="center-area">
      <TelemetryGrid />
      <TelemetryCharts />
    </main>

    <aside class="sidebar-right">
      <GpsMap />
      <AttitudeIndicator />
    </aside>
  </div>

  <StatusBar />
</div>

<style>
  .app-layout {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
    max-height: 100vh;
    background: var(--bg-gradient);
    overflow: hidden;
    position: relative;
  }

  .app-layout::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: linear-gradient(90deg, transparent, var(--accent-cyan-dim), transparent);
    animation: scan-line 8s linear infinite;
    pointer-events: none;
    z-index: 100;
    opacity: 0.4;
  }

  .top-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--sp-3) var(--sp-5);
    background: var(--glass-bg);
    backdrop-filter: var(--glass-blur);
    border-bottom: 1px solid var(--glass-border);
    flex-shrink: 0;
    z-index: 10;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
  }

  .logo {
    display: flex;
    animation: glow 3s ease-in-out infinite;
  }

  .brand-text h1 {
    color: var(--text-primary);
    font-size: var(--fs-md);
    font-weight: 700;
    letter-spacing: 0.04em;
  }

  .brand-sub {
    color: var(--text-tertiary);
    font-size: var(--fs-xs);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .top-bar-right {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
  }

  .packet-chip {
    padding: var(--sp-1) var(--sp-3);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-full);
    color: var(--text-secondary);
    font-size: var(--fs-xs);
  }

  .status-badge {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    padding: var(--sp-1) var(--sp-3);
    background: var(--accent-red-dim);
    border: 1px solid rgba(255, 59, 59, 0.2);
    border-radius: var(--radius-full);
    color: var(--accent-red);
    font-family: var(--font-mono);
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.08em;
  }

  .status-badge.online {
    background: var(--accent-green-dim);
    border-color: rgba(0, 255, 136, 0.2);
    color: var(--accent-green);
  }

  .badge-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
    animation: pulse 2s ease-in-out infinite;
  }

  .main-content {
    display: grid;
    grid-template-columns: 260px minmax(0, 1fr) 300px;
    gap: var(--sp-4);
    flex: 1;
    min-height: 0;
    padding: var(--sp-4);
    overflow: hidden;
  }

  .sidebar-left,
  .sidebar-right,
  .center-area {
    min-height: 0;
    overflow-y: auto;
  }

  .sidebar-right {
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
  }

  .center-area {
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
    padding: 0 var(--sp-1);
  }

  @media (max-width: 1180px) {
    .main-content {
      grid-template-columns: 1fr;
      overflow-y: auto;
    }

    .sidebar-left,
    .sidebar-right,
    .center-area {
      overflow: visible;
      padding: 0;
    }
  }
</style>
