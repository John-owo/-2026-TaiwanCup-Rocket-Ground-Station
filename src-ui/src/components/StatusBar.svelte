<script lang="ts">
  import { store } from '@/lib/stores.svelte';
  import { getTelemetryLinkState } from '@/lib/telemetry-link.js';
  import type { PacketStats } from '@/lib/types';

  let stats: PacketStats = $derived(store.stats);
  let connected = $derived(store.connected);
  let flightStats = $derived(store.flightStats);
  let nowMs = $state(Date.now());

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
    if (!connected) {
      nowMs = Date.now();
      return;
    }
    const interval = setInterval(() => {
      nowMs = Date.now();
    }, 250);
    return () => clearInterval(interval);
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

  let linkState = $derived(getTelemetryLinkState(connected, store.lastPacketAt, nowMs));
  let receiving = $derived(linkState === 'live');
  let statusLabel = $derived({
    standby: '待命',
    waiting: '等待資料',
    live: '接收中',
    lost: '失聯',
  }[linkState]);
</script>

<div class="status-bar">
  <div class="status-item">
    <div class="pulse-dot" class:active={receiving} class:lost={linkState === 'lost'}></div>
    <span class="status-label">{statusLabel}</span>
  </div>

  <div class="separator"></div>

  <div class="status-item">
    <span class="status-label">封包</span>
    <span class="status-value mono">{stats.totalPackets.toLocaleString()}</span>
  </div>

  <div class="separator"></div>

  <div class="status-item" class:warn={errorRateLevel === 'warn'} class:crit={errorRateLevel === 'crit'}>
    <span class="status-label">解析失敗</span>
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
    <span class="status-label">CRC 錯誤</span>
    <span class="status-value mono">{flightStats.crcErrors.toLocaleString()}</span>
  </div>

  <div class="separator"></div>

  <div class="status-item">
    <span class="status-label">頻率</span>
    <span class="status-value mono">{stats.packetsPerSecond.toFixed(1)} Hz</span>
  </div>

  <div class="separator"></div>

  <div class="status-item">
    <span class="status-label">運行時間</span>
    <span class="status-value mono">{elapsed}</span>
  </div>
</div>

<style>
  .status-bar {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    min-height: 32px;
    padding: 6px 22px;
    border-top: 1px solid var(--border-muted);
    background: rgba(5, 12, 17, .9);
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
    animation: pulse 1.5s ease-in-out infinite;
  }
  .pulse-dot.lost { background: var(--accent-red); box-shadow: 0 0 8px rgba(229, 109, 121, .45); }
</style>
