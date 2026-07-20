<script lang="ts">
  import { store } from '@/lib/stores.svelte';
  import type { TelemetryPayload } from '@/lib/types';

  let telemetry: TelemetryPayload = $derived(store.telemetry);
  let connected = $derived(store.connected);
  let accelerationMagnitude = $derived(Math.sqrt(
    telemetry.xAcceleration ** 2
    + telemetry.yAcceleration ** 2
    + telemetry.zAcceleration ** 2,
  ));

  interface TelemetryField {
    key: keyof TelemetryPayload;
    label: string;
    unit: string;
    category: 'imu' | 'gps' | 'env';
    precision?: number;
    warnThreshold?: number;
    critThreshold?: number;
    icon: string;
  }

  const fields: TelemetryField[] = [
    { key: 'xAcceleration', label: 'X 軸加速度', unit: 'm/s²', category: 'imu', precision: 2, warnThreshold: 20, critThreshold: 50, icon: 'AX' },
    { key: 'yAcceleration', label: 'Y 軸加速度', unit: 'm/s²', category: 'imu', precision: 2, warnThreshold: 20, critThreshold: 50, icon: 'AY' },
    { key: 'zAcceleration', label: 'Z 軸加速度', unit: 'm/s²', category: 'imu', precision: 2, warnThreshold: 20, critThreshold: 50, icon: 'AZ' },
    { key: 'xAngularVelocity', label: 'X 軸角速度', unit: 'deg/s', category: 'imu', precision: 2, warnThreshold: 200, critThreshold: 500, icon: 'GX' },
    { key: 'yAngularVelocity', label: 'Y 軸角速度', unit: 'deg/s', category: 'imu', precision: 2, warnThreshold: 200, critThreshold: 500, icon: 'GY' },
    { key: 'zAngularVelocity', label: 'Z 軸角速度', unit: 'deg/s', category: 'imu', precision: 2, warnThreshold: 200, critThreshold: 500, icon: 'GZ' },
    { key: 'longitude', label: '經度', unit: 'deg', category: 'gps', precision: 6, icon: 'LON' },
    { key: 'latitude', label: '緯度', unit: 'deg', category: 'gps', precision: 6, icon: 'LAT' },
    { key: 'altitude', label: '相對高度', unit: 'm', category: 'gps', precision: 2, warnThreshold: 1000, critThreshold: 3000, icon: 'ALT' },
    { key: 'groundSpeed', label: '地面速度', unit: 'm/s', category: 'gps', precision: 2, warnThreshold: 100, critThreshold: 300, icon: 'GS' },
    { key: 'verticalVelocity', label: '垂直速度', unit: 'm/s', category: 'gps', precision: 2, warnThreshold: 50, critThreshold: 200, icon: 'VV' },
    { key: 'airPressure', label: '氣壓', unit: 'hPa', category: 'env', precision: 1, icon: 'P' },
    { key: 'temperature', label: '溫度', unit: '°C', category: 'env', precision: 1, warnThreshold: 50, critThreshold: 80, icon: 'T' },
  ];

  const categories = [
    { id: 'imu', label: 'IMU 感測器' },
    { id: 'gps', label: '飛行與定位' },
    { id: 'env', label: '環境感測' },
  ] as const;

  function getLevel(field: TelemetryField, value: number): 'normal' | 'warn' | 'crit' {
    const abs = Math.abs(value);
    if (field.critThreshold && abs >= field.critThreshold) return 'crit';
    if (field.warnThreshold && abs >= field.warnThreshold) return 'warn';
    return 'normal';
  }

  function formatValue(value: number, precision = 2): string {
    return Number.isFinite(value) ? value.toFixed(precision) : '--';
  }
</script>

