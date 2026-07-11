<script lang="ts">
  import { store } from '@/lib/stores.svelte';
  import type { TelemetryPayload } from '@/lib/types';

  let history: TelemetryPayload[] = $derived(store.history);

  const WIDTH = 600;
  const HEIGHT = 200;
  const PADDING = { top: 20, right: 20, bottom: 30, left: 55 };
  const CHART_W = WIDTH - PADDING.left - PADDING.right;
  const CHART_H = HEIGHT - PADDING.top - PADDING.bottom;
  const MAX_POINTS = 100;

  function getChartData(data: TelemetryPayload[], key: keyof TelemetryPayload) {
    return data.slice(-MAX_POINTS).map((item) => item[key] as number);
  }

  function buildPath(values: number[], minV: number, maxV: number): string {
    if (values.length < 2) return '';
    const range = maxV - minV || 1;
    const stepX = CHART_W / Math.max(values.length - 1, 1);

    return values.map((value, index) => {
      const x = PADDING.left + index * stepX;
      const y = PADDING.top + CHART_H - ((value - minV) / range) * CHART_H;
      return `${index === 0 ? 'M' : 'L'}${x.toFixed(1)},${y.toFixed(1)}`;
    }).join(' ');
  }

  function getGridLines(minV: number, maxV: number, count = 5) {
    const range = maxV - minV || 1;
    const step = range / (count - 1);
    return Array.from({ length: count }, (_, index) => {
      const value = minV + step * index;
      const y = PADDING.top + CHART_H - ((value - minV) / range) * CHART_H;
      return { value, y };
    });
  }

  let altValues = $derived(getChartData(history, 'altitude'));
  let altMin = $derived(altValues.length ? Math.min(...altValues, 0) : 0);
  let altMax = $derived(altValues.length ? Math.max(...altValues, 10) : 100);
  let altPath = $derived(buildPath(altValues, altMin, altMax));
  let altGrid = $derived(getGridLines(altMin, altMax));

  let velValues = $derived(getChartData(history, 'verticalVelocity'));
  let velMin = $derived(velValues.length ? Math.min(...velValues, -10) : -10);
  let velMax = $derived(velValues.length ? Math.max(...velValues, 10) : 10);
  let velPath = $derived(buildPath(velValues, velMin, velMax));
  let velGrid = $derived(getGridLines(velMin, velMax));
</script>

<div class="charts-container">
  <article class="chart-card">
    <div class="chart-header">
      <div class="chart-title">
        <span class="dot cyan"></span>
        相對高度
      </div>
      <span class="chart-latest mono">
        {altValues.length ? altValues[altValues.length - 1].toFixed(1) : '--'} m
      </span>
    </div>
    <svg viewBox="0 0 {WIDTH} {HEIGHT}" preserveAspectRatio="none" class="chart-svg">
      {#each altGrid as line}
        <line x1={PADDING.left} y1={line.y} x2={PADDING.left + CHART_W} y2={line.y}
              stroke="var(--surface-border)" stroke-width="0.5"/>
        <text x={PADDING.left - 8} y={line.y + 4} text-anchor="end"
              fill="var(--text-tertiary)" font-size="9" font-family="var(--font-mono)">
          {line.value.toFixed(0)}
        </text>
      {/each}
      {#if altPath}
        <path d={altPath} fill="none" stroke="var(--accent-cyan)" stroke-width="2"
              stroke-linecap="round" stroke-linejoin="round"/>
      {:else}
        <text x={WIDTH / 2} y={HEIGHT / 2} text-anchor="middle"
              fill="var(--text-tertiary)" font-size="13">等待遙測資料…</text>
      {/if}
    </svg>
  </article>

  <article class="chart-card">
    <div class="chart-header">
      <div class="chart-title">
        <span class="dot green"></span>
        垂直速度
      </div>
      <span class="chart-latest mono">
        {velValues.length ? velValues[velValues.length - 1].toFixed(2) : '--'} m/s
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
      {#if velPath}
        <path d={velPath} fill="none" stroke="var(--accent-green)" stroke-width="2"
              stroke-linecap="round" stroke-linejoin="round"/>
      {:else}
        <text x={WIDTH / 2} y={HEIGHT / 2} text-anchor="middle"
              fill="var(--text-tertiary)" font-size="13">等待遙測資料…</text>
      {/if}
    </svg>
  </article>
</div>

<style>
  .charts-container {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--sp-4);
  }

  .chart-card {
    padding: var(--sp-4);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-lg);
    background: var(--glass-bg);
    backdrop-filter: var(--glass-blur);
    box-shadow: var(--glass-shadow);
    opacity: 0;
    animation: slide-up 0.5s ease-out forwards;
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
    color: var(--text-primary);
    font-size: var(--fs-sm);
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
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
    color: var(--text-secondary);
    font-size: var(--fs-md);
    font-weight: 600;
  }

  .chart-svg {
    width: 100%;
    height: 180px;
    border-radius: var(--radius-sm);
    background: rgba(0, 0, 0, 0.2);
  }

  @media (max-width: 1200px) {
    .charts-container {
      grid-template-columns: 1fr;
    }
  }
</style>
