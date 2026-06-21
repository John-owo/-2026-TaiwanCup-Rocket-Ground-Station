<script lang="ts">
  import { store } from '@/lib/stores.svelte';
  import type { TelemetryPayload } from '@/lib/types';

  let history: TelemetryPayload[] = $derived(store.history);

  // Chart dimensions
  const WIDTH = 600;
  const HEIGHT = 200;
  const PADDING = { top: 20, right: 20, bottom: 30, left: 55 };
  const CHART_W = WIDTH - PADDING.left - PADDING.right;
  const CHART_H = HEIGHT - PADDING.top - PADDING.bottom;
  const MAX_POINTS = 100;

  function getChartData(data: TelemetryPayload[], key: keyof TelemetryPayload) {
    const slice = data.slice(-MAX_POINTS);
    return slice.map(d => d[key] as number);
  }

  function buildPath(values: number[], minV: number, maxV: number): string {
    if (values.length < 2) return '';
    const range = maxV - minV || 1;
    const stepX = CHART_W / Math.max(values.length - 1, 1);

    return values.map((v, i) => {
      const x = PADDING.left + i * stepX;
      const y = PADDING.top + CHART_H - ((v - minV) / range) * CHART_H;
      return `${i === 0 ? 'M' : 'L'}${x.toFixed(1)},${y.toFixed(1)}`;
    }).join(' ');
  }

  function buildAreaPath(values: number[], minV: number, maxV: number): string {
    const linePath = buildPath(values, minV, maxV);
    if (!linePath) return '';
    const stepX = CHART_W / Math.max(values.length - 1, 1);
    const lastX = PADDING.left + (values.length - 1) * stepX;
    const baseY = PADDING.top + CHART_H;
    return `${linePath} L${lastX.toFixed(1)},${baseY} L${PADDING.left},${baseY} Z`;
  }

  function getGridLines(minV: number, maxV: number, count: number = 5) {
    const range = maxV - minV || 1;
    const step = range / (count - 1);
    return Array.from({ length: count }, (_, i) => {
      const value = minV + step * i;
      const y = PADDING.top + CHART_H - ((value - minV) / range) * CHART_H;
      return { value, y };
    });
  }

  // Altitude chart data
  let altValues = $derived(getChartData(history, 'altitude'));
  let altMin = $derived(altValues.length ? Math.min(...altValues, 0) : 0);
  let altMax = $derived(altValues.length ? Math.max(...altValues, 10) : 100);
  let altPath = $derived(buildPath(altValues, altMin, altMax));
  let altArea = $derived(buildAreaPath(altValues, altMin, altMax));
  let altGrid = $derived(getGridLines(altMin, altMax));

  // Velocity chart data
  let velValues = $derived(getChartData(history, 'verticalVelocity'));
  let velMin = $derived(velValues.length ? Math.min(...velValues, -10) : -10);
  let velMax = $derived(velValues.length ? Math.max(...velValues, 10) : 10);
  let velPath = $derived(buildPath(velValues, velMin, velMax));
  let velArea = $derived(buildAreaPath(velValues, velMin, velMax));
  let velGrid = $derived(getGridLines(velMin, velMax));
</script>

