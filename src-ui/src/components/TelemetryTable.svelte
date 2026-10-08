<script lang="ts">
  import { store } from '@/lib/stores.svelte';
  import type { TelemetryPayload } from '@/lib/types';

  interface TelemetryField {
    key: keyof TelemetryPayload;
    label: string;
    unit: string;
    precision: number;
    warnThreshold?: number;
    critThreshold?: number;
  }

  // Three columns read like a chart legend: IMU, then flight/position, then environment.
  const columns: TelemetryField[][] = [
    [
      { key: 'xAcceleration', label: '加速度 X', unit: 'm/s²', precision: 2, warnThreshold: 20, critThreshold: 50 },
      { key: 'yAcceleration', label: '加速度 Y', unit: 'm/s²', precision: 2, warnThreshold: 20, critThreshold: 50 },
      { key: 'zAcceleration', label: '加速度 Z', unit: 'm/s²', precision: 2, warnThreshold: 20, critThreshold: 50 },
      { key: 'xAngularVelocity', label: '角速度 X', unit: '°/s', precision: 1, warnThreshold: 200, critThreshold: 500 },
      { key: 'yAngularVelocity', label: '角速度 Y', unit: '°/s', precision: 1, warnThreshold: 200, critThreshold: 500 },
    ],
    [
      { key: 'zAngularVelocity', label: '角速度 Z', unit: '°/s', precision: 1, warnThreshold: 200, critThreshold: 500 },
      { key: 'altitude', label: '相對高度', unit: 'm', precision: 2, warnThreshold: 1000, critThreshold: 3000 },
      { key: 'verticalVelocity', label: '垂直速度', unit: 'm/s', precision: 2, warnThreshold: 50, critThreshold: 200 },
      { key: 'groundSpeed', label: '地面速度', unit: 'm/s', precision: 2, warnThreshold: 100, critThreshold: 300 },
      { key: 'remainingS', label: '倒數剩餘', unit: 's', precision: 0 },
    ],
    [
      { key: 'longitude', label: '經度', unit: '°', precision: 6 },
      { key: 'latitude', label: '緯度', unit: '°', precision: 6 },
      { key: 'airPressure', label: '氣壓', unit: 'hPa', precision: 1 },
      { key: 'temperature', label: '溫度', unit: '°C', precision: 1, warnThreshold: 50, critThreshold: 80 },
    ],
  ];

  let telemetry: TelemetryPayload = $derived(store.telemetry);
  let hasTelemetry = $derived(store.telemetryRevision > 0);
  let sessionHex = $derived(
    telemetry.sessionId ? `0x${telemetry.sessionId.toString(16).toUpperCase().padStart(8, '0')}` : '--',
  );

  function level(field: TelemetryField, value: number): 'normal' | 'warn' | 'crit' {
    const abs = Math.abs(value);
    if (field.critThreshold && abs >= field.critThreshold) return 'crit';
    if (field.warnThreshold && abs >= field.warnThreshold) return 'warn';
    return 'normal';
  }

  function format(value: number, precision: number): string {
    return hasTelemetry && Number.isFinite(value) ? value.toFixed(precision) : '--';
  }
</script>

<section class="table" aria-label="全部遙測">
  <header>
    <h2>全部遙測</h2>
    <p>13 項 · 封包 <span class="num">#{hasTelemetry ? telemetry.frameSeq : '--'}</span> · Session <span class="num">{sessionHex}</span></p>
  </header>

  <div class="columns">
    {#each columns as column}
      <dl>
        {#each column as field}
          {@const value = telemetry[field.key] as number}
          {@const state = level(field, value)}
          <div class:warn={state === 'warn'} class:crit={state === 'crit'}>
            <dt>{field.label}</dt>
            <dd class="num">{format(value, field.precision)}<small>{field.unit}</small></dd>
          </div>
        {/each}
      </dl>
    {/each}
  </div>
</section>

<style>
  .table {
    min-width: 0;
    padding: 14px clamp(16px, 2vw, 26px) 10px;
    overflow: hidden;
  }

  header { display: flex; justify-content: space-between; align-items: baseline; gap: 16px; }
  h2 { font-size: 15px; font-weight: 700; white-space: nowrap; }
  header p { overflow: hidden; color: var(--ink-3); font-size: 12px; text-overflow: ellipsis; white-space: nowrap; }

  .columns {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0 clamp(14px, 2vw, 28px);
    margin-top: 8px;
  }
  .table { overflow: visible; }

  dl div {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
    padding: 3.5px 0;
    border-bottom: 1px solid var(--rule);
  }
  dt { color: var(--ink-2); font-size: 13px; white-space: nowrap; }
  dd { font-size: 14.5px; font-weight: 500; white-space: nowrap; }
  dd small { display: inline-block; min-width: 34px; margin-left: 5px; color: var(--ink-3); font-size: 11.5px; }
  .warn dd { color: var(--warn); font-weight: 700; }
  .crit dd { color: var(--danger); font-weight: 700; }
  .warn dt::after,
  .crit dt::after { margin-left: 6px; font-size: 11px; font-weight: 700; }
  .warn dt::after { content: '偏高'; color: var(--warn); }
  .crit dt::after { content: '超限'; color: var(--danger); }

  @media (max-width: 1360px) {
    .columns { grid-template-columns: repeat(2, minmax(0, 1fr)); }
    .table { padding-top: 10px; padding-bottom: 8px; }
    dl div { padding: 2px 0; }
    dt { font-size: 12.5px; }
    dd { font-size: 13px; }
  }

  @media (max-width: 560px) {
    .columns { grid-template-columns: 1fr; }
  }
</style>
