<script lang="ts">
  import { store } from '@/lib/stores.svelte';
  import type { TelemetryPayload } from '@/lib/types';

  let history: TelemetryPayload[] = $derived(store.history);

  const WIDTH = 760;
  const HEIGHT = 236;
  const PADDING = { top: 18, right: 22, bottom: 30, left: 56 };
  const CHART_W = WIDTH - PADDING.left - PADDING.right;
  const CHART_H = HEIGHT - PADDING.top - PADDING.bottom;
  const MAX_POINTS = 100;

  function valuesFor(data: TelemetryPayload[], key: keyof TelemetryPayload): number[] {
    return data.slice(-MAX_POINTS).map((item) => Number(item[key]));
  }

  function pointPath(values: number[], minValue: number, maxValue: number): string {
    if (values.length < 2) return '';
    const range = maxValue - minValue || 1;
    const stepX = CHART_W / Math.max(values.length - 1, 1);
    return values.map((value, index) => {
      const x = PADDING.left + index * stepX;
      const y = PADDING.top + CHART_H - ((value - minValue) / range) * CHART_H;
      return `${index === 0 ? 'M' : 'L'}${x.toFixed(1)},${y.toFixed(1)}`;
    }).join(' ');
  }

  function gridLines(minValue: number, maxValue: number, count = 5) {
    const range = maxValue - minValue || 1;
    return Array.from({ length: count }, (_, index) => {
      const value = minValue + (range * index) / (count - 1);
      const y = PADDING.top + CHART_H - ((value - minValue) / range) * CHART_H;
      return { value, y };
    });
  }

  let altitudeValues = $derived(valuesFor(history, 'altitude'));
  let altitudeMin = $derived(altitudeValues.length ? Math.min(...altitudeValues, 0) : 0);
  let altitudeMax = $derived(altitudeValues.length ? Math.max(...altitudeValues, 10) : 100);
  let altitudePath = $derived(pointPath(altitudeValues, altitudeMin, altitudeMax));
  let altitudeArea = $derived(
    altitudePath
      ? `${altitudePath} L${PADDING.left + CHART_W},${PADDING.top + CHART_H} L${PADDING.left},${PADDING.top + CHART_H} Z`
      : '',
  );
  let altitudeGrid = $derived(gridLines(altitudeMin, altitudeMax));
  let latest = $derived(history.at(-1));
</script>

