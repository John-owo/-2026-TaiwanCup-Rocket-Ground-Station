<script lang="ts">
  import { store } from '@/lib/stores.svelte';
  import type { PacketStats } from '@/lib/types';

  let stats: PacketStats = $derived(store.stats);
  let connected = $derived(store.connected);

  let errorRate = $derived(
    stats.totalPackets > 0 ? (stats.failedPackets / stats.totalPackets) * 100 : 0
  );

  let errorRateLevel = $derived(
    errorRate > 10 ? 'crit' : errorRate > 5 ? 'warn' : 'normal'
  );

  let connectTime = $state<number | null>(null);
  let elapsed = $state('00:00:00');

  $effect(() => {
    if (connected && !connectTime) {
      connectTime = Date.now();
    } else if (!connected) {
      connectTime = null;
      elapsed = '00:00:00';
    }
  });

  $effect(() => {
    if (!connected || !connectTime) return;

    const interval = setInterval(() => {
      const diff = Math.floor((Date.now() - connectTime!) / 1000);
      const h = String(Math.floor(diff / 3600)).padStart(2, '0');
      const m = String(Math.floor((diff % 3600) / 60)).padStart(2, '0');
      const s = String(diff % 60).padStart(2, '0');
      elapsed = `${h}:${m}:${s}`;
    }, 1000);

    return () => clearInterval(interval);
  });

  let receiving = $derived(connected && stats.totalPackets > 0);
</script>

<div class="status-bar">
  <div class="status-item">
    <div class="pulse-dot" class:active={receiving}></div>
    <span class="status-label">{connected ? 'RECEIVING' : 'STANDBY'}</span>
  </div>

  <div class="separator"></div>

  <div class="status-item">
    <span class="status-label">Packets</span>
    <span class="status-value mono">{stats.totalPackets.toLocaleString()}</span>
  </div>

  <div class="separator"></div>

  <div class="status-item" class:warn={errorRateLevel === 'warn'} class:crit={errorRateLevel === 'crit'}>
    <span class="status-label">CRC Fail</span>
    <span
      class="status-value mono"
      class:text-red={errorRateLevel === 'crit'}
      class:text-orange={errorRateLevel === 'warn'}
      class:text-green={errorRateLevel === 'normal'}
    >
      {errorRate.toFixed(1)}%
    </span>
    <span class="error-count mono">({stats.failedPackets})</span>
  </div>

  <div class="separator"></div>

  <div class="status-item">
    <span class="status-label">Rate</span>
    <span class="status-value mono">{stats.packetsPerSecond.toFixed(1)} Hz</span>
  </div>

  <div class="separator"></div>

  <div class="status-item">
    <span class="status-label">Runtime</span>
    <span class="status-value mono">{elapsed}</span>
  </div>
</div>

<style>
  .status-bar {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    min-height: 36px;
    padding: var(--sp-2) var(--sp-5);
    border-top: 1px solid var(--glass-border);
    background: var(--glass-bg);
    backdrop-filter: var(--glass-blur);
    flex-shrink: 0;
    overflow-x: auto;
  }

  .status-item {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    white-space: nowrap;
  }

  .separator {
    width: 1px;
    height: 16px;
    background: var(--surface-border);
    flex-shrink: 0;
  }

  .status-label {
    color: var(--text-secondary);
    font-size: var(--fs-xs);
    font-weight: 500;
  }

  .status-value {
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: var(--fs-sm);
    font-weight: 600;
  }

  .error-count {
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    font-size: var(--fs-xs);
  }

  .pulse-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--text-tertiary);
    transition: background var(--transition-base), box-shadow var(--transition-base);
  }

  .pulse-dot.active {
    background: var(--accent-green);
    box-shadow: 0 0 8px var(--accent-green-glow);
    animation: pulse 1.5s ease-in-out infinite;
  }
</style>
