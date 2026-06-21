<script lang="ts">
  import { store } from '@/lib/stores.svelte';
  import { setupEventListeners } from '@/lib/tauri';
  import type { UnlistenFn } from '@tauri-apps/api/event';

  import ConnectionPanel from '@/components/ConnectionPanel.svelte';
  import TelemetryGrid from '@/components/TelemetryGrid.svelte';
  import TelemetryCharts from '@/components/TelemetryCharts.svelte';
  import AttitudeIndicator from '@/components/AttitudeIndicator.svelte';
  import StatusBar from '@/components/StatusBar.svelte';

  let connected = $derived(store.connected);

  // Initialize Tauri event listeners when component mounts
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
  <!-- ─── Top Bar ─────────────────────────────────────────────── -->
  <header class="top-bar">
    <div class="brand">
      <div class="logo">
        <svg width="28" height="28" viewBox="0 0 32 32" fill="none">
          <path d="M16 2L14 12L16 8L18 12L16 2Z" fill="var(--accent-cyan)" opacity="0.9"/>
          <path d="M16 8L12 24L16 20L20 24L16 8Z" fill="var(--accent-cyan)"/>
          <path d="M10 24L16 30L22 24L20 24L16 28L12 24Z" fill="var(--accent-cyan)" opacity="0.5"/>
          <circle cx="16" cy="16" r="14" stroke="var(--accent-cyan)" stroke-width="0.5" opacity="0.3"/>
        </svg>
      </div>
      <div class="brand-text">
        <h1>PENTAX Ground Station</h1>
        <span class="brand-sub">2026 TaiwanCup Rocket Monitoring</span>
      </div>
    </div>
    <div class="top-bar-right">
      <div class="status-badge" class:online={connected}>
        <div class="badge-dot"></div>
        <span>{connected ? 'ONLINE' : 'OFFLINE'}</span>
      </div>
    </div>
  </header>

  <!-- ─── Main Content ────────────────────────────────────────── -->
  <div class="main-content">
    <!-- Left Sidebar -->
    <aside class="sidebar-left">
      <ConnectionPanel />
    </aside>

    <!-- Center Area -->
    <main class="center-area">
      <section class="telemetry-section">
        <TelemetryGrid />
      </section>
      <section class="charts-section">
        <TelemetryCharts />
      </section>
    </main>

    <!-- Right Sidebar -->
    <aside class="sidebar-right">
      <AttitudeIndicator />
    </aside>
  </div>

  <!-- ─── Status Bar ──────────────────────────────────────────── -->
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

  /* Subtle animated scan line effect */
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

  /* ─── Top Bar ───────────────────────────────────────── */
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
    font-size: var(--fs-md);
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: 0.04em;
  }

  .brand-sub {
    font-size: var(--fs-xs);
    color: var(--text-tertiary);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .top-bar-right {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
  }

  .status-badge {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    padding: var(--sp-1) var(--sp-3);
    background: var(--accent-red-dim);
    border: 1px solid rgba(255, 59, 59, 0.2);
    border-radius: var(--radius-full);
    font-size: var(--fs-xs);
    font-weight: 600;
    color: var(--accent-red);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-family: var(--font-mono);
    transition: all var(--transition-base);
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

  /* ─── Main Content ──────────────────────────────────── */
  .main-content {
    display: grid;
    grid-template-columns: 260px 1fr 260px;
    gap: var(--sp-4);
    padding: var(--sp-4);
    flex: 1;
    overflow: hidden;
    min-height: 0;
  }

  .sidebar-left {
    overflow-y: auto;
    padding-right: var(--sp-1);
  }

  .center-area {
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
    overflow-y: auto;
    padding: 0 var(--sp-1);
  }

  .sidebar-right {
    overflow-y: auto;
    padding-left: var(--sp-1);
  }

  .telemetry-section,
  .charts-section {
    flex-shrink: 0;
  }

  /* ─── Responsive ────────────────────────────────────── */
  @media (max-width: 1400px) {
    .main-content {
      grid-template-columns: 240px 1fr 220px;
    }
  }

  @media (max-width: 1100px) {
    .main-content {
      grid-template-columns: 1fr;
      grid-template-rows: auto 1fr auto;
    }

    .sidebar-left,
    .sidebar-right {
      padding: 0;
    }
  }
</style>
