<script lang="ts">
  import { store } from '@/lib/stores.svelte';
  import type { TelemetryPayload } from '@/lib/types';

  let telemetry: TelemetryPayload = $derived(store.telemetry);

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
    { id: 'imu', label: 'IMU 感測器', color: 'var(--accent-cyan)' },
    { id: 'gps', label: '飛行／定位', color: 'var(--accent-green)' },
    { id: 'env', label: '環境感測', color: 'var(--accent-orange)' },
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

<div class="telemetry-grid">
  {#each categories as cat, ci}
    <section class="category-section" style="animation-delay: {ci * 100}ms">
      <div class="category-header">
        <div class="cat-line" style="background: {cat.color}"></div>
        <span class="cat-label" style="color: {cat.color}">{cat.label}</span>
      </div>

      <div class="fields-grid">
        {#each fields.filter((field) => field.category === cat.id) as field, i}
          {@const value = telemetry[field.key] as number}
          {@const level = getLevel(field, value)}
          <article
            class="field-card"
            class:warn={level === 'warn'}
            class:crit={level === 'crit'}
            style="animation-delay: {(ci * 100) + (i * 45)}ms"
          >
            <div class="field-header">
              <span class="field-icon">{field.icon}</span>
              <span class="field-label">{field.label}</span>
            </div>
            <div class="field-value">
              <span class="value mono">{formatValue(value, field.precision)}</span>
              <span class="unit">{field.unit}</span>
            </div>
            {#if level !== 'normal'}
              <div class="alert-bar" class:warn={level === 'warn'} class:crit={level === 'crit'}></div>
            {/if}
          </article>
        {/each}
      </div>
    </section>
  {/each}
</div>

<style>
  .telemetry-grid {
    display: flex;
    flex-direction: column;
    gap: var(--sp-5);
  }

  .category-section {
    opacity: 0;
    animation: slide-up 0.4s ease-out forwards;
  }

  .category-header {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    margin-bottom: var(--sp-3);
  }

  .cat-line {
    width: 3px;
    height: 16px;
    border-radius: 2px;
  }

  .cat-label {
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }

  .fields-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: var(--sp-3);
  }

  .field-card {
    position: relative;
    overflow: hidden;
    padding: var(--sp-3) var(--sp-4);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-md);
    background: var(--glass-bg);
    backdrop-filter: var(--glass-blur);
    opacity: 0;
    animation: slide-up 0.4s ease-out forwards;
    transition: border-color var(--transition-base), transform var(--transition-base), box-shadow var(--transition-base);
  }

  .field-card:hover {
    transform: translateY(-2px);
    border-color: rgba(255, 255, 255, 0.12);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
  }

  .field-card.warn {
    border-color: rgba(255, 140, 0, 0.3);
    box-shadow: 0 0 12px var(--accent-orange-dim);
  }

  .field-card.crit {
    border-color: rgba(255, 59, 59, 0.4);
    box-shadow: 0 0 16px var(--accent-red-dim);
  }

  .field-header {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    margin-bottom: var(--sp-2);
  }

  .field-icon {
    min-width: 24px;
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    font-size: 9px;
    font-weight: 700;
  }

  .field-label {
    color: var(--text-secondary);
    font-size: var(--fs-xs);
    font-weight: 500;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .field-value {
    display: flex;
    align-items: baseline;
    gap: var(--sp-2);
  }

  .value {
    color: var(--text-primary);
    font-size: var(--fs-lg);
    font-weight: 600;
    letter-spacing: -0.02em;
  }

  .field-card.warn .value {
    color: var(--accent-orange);
  }

  .field-card.crit .value {
    color: var(--accent-red);
  }

  .unit {
    color: var(--text-tertiary);
    font-size: var(--fs-xs);
  }

  .alert-bar {
    position: absolute;
    right: 0;
    bottom: 0;
    left: 0;
    height: 2px;
  }

  .alert-bar.warn {
    background: linear-gradient(90deg, transparent, var(--accent-orange), transparent);
  }

  .alert-bar.crit {
    background: linear-gradient(90deg, transparent, var(--accent-red), transparent);
  }
</style>
