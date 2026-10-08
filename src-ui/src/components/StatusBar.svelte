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
  let statusLabel = $derived({
    standby: '待命',
    waiting: '等待資料',
    live: '接收中',
    lost: '失聯',
  }[linkState]);
</script>

<footer class="status-bar" aria-label="通訊診斷">
  <span class="item link" data-state={linkState}><i aria-hidden="true"></i>{statusLabel}</span>
  <span class="item">封包<b class="num">{stats.totalPackets.toLocaleString()}</b></span>
  <span class="item" class:warn={errorRateLevel === 'warn'} class:crit={errorRateLevel === 'crit'}>
    解析失敗<b class="num">{errorRate.toFixed(1)}%</b><small class="num">({stats.failedPackets})</small>
  </span>
  <span class="item">CRC 錯誤<b class="num">{flightStats.crcErrors.toLocaleString()}</b></span>
  <span class="item">遺失<b class="num">{flightStats.lostPackets}</b></span>
  <span class="item">重複<b class="num">{flightStats.duplicatePackets}</b></span>
  <span class="item">失聯<b class="num">{flightStats.linkOutages}</b><small>次，最長</small><b class="num">{(flightStats.maxLinkLossMs / 1000).toFixed(1)} s</b></span>
  <span class="item">重啟<b class="num">{flightStats.restartCount}</b></span>
  <span class="item">頻率<b class="num">{stats.packetsPerSecond.toFixed(2)} Hz</b></span>
  <span class="item">運行時間<b class="num">{elapsed}</b></span>
  {#if store.demoMode}
    <span class="demo" title="瀏覽器預覽：未連接 Tauri 後端，數值為模擬飛行">示範資料</span>
  {/if}
</footer>

<style>
  .status-bar {
    grid-column: 2 / -1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: clamp(14px, 1.4vw, 22px);
    min-height: 34px;
    padding: 0 clamp(14px, 1.6vw, 22px);
    overflow-x: auto;
    border-top: 1px solid var(--rule);
    background: var(--paper-2);
    color: var(--ink-2);
    font-size: 12.5px;
    white-space: nowrap;
    scrollbar-width: none;
  }

  .item { display: inline-flex; align-items: baseline; gap: 6px; }
  b { color: var(--ink); font-size: 13.5px; font-weight: 600; }
  small { color: var(--ink-3); font-size: 11.5px; }
  .warn b { color: var(--warn); }
  .crit b { color: var(--danger); }

  .link { align-items: center; color: var(--ink); font-weight: 600; }
  .link i {
    width: 9px;
    height: 9px;
    border: 1.5px solid var(--ink-3);
    border-radius: 50%;
  }
  .link[data-state="live"] i { border-color: var(--live); background: var(--live); }
  .link[data-state="waiting"] i { border-style: dashed; }
  .link[data-state="lost"] { color: var(--danger); }
  .link[data-state="lost"] i { border-color: var(--danger); background: var(--danger); }

  .demo { margin-left: auto; padding-left: 12px; color: var(--warn); font-weight: 700; }

  @media (max-width: 900px) {
    .status-bar { grid-column: auto; }
  }
</style>