<div class="charts-container">
  <!-- Altitude Chart -->
  <div class="chart-card">
    <div class="chart-header">
      <div class="chart-title">
        <span class="dot cyan"></span>
        高度 Altitude
      </div>
      <span class="chart-latest mono">
        {altValues.length ? altValues[altValues.length - 1].toFixed(1) : '—'} m
      </span>
    </div>
    <svg viewBox="0 0 {WIDTH} {HEIGHT}" preserveAspectRatio="none" class="chart-svg">
      <defs>
        <linearGradient id="altGradient" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stop-color="var(--accent-cyan)" stop-opacity="0.3"/>
          <stop offset="100%" stop-color="var(--accent-cyan)" stop-opacity="0.02"/>
        </linearGradient>
        <linearGradient id="velGradient" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stop-color="var(--accent-green)" stop-opacity="0.3"/>
          <stop offset="100%" stop-color="var(--accent-green)" stop-opacity="0.02"/>
        </linearGradient>
      </defs>
      <!-- Grid lines -->
      {#each altGrid as line}
        <line x1={PADDING.left} y1={line.y} x2={PADDING.left + CHART_W} y2={line.y}
              stroke="var(--surface-border)" stroke-width="0.5"/>
        <text x={PADDING.left - 8} y={line.y + 4} text-anchor="end"
              fill="var(--text-tertiary)" font-size="9" font-family="var(--font-mono)">
          {line.value.toFixed(0)}
        </text>
      {/each}
      <!-- Area fill -->
      {#if altArea}
        <path d={altArea} fill="url(#altGradient)"/>
      {/if}
      <!-- Line -->
      {#if altPath}
        <path d={altPath} fill="none" stroke="var(--accent-cyan)" stroke-width="2"
              stroke-linecap="round" stroke-linejoin="round"/>
      {/if}
      <!-- Latest point -->
      {#if altValues.length > 0}
        {@const lastX = PADDING.left + (altValues.length - 1) * (CHART_W / Math.max(altValues.length - 1, 1))}
        {@const lastY = PADDING.top + CHART_H - ((altValues[altValues.length - 1] - altMin) / (altMax - altMin || 1)) * CHART_H}
        <circle cx={lastX} cy={lastY} r="4" fill="var(--accent-cyan)" opacity="0.9"/>
        <circle cx={lastX} cy={lastY} r="8" fill="var(--accent-cyan)" opacity="0.2"/>
      {/if}
      <!-- No data label -->
      {#if altValues.length === 0}
        <text x={WIDTH / 2} y={HEIGHT / 2} text-anchor="middle"
              fill="var(--text-tertiary)" font-size="13">等待數據...</text>
      {/if}
    </svg>
  </div>

  <!-- Velocity Chart -->
  <div class="chart-card">
    <div class="chart-header">
      <div class="chart-title">
        <span class="dot green"></span>
        垂直速度 Vertical Velocity
      </div>
      <span class="chart-latest mono">
        {velValues.length ? velValues[velValues.length - 1].toFixed(2) : '—'} m/s
      </span>
    </div>
    <svg viewBox="0 0 {WIDTH} {HEIGHT}" preserveAspectRatio="none" class="chart-svg">
      {#each velGrid as line}
        <line x1={PADDING.left} y1={line.y} x2={PADDING.left + CHART_W} y2={line.y}
              stroke="var(--surface-border)" stroke-width="0.5"/>
        <text x={PADDING.left - 8} y={line.y + 4} text-anchor="end"
              fill="var(--text-tertiary)" font-size="9" font-family="var(--font-mono)">
          {line.value.toFixed(0)}
        </text>
      {/each}
      {#if velArea}
        <path d={velArea} fill="url(#velGradient)"/>
      {/if}
      {#if velPath}
        <path d={velPath} fill="none" stroke="var(--accent-green)" stroke-width="2"
              stroke-linecap="round" stroke-linejoin="round"/>
      {/if}
      {#if velValues.length > 0}
        {@const lastX = PADDING.left + (velValues.length - 1) * (CHART_W / Math.max(velValues.length - 1, 1))}
        {@const lastY = PADDING.top + CHART_H - ((velValues[velValues.length - 1] - velMin) / (velMax - velMin || 1)) * CHART_H}
        <circle cx={lastX} cy={lastY} r="4" fill="var(--accent-green)" opacity="0.9"/>
        <circle cx={lastX} cy={lastY} r="8" fill="var(--accent-green)" opacity="0.2"/>
      {/if}
      {#if velValues.length === 0}
        <text x={WIDTH / 2} y={HEIGHT / 2} text-anchor="middle"
              fill="var(--text-tertiary)" font-size="13">等待數據...</text>
      {/if}
    </svg>
  </div>
</div>

<style>
  .charts-container {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--sp-4);
  }

  @media (max-width: 1200px) {
    .charts-container {
      grid-template-columns: 1fr;
    }
  }

  .chart-card {
    background: var(--glass-bg);
    backdrop-filter: var(--glass-blur);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-lg);
    padding: var(--sp-4);
    box-shadow: var(--glass-shadow);
    animation: slide-up 0.5s ease-out forwards;
    opacity: 0;
    animation-delay: 200ms;
  }

  .chart-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--sp-3);
  }

  .chart-title {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    font-size: var(--fs-sm);
    font-weight: 600;
    color: var(--text-primary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .dot.cyan {
    background: var(--accent-cyan);
    box-shadow: 0 0 6px var(--accent-cyan-glow);
  }

  .dot.green {
    background: var(--accent-green);
    box-shadow: 0 0 6px var(--accent-green-glow);
  }

  .chart-latest {
    font-size: var(--fs-md);
    font-weight: 600;
    color: var(--text-secondary);
  }

  .chart-svg {
    width: 100%;
    height: 180px;
    border-radius: var(--radius-sm);
    background: rgba(0, 0, 0, 0.2);
  }

  .mono {
    font-family: var(--font-mono);
  }
</style>
