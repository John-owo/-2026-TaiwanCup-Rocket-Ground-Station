<script lang="ts">
  import { store } from '@/lib/stores.svelte';
  import { currentSessionRun, niceAltitudeCeiling, TINT_BANDS } from '@/lib/flight-display.js';
  import type { TelemetryPayload } from '@/lib/types';

  // The plot is drawn in a 1000×1000 unit box stretched over the canvas; tick labels
  // live in fixed-pixel gutters so they never collide when the panel gets short.
  const UNITS = 1000;
  const MAX_POINTS = 100;

  let run: TelemetryPayload[] = $derived(currentSessionRun(store.history).slice(-MAX_POINTS));
  let latest = $derived(run.at(-1));
  let ceiling = $derived(niceAltitudeCeiling(Math.max(store.peakAltitude ?? 0, ...run.map((sample) => sample.altitude))));
  let floor = $derived(Math.min(0, ...run.map((sample) => sample.altitude)));
  let spanMs = $derived(run.length > 1 ? Math.max(1_000, run.at(-1)!.uptimeMs - run[0].uptimeMs) : 1_000);

  function x(sample: TelemetryPayload) {
    const startMs = run[0]?.uptimeMs ?? 0;
    return ((sample.uptimeMs - startMs) / spanMs) * UNITS;
  }

  function y(altitude: number) {
    return UNITS - ((altitude - floor) / (ceiling - floor || 1)) * UNITS;
  }

  let path = $derived(
    run.length > 1
      ? run.map((sample, index) => `${index === 0 ? 'M' : 'L'}${x(sample).toFixed(1)},${y(sample.altitude).toFixed(1)}`).join(' ')
      : '',
  );

  let bands = $derived(Array.from({ length: TINT_BANDS }, (_, index) => {
    const low = (ceiling / TINT_BANDS) * index;
    const high = (ceiling / TINT_BANDS) * (index + 1);
    return { index, top: y(high), height: y(low) - y(high) };
  }));

  let ticks = $derived(Array.from({ length: TINT_BANDS + 1 }, (_, index) => {
    const value = (ceiling / TINT_BANDS) * index;
    return { value, y: y(value) };
  }));

  let timeTicks = $derived(Array.from({ length: 4 }, (_, index) => {
    const offsetS = -(spanMs / 1000) * (1 - index / 3);
    return { x: (UNITS * index) / 3, label: index === 3 ? '現在' : `${Math.round(offsetS)} s` };
  }));

  let peakSample = $derived(run.reduce<TelemetryPayload | undefined>(
    (best, sample) => (!best || sample.altitude > best.altitude ? sample : best),
    undefined,
  ));
</script>

<section class="profile" aria-label="高度剖面">
  <header>
    <h2>高度剖面</h2>
    <p>
      {#if latest}
        最高 <b class="num">{(peakSample?.altitude ?? 0).toFixed(1)} m</b> · 依空中端時間繪製 · 最近 {run.length} 筆
      {:else}
        依空中端時間繪製
      {/if}
    </p>
  </header>

  <div class="plot">
    <div class="canvas">
      <svg viewBox="0 0 {UNITS} {UNITS}" preserveAspectRatio="none" role="img" aria-label="最近遙測的高度隨時間變化">
        {#each bands as band}
          <rect x="0" y={band.top} width={UNITS} height={band.height} style:fill="var(--t{band.index})" style:opacity="var(--tint-opacity)" />
        {/each}
        <line x1="0" y1={y(0)} x2={UNITS} y2={y(0)} stroke="var(--rule-strong)" stroke-width="1" vector-effect="non-scaling-stroke" />
        {#if path}
          <path d={path} fill="none" stroke="var(--rocket)" stroke-width="3" stroke-linejoin="round" stroke-linecap="round" vector-effect="non-scaling-stroke" />
        {/if}
      </svg>
      {#if latest && run.length > 1}
        <i class="dot" style:left="{x(latest) / 10}%" style:top="{y(latest.altitude) / 10}%"></i>
        <span class="now-label num" style:top="{y(latest.altitude) / 10}%">{latest.altitude.toFixed(1)} m</span>
      {/if}
      {#if run.length < 2}
        <p class="empty">等待遙測資料，收到兩筆以上後開始繪製</p>
      {/if}
    </div>

    <div class="y-axis" aria-hidden="true">
      {#each ticks as tick}
        <span class="num" style:top="{tick.y / 10}%">{Math.round(tick.value)}</span>
      {/each}
    </div>
    {#if run.length > 1}
      <div class="x-axis" aria-hidden="true">
        {#each timeTicks as tick, index}
          <span class="num" class:first={index === 0} class:last={index === timeTicks.length - 1} style:left="{tick.x / 10}%">{tick.label}</span>
        {/each}
      </div>
    {/if}
  </div>
</section>

<style>
  .profile {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    min-height: 0;
    padding: 14px clamp(18px, 2vw, 28px) 8px;
  }

  header { display: flex; justify-content: space-between; align-items: baseline; gap: 16px; }
  h2 { font-size: 15px; font-weight: 700; white-space: nowrap; }
  header p { overflow: hidden; color: var(--ink-3); font-size: 12.5px; text-align: right; text-overflow: ellipsis; white-space: nowrap; }
  header b { color: var(--rocket); font-weight: 600; }

  .plot {
    --gutter-left: 40px;
    --gutter-right: 66px;
    --gutter-bottom: 20px;
    --gutter-top: 8px;
    position: relative;
    min-height: 0;
    margin-top: 6px;
  }

  .canvas {
    position: absolute;
    inset: var(--gutter-top) var(--gutter-right) var(--gutter-bottom) var(--gutter-left);
  }
  svg { position: absolute; inset: 0; width: 100%; height: 100%; overflow: visible; }

  .y-axis {
    position: absolute;
    top: var(--gutter-top);
    bottom: var(--gutter-bottom);
    left: 0;
    width: calc(var(--gutter-left) - 8px);
  }
  .y-axis span,
  .x-axis span {
    position: absolute;
    color: var(--ink-3);
    font-size: 12px;
    line-height: 1;
    white-space: nowrap;
  }
  .y-axis span { right: 0; font-size: 11px; transform: translateY(-50%); }

  .x-axis {
    position: absolute;
    left: var(--gutter-left);
    right: var(--gutter-right);
    bottom: 0;
    height: calc(var(--gutter-bottom) - 6px);
  }
  .x-axis span { bottom: 0; transform: translateX(-50%); }
  .x-axis span.first { transform: none; }
  .x-axis span.last { transform: translateX(-100%); }

  .dot {
    position: absolute;
    width: 11px;
    height: 11px;
    margin: -5.5px 0 0 -5.5px;
    border: 2px solid var(--paper);
    border-radius: 50%;
    background: var(--rocket);
  }
  .now-label {
    position: absolute;
    left: calc(100% + 10px);
    transform: translateY(-50%);
    color: var(--rocket);
    font-size: 13px;
    font-weight: 600;
    white-space: nowrap;
  }

  .empty {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    color: var(--ink-2);
    font-size: 13.5px;
  }
</style>
