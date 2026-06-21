<script lang="ts">
  import { store } from '@/lib/stores.svelte';
  import type { PacketStats } from '@/lib/types';

  let stats: PacketStats = $derived(store.stats);
  let connected = $derived(store.connected);

  let errorRate = $derived(
    stats.totalPackets > 0
      ? ((stats.failedPackets / stats.totalPackets) * 100)
      : 0
  );

  let errorRateLevel = $derived(
    errorRate > 10 ? 'crit' : errorRate > 5 ? 'warn' : 'normal'
  );

  // Connection timer
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

  // Update elapsed time every second
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

  // Compute Hz from stats
  let hz = $derived(stats.packetsPerSecond);

  // Determine if we're receiving data (approximate - based on total packets changing)
  let receiving = $derived(connected && stats.totalPackets > 0);
</script>

<div class="status-bar">
  <!-- Connection indicator -->
  <div class="status-item">
    <div class="pulse-dot" class:active={receiving}></div>
    <span class="status-label">{connected ? '接收中' : '離線'}</span>
  </div>

  <div class="separator"></div>

  <!-- Total packets -->
  <div class="status-item">
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--accent-cyan)" stroke-width="2">
      <path d="M22 12h-4l-3 9L9 3l-3 9H2"/>
    </svg>
    <span class="status-label">封包</span>
    <span class="status-value mono">{stats.totalPackets.toLocaleString()}</span>
  </div>

  <div class="separator"></div>

  <!-- Error rate -->
  <div class="status-item" class:warn={errorRateLevel === 'warn'} class:crit={errorRateLevel === 'crit'}>
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none"
         stroke={errorRateLevel === 'crit' ? 'var(--accent-red)' : errorRateLevel === 'warn' ? 'var(--accent-orange)' : 'var(--accent-green)'}
         stroke-width="2">
      <path d="M12 9v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
    </svg>
    <span class="status-label">CRC 錯誤</span>
    <span class="status-value mono" class:text-red={errorRateLevel === 'crit'}
          class:text-orange={errorRateLevel === 'warn'}
          class:text-green={errorRateLevel === 'normal'}>
      {errorRate.toFixed(1)}%
    </span>
    <span class="error-count mono">({stats.failedPackets})</span>
  </div>

  <div class="separator"></div>

  <!-- Hz -->
  <div class="status-item">
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--accent-cyan)" stroke-width="2">
      <circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/>
    </svg>
    <span class="status-label">頻率</span>
    <span class="status-value mono">{hz.toFixed(1)} Hz</span>
  </div>

  <div class="separator"></div>

  <!-- Duration -->
  <div class="status-item">
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--text-secondary)" stroke-width="2">
      <rect x="3" y="4" width="18" height="18" rx="2"/><line x1="16" y1="2" x2="16" y2="6"/><line x1="8" y1="2" x2="8" y2="6"/>
    </svg>
    <span class="status-label">連線時間</span>
    <span class="status-value mono">{elapsed}</span>
  </div>
</div>

<style>
  .status-bar {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    padding: var(--sp-2) var(--sp-5);
    background: var(--glass-bg);
    backdrop-filter: var(--glass-blur);
    border-top: 1px solid var(--glass-border);
    min-height: 36px;
    flex-shrink: 0;
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
    font-size: var(--fs-xs);
    color: var(--text-secondary);
    font-weight: 500;
  }

  .status-value {
    font-size: var(--fs-sm);
    color: var(--text-primary);
    font-weight: 600;
    font-family: var(--font-mono);
  }

  .error-count {
    font-size: var(--fs-xs);
    color: var(--text-tertiary);
    font-family: var(--font-mono);
  }

  .pulse-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--text-tertiary);
    transition: all var(--transition-base);
  }

  .pulse-dot.active {
    background: var(--accent-green);
    box-shadow: 0 0 8px var(--accent-green-glow);
    animation: pulse 1.5s ease-in-out infinite;
  }

  .text-red { color: var(--accent-red); }
  .text-orange { color: var(--accent-orange); }
  .text-green { color: var(--accent-green); }

  .mono {
    font-family: var(--font-mono);
  }
</style>