<article class="flight-chart">
  <section class="chart-section">
    <header class="chart-header">
      <div>
        <span class="eyebrow">FLIGHT PROFILE</span>
        <h2>高度軌跡</h2>
      </div>
      <div class="current-altitude">
        <span>相對高度</span>
        <strong class="mono">{latest ? latest.altitude.toFixed(1) : '--'} <small>m</small></strong>
      </div>
    </header>

    <div class="chart-frame">
      <svg viewBox="0 0 {WIDTH} {HEIGHT}" preserveAspectRatio="none" class="chart-svg" role="img" aria-label="最近一百筆遙測高度軌跡">
        <defs>
          <linearGradient id="altitude-fill" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stop-color="var(--accent-cyan)" stop-opacity="0.26" />
            <stop offset="100%" stop-color="var(--accent-cyan)" stop-opacity="0" />
          </linearGradient>
        </defs>
        {#each altitudeGrid as line}
          <line
            x1={PADDING.left}
            y1={line.y}
            x2={PADDING.left + CHART_W}
            y2={line.y}
            stroke="var(--surface-border)"
            stroke-width="1"
          />
          <text
            x={PADDING.left - 10}
            y={line.y + 4}
            text-anchor="end"
            fill="var(--text-tertiary)"
            font-size="10"
            font-family="var(--font-mono)"
          >{line.value.toFixed(0)}</text>
        {/each}
        {#if altitudePath}
          <path d={altitudeArea} fill="url(#altitude-fill)" />
          <path
            d={altitudePath}
            fill="none"
            stroke="var(--accent-cyan)"
            stroke-width="2.4"
            stroke-linecap="round"
            stroke-linejoin="round"
            vector-effect="non-scaling-stroke"
          />
        {:else}
          <text x={WIDTH / 2} y={HEIGHT / 2} text-anchor="middle" fill="var(--text-tertiary)" font-size="13">
            等待遙測資料…
          </text>
        {/if}
        <text x={PADDING.left} y={HEIGHT - 8} fill="var(--text-tertiary)" font-size="9">較早</text>
        <text x={PADDING.left + CHART_W} y={HEIGHT - 8} text-anchor="end" fill="var(--text-tertiary)" font-size="9">現在</text>
      </svg>
    </div>
  </section>

  <aside class="flight-summary" aria-label="即時飛行摘要">
    <header>
      <span class="eyebrow">LIVE SUMMARY</span>
      <small class="mono">最近 {Math.min(history.length, MAX_POINTS)} 筆</small>
    </header>
    <dl>
      <div>
        <dt>垂直速度</dt>
        <dd class="mono">{latest ? latest.verticalVelocity.toFixed(2) : '--'} <small>m/s</small></dd>
      </div>
      <div>
        <dt>地面速度</dt>
        <dd class="mono">{latest ? latest.groundSpeed.toFixed(1) : '--'} <small>m/s</small></dd>
      </div>
      <div>
        <dt>氣壓</dt>
        <dd class="mono">{latest ? latest.airPressure.toFixed(1) : '--'} <small>hPa</small></dd>
      </div>
      <div>
        <dt>溫度</dt>
        <dd class="mono">{latest ? latest.temperature.toFixed(1) : '--'} <small>°C</small></dd>
      </div>
    </dl>
  </aside>
</article>

<style>
  .flight-chart {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 188px;
    overflow: hidden;
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-lg);
    background: var(--glass-bg);
    box-shadow: var(--glass-shadow);
  }

  .chart-section { min-width: 0; padding: var(--sp-5); }
  .chart-header,
  .flight-summary header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--sp-3);
  }

  .eyebrow {
    display: block;
    color: var(--accent-cyan);
    font-family: var(--font-mono);
    font-size: 9px;
    letter-spacing: .14em;
  }

  h2 {
    margin-top: 3px;
    color: var(--text-primary);
    font-size: var(--fs-lg);
    font-weight: 560;
    letter-spacing: -.02em;
  }

  .current-altitude { text-align: right; }
  .current-altitude > span { display: block; color: var(--text-tertiary); font-size: var(--fs-xs); }
  .current-altitude strong { color: var(--text-primary); font-size: var(--fs-lg); font-weight: 520; }
  small { color: var(--text-tertiary); font-size: 9px; font-weight: 450; }

  .chart-frame {
    min-height: 210px;
    margin-top: var(--sp-3);
    border-top: 1px solid var(--border-muted);
    background-image: linear-gradient(90deg, transparent 49.8%, rgba(137, 169, 184, .045) 50%);
    background-size: 25% 100%;
  }

  .chart-svg { display: block; width: 100%; height: 220px; }

  .flight-summary {
    display: flex;
    flex-direction: column;
    padding: var(--sp-5);
    border-left: 1px solid var(--border-muted);
    background: rgba(5, 13, 18, .34);
  }
  .flight-summary header { align-items: center; }
  .flight-summary dl { display: grid; flex: 1; margin-top: var(--sp-4); }
  .flight-summary dl > div {
    display: flex;
    flex-direction: column;
    justify-content: center;
    border-top: 1px solid var(--border-muted);
  }
  .flight-summary dt { color: var(--text-tertiary); font-size: var(--fs-xs); }
  .flight-summary dd { margin-top: 3px; color: var(--text-primary); font-size: var(--fs-md); }

  @media (max-width: 940px) {
    .flight-chart { grid-template-columns: 1fr; }
    .flight-summary { border-top: 1px solid var(--border-muted); border-left: 0; }
    .flight-summary dl { grid-template-columns: repeat(4, 1fr); gap: var(--sp-3); }
    .flight-summary dl > div { padding-top: var(--sp-3); }
  }

  @media (max-width: 560px) {
    .chart-section,
    .flight-summary { padding: var(--sp-4); }
    .flight-summary dl { grid-template-columns: repeat(2, 1fr); }
  }

  @media (max-height: 900px) and (min-width: 1241px) {
    .chart-section,
    .flight-summary { padding: var(--sp-4); }
    .chart-frame { min-height: 168px; margin-top: var(--sp-2); }
    .chart-svg { height: 176px; }
    .flight-summary dl { margin-top: var(--sp-2); }
  }
</style>