<section class="primary-telemetry">
  <header class="telemetry-header">
    <div>
      <span class="section-kicker">主要飛行數據</span>
      <h2>相對高度</h2>
    </div>
    <span class:active={connected} class="live-state"><i aria-hidden="true"></i>{connected ? 'LIVE' : 'WAITING'}</span>
  </header>

  <div class="hero-reading">
    <span class="reading-label">ALTITUDE ABOVE GROUND</span>
    <div class="reading-value">
      <strong>{formatValue(telemetry.altitude, 1)}</strong><span>m</span>
    </div>
    <div class:descending={telemetry.verticalVelocity < 0} class="vertical-trend">
      <i aria-hidden="true"></i>{telemetry.verticalVelocity >= 0 ? '+' : ''}{formatValue(telemetry.verticalVelocity, 1)} m/s 垂直速度
    </div>
  </div>

  <div class="metric-strip">
    <div><span>氣壓</span><strong>{formatValue(telemetry.airPressure, 1)}</strong><small>hPa</small></div>
    <div><span>溫度</span><strong>{formatValue(telemetry.temperature, 1)}</strong><small>°C</small></div>
    <div><span>總加速度</span><strong>{formatValue(accelerationMagnitude / 9.80665, 2)}</strong><small>g</small></div>
    <div><span>地面速度</span><strong>{formatValue(telemetry.groundSpeed, 1)}</strong><small>m/s</small></div>
  </div>

  <details class="sensor-details">
    <summary>查看全部 13 項遙測</summary>
    <div class="telemetry-grid">
      {#each categories as cat}
        <section class="category-section">
          <h3>{cat.label}</h3>
          <div class="fields-grid">
            {#each fields.filter((field) => field.category === cat.id) as field}
              {@const value = telemetry[field.key] as number}
              {@const level = getLevel(field, value)}
              <article class:warn={level === 'warn'} class:crit={level === 'crit'} class="field-row">
                <span class="field-icon mono">{field.icon}</span>
                <span class="field-label">{field.label}</span>
                <strong class="mono">{formatValue(value, field.precision)}</strong>
                <small>{field.unit}</small>
              </article>
            {/each}
          </div>
        </section>
      {/each}
    </div>
  </details>
</section>

<style>
  .primary-telemetry {
    position: relative;
    overflow: hidden;
    padding: 20px 24px 18px;
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-lg);
    background: var(--glass-bg);
    box-shadow: var(--glass-shadow);
  }

  .primary-telemetry::after {
    content: '';
    position: absolute;
    right: -130px;
    bottom: -210px;
    width: 420px;
    height: 420px;
    border: 1px solid rgba(115, 210, 182, .08);
    border-radius: 50%;
    box-shadow: 0 0 0 44px rgba(115, 210, 182, .018), 0 0 0 98px rgba(115, 210, 182, .012);
    pointer-events: none;
  }

  .telemetry-header,
  .live-state,
  .reading-value,
  .vertical-trend {
    display: flex;
    align-items: center;
  }

  .telemetry-header { justify-content: space-between; gap: 16px; }
  .section-kicker { color: var(--text-secondary); font-size: 10px; letter-spacing: .08em; }
  h2 { margin-top: 4px; font-size: 13px; font-weight: 590; letter-spacing: .04em; }

  .live-state { gap: 7px; color: var(--text-tertiary); font: 10px/1 var(--font-mono); }
  .live-state i { width: 6px; height: 6px; border-radius: 50%; background: currentColor; }
  .live-state.active { color: var(--accent-cyan); }

  .hero-reading { position: relative; z-index: 1; padding: clamp(22px, 4vh, 42px) 0 28px; }
  .reading-label { color: var(--text-secondary); font-size: 10px; letter-spacing: .08em; }
  .reading-value { gap: 14px; margin-top: 4px; }
  .reading-value strong {
    color: var(--text-primary);
    font: 500 clamp(68px, 8vw, 116px)/.9 var(--font-sans);
    letter-spacing: -.075em;
  }
  .reading-value span { color: var(--accent-cyan); font: 500 18px/1 var(--font-mono); }

  .vertical-trend { gap: 8px; margin-top: 14px; color: var(--accent-cyan); font: 11px/1.2 var(--font-mono); }
  .vertical-trend i {
    width: 8px;
    height: 8px;
    border-top: 2px solid currentColor;
    border-right: 2px solid currentColor;
    transform: rotate(-45deg);
  }
  .vertical-trend.descending { color: var(--accent-orange); }
  .vertical-trend.descending i { transform: rotate(135deg); }

  .metric-strip {
    position: relative;
    z-index: 1;
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 1px;
    overflow: hidden;
    border: 1px solid var(--border-muted);
    border-radius: var(--radius-md);
    background: var(--border-muted);
  }
  .metric-strip > div { min-width: 0; padding: 13px 14px; background: rgba(7, 16, 22, .78); }
  .metric-strip span { display: block; color: var(--text-secondary); font-size: 9px; }
  .metric-strip strong { display: inline-block; margin-top: 6px; font: 500 17px/1.2 var(--font-mono); }
  .metric-strip small { margin-left: 5px; color: var(--text-tertiary); font-size: 9px; }

  .sensor-details { position: relative; z-index: 1; margin-top: 14px; }
  .sensor-details summary { color: var(--text-secondary); cursor: pointer; font-size: 10px; }
  .telemetry-grid { display: grid; grid-template-columns: 1.2fr 1fr .7fr; gap: 16px; margin-top: 14px; }
  .category-section h3 { margin-bottom: 8px; color: var(--text-secondary); font-size: 9px; font-weight: 600; letter-spacing: .08em; }
  .fields-grid { display: grid; gap: 1px; background: var(--border-muted); }
  .field-row {
    display: grid;
    grid-template-columns: 28px minmax(75px, 1fr) auto auto;
    align-items: baseline;
    gap: 7px;
    padding: 7px 8px;
    background: #0b171e;
  }
  .field-icon { color: var(--text-tertiary); font-size: 8px; }
  .field-label { color: var(--text-secondary); font-size: 9px; }
  .field-row strong { font-size: 10px; }
  .field-row small { color: var(--text-tertiary); font-size: 8px; }
  .field-row.warn strong { color: var(--accent-orange); }
  .field-row.crit strong { color: var(--accent-red); }

  @media (max-width: 900px) {
    .metric-strip { grid-template-columns: 1fr 1fr; }
    .telemetry-grid { grid-template-columns: 1fr; }
  }

  @media (max-height: 900px) and (min-width: 1241px) {
    .primary-telemetry { padding: 16px 20px; }
    .hero-reading { padding: 16px 0 18px; }
    .reading-value strong { font-size: clamp(64px, 9vh, 78px); }
    .vertical-trend { margin-top: 10px; }
    .metric-strip > div { padding: 10px 12px; }
    .sensor-details { margin-top: 10px; }
  }
</style>
