<script lang="ts">
  import { store } from '@/lib/stores.svelte';
  import { formatUptime, ladderFraction, niceAltitudeCeiling, TINT_BANDS } from '@/lib/flight-display.js';
  import type { TelemetryPayload } from '@/lib/types';

  let telemetry: TelemetryPayload = $derived(store.telemetry);
  let hasTelemetry = $derived(store.telemetryRevision > 0);
  let peak = $derived(store.peakAltitude);
  let ceiling = $derived(niceAltitudeCeiling(Math.max(peak ?? 0, telemetry.altitude)));
  let pinFraction = $derived(ladderFraction(telemetry.altitude, ceiling));
  let accelerationG = $derived(Math.sqrt(
    telemetry.xAcceleration ** 2
    + telemetry.yAcceleration ** 2
    + telemetry.zAcceleration ** 2,
  ) / 9.80665);
  let climbing = $derived(telemetry.verticalVelocity >= 0);

  const bands = Array.from({ length: TINT_BANDS }, (_, index) => TINT_BANDS - 1 - index);

  function formatValue(value: number, precision = 1): string {
    return hasTelemetry && Number.isFinite(value) ? value.toFixed(precision) : '--';
  }
</script>

<section class="readout" aria-label="主要飛行數據">
  <div class="altitude">
    <div class="ladder" aria-hidden="true">
      {#each bands as band}
        <span style:background="var(--t{band})"></span>
      {/each}
      {#if hasTelemetry}
        <i class="pin" style:bottom="{pinFraction * 100}%"></i>
      {/if}
    </div>
    <div class="ladder-scale num" aria-hidden="true">
      {#each bands as band}
        <span>{Math.round((ceiling / TINT_BANDS) * (band + 1))}</span>
      {/each}
      <span>0</span>
    </div>

    <div class="altitude-copy">
      <h2>相對高度</h2>
      <p class="value num"><strong>{formatValue(telemetry.altitude)}</strong><span>m</span></p>
      <div class="trend">
        <span class:climbing class:descending={!climbing && hasTelemetry}>
          <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true">
            <path d={climbing ? 'M7 12V2M2.5 6.5 7 2l4.5 4.5' : 'M7 2v10M2.5 7.5 7 12l4.5-4.5'} fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
          <b class="num">{hasTelemetry && climbing ? '+' : ''}{formatValue(telemetry.verticalVelocity)}</b>
          m/s 垂直速度
        </span>
        <span><b class="num">{peak === null ? '--' : peak.toFixed(1)}</b> m 本場最高</span>
        <span><b class="num">{hasTelemetry ? formatUptime(telemetry.uptimeMs) : '--'}</b> 空中端時間</span>
      </div>
    </div>
  </div>

  <dl class="secondary">
    <div><dt>總加速度</dt><dd class="num">{formatValue(accelerationG, 2)}<small>g</small></dd></div>
    <div><dt>地面速度</dt><dd class="num">{formatValue(telemetry.groundSpeed)}<small>m/s</small></dd></div>
    <div><dt>氣壓</dt><dd class="num">{formatValue(telemetry.airPressure)}<small>hPa</small></dd></div>
    <div><dt>溫度</dt><dd class="num">{formatValue(telemetry.temperature)}<small>°C</small></dd></div>
  </dl>
</section>

<style>
  .readout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) clamp(200px, 22vw, 300px);
    border-bottom: 1px solid var(--rule);
  }

  .altitude {
    display: grid;
    grid-template-columns: 40px auto minmax(0, 1fr);
    gap: 0 14px;
    padding: clamp(16px, 2.6vh, 26px) clamp(18px, 2vw, 28px);
  }

  .ladder {
    position: relative;
    display: grid;
    grid-template-rows: repeat(6, 1fr);
    min-height: 150px;
    border: 1px solid var(--rule-strong);
    border-radius: 3px;
  }
  .ladder span:first-child { border-radius: 2px 2px 0 0; }
  .ladder span:last-child { border-radius: 0 0 2px 2px; }

  .pin {
    position: absolute;
    left: -4px;
    right: -4px;
    height: 3px;
    margin-bottom: -1.5px;
    background: var(--rocket);
    transition: bottom 500ms var(--ease-out);
  }
  .pin::after {
    content: '';
    position: absolute;
    right: -10px;
    top: -5.5px;
    border: 7px solid transparent;
    border-left: 8px solid var(--rocket);
    border-right: 0;
  }

  .ladder-scale {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    margin: -5px 0;
    padding: 0 0 0 4px;
    color: var(--ink-3);
    font-size: 11px;
    font-weight: 500;
    line-height: 1;
  }

  .altitude-copy { min-width: 0; padding-left: 6px; }
  h2 { color: var(--ink-2); font-size: 15px; font-weight: 500; }

  .value { display: flex; align-items: baseline; gap: 10px; margin-top: 8px; }
  .value strong {
    font-size: clamp(72px, 8.6vw, 132px);
    font-weight: 600;
    line-height: .86;
    letter-spacing: -.035em;
  }
  .value span { color: var(--ink-2); font-size: clamp(20px, 1.8vw, 28px); font-weight: 600; }

  .trend {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 22px;
    margin-top: 16px;
    color: var(--ink-2);
    font-size: 13.5px;
  }
  .trend span { display: inline-flex; align-items: baseline; gap: 4px; }
  .trend svg { align-self: center; }
  .trend b { color: var(--ink); font-size: 20px; font-weight: 600; line-height: 1; }
  .trend .climbing,
  .trend .climbing b { color: var(--live); }
  .trend .descending,
  .trend .descending b { color: var(--warn); }

  .secondary {
    display: grid;
    grid-template-rows: repeat(4, 1fr);
    border-left: 1px solid var(--rule);
  }
  .secondary div {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 0 clamp(14px, 1.6vw, 22px);
    border-bottom: 1px solid var(--rule);
  }
  .secondary div:last-child { border-bottom: 0; }
  dt { color: var(--ink-2); font-size: 13.5px; }
  dd { font-size: clamp(20px, 1.9vw, 27px); font-weight: 600; line-height: 1; white-space: nowrap; }
  dd small { margin-left: 4px; color: var(--ink-3); font-size: 13px; font-weight: 500; }

  @media (max-width: 1180px) {
    .readout { grid-template-columns: 1fr; }
    .secondary { grid-template-columns: repeat(4, 1fr); grid-template-rows: none; border-left: 0; border-top: 1px solid var(--rule); }
    .secondary div { flex-direction: column; align-items: flex-start; justify-content: center; gap: 6px; padding: 12px 18px; border-bottom: 0; border-right: 1px solid var(--rule); }
    .secondary div:last-child { border-right: 0; }
  }

  @media (max-width: 560px) {
    .secondary { grid-template-columns: repeat(2, 1fr); }
  }

  @media (max-height: 820px) and (min-width: 1181px) {
    .ladder { min-height: 128px; }
    .value strong { font-size: clamp(64px, 7.4vw, 104px); }
    .trend { margin-top: 12px; }
  }
</style>
