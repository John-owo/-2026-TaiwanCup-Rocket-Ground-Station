<script lang="ts">
  import { store } from '@/lib/stores.svelte';
  import type { TelemetryPayload } from '@/lib/types';

  let telemetry: TelemetryPayload = $derived(store.telemetry);

  interface TelemetryField {
    key: keyof TelemetryPayload;
    label: string;
    unit: string;
    category: 'imu' | 'gps' | 'env';
    warnThreshold?: number;
    critThreshold?: number;
    icon: string;
  }

  const fields: TelemetryField[] = [
    { key: 'xAcceleration', label: 'Accel X', unit: 'm/s²', category: 'imu', warnThreshold: 20, critThreshold: 50, icon: '⟶' },
    { key: 'yAcceleration', label: 'Accel Y', unit: 'm/s²', category: 'imu', warnThreshold: 20, critThreshold: 50, icon: '⟶' },
    { key: 'zAcceleration', label: 'Accel Z', unit: 'm/s²', category: 'imu', warnThreshold: 20, critThreshold: 50, icon: '⟶' },
    { key: 'xAngularVelocity', label: 'Gyro X', unit: '°/s', category: 'imu', warnThreshold: 200, critThreshold: 500, icon: '↻' },
    { key: 'yAngularVelocity', label: 'Gyro Y', unit: '°/s', category: 'imu', warnThreshold: 200, critThreshold: 500, icon: '↻' },
    { key: 'zAngularVelocity', label: 'Gyro Z', unit: '°/s', category: 'imu', warnThreshold: 200, critThreshold: 500, icon: '↻' },
    { key: 'longitude', label: 'Longitude', unit: '°', category: 'gps', icon: '◎' },
    { key: 'latitude', label: 'Latitude', unit: '°', category: 'gps', icon: '◎' },
    { key: 'altitude', label: 'Altitude', unit: 'm', category: 'gps', warnThreshold: 1000, critThreshold: 3000, icon: '△' },
    { key: 'groundSpeed', label: 'Ground Speed', unit: 'm/s', category: 'gps', warnThreshold: 100, critThreshold: 300, icon: '▷' },
    { key: 'verticalVelocity', label: 'V. Velocity', unit: 'm/s', category: 'gps', warnThreshold: 50, critThreshold: 200, icon: '↕' },
    { key: 'airPressure', label: 'Pressure', unit: 'hPa', category: 'env', icon: '◉' },
    { key: 'temperature', label: 'Temperature', unit: '°C', category: 'env', warnThreshold: 50, critThreshold: 80, icon: '♨' },
  ];

  function getLevel(field: TelemetryField, value: number): 'normal' | 'warn' | 'crit' {
    const abs = Math.abs(value);
    if (field.critThreshold && abs >= field.critThreshold) return 'crit';
    if (field.warnThreshold && abs >= field.warnThreshold) return 'warn';
    return 'normal';
  }

  function formatValue(value: number, key: string): string {
    if (key === 'longitude' || key === 'latitude') return value.toFixed(6);
    if (key === 'airPressure') return value.toFixed(1);
    if (key === 'temperature') return value.toFixed(1);
    return value.toFixed(2);
  }

  const categories = [
    { id: 'imu', label: 'IMU 感測器', color: 'var(--accent-cyan)' },
    { id: 'gps', label: 'GPS / 導航', color: 'var(--accent-green)' },
    { id: 'env', label: '環境', color: 'var(--accent-orange)' },
  ] as const;
</script>

<div class="telemetry-grid">
  {#each categories as cat, ci}
    <div class="category-section" style="animation-delay: {ci * 100}ms">
      <div class="category-header">
        <div class="cat-line" style="background: {cat.color}"></div>
        <span class="cat-label" style="color: {cat.color}">{cat.label}</span>
      </div>
      <div class="fields-grid">
        {#each fields.filter(f => f.category === cat.id) as field, i}
          {@const value = telemetry[field.key]}
          {@const level = getLevel(field, value)}
          <div
            class="field-card"
            class:warn={level === 'warn'}
            class:crit={level === 'crit'}
            style="animation-delay: {(ci * 100) + (i * 50)}ms"
          >
            <div class="field-header">
              <span class="field-icon">{field.icon}</span>
              <span class="field-label">{field.label}</span>
            </div>
            <div class="field-value">
              <span class="value mono">{formatValue(value, field.key)}</span>
              <span class="unit">{field.unit}</span>
            </div>
            {#if level !== 'normal'}
              <div class="alert-bar" class:warn={level === 'warn'} class:crit={level === 'crit'}></div>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {/each}
</div>

<style>
  .telemetry-grid {
    display: flex;
    flex-direction: column;
    gap: var(--sp-5);
  }

  .category-section {
    animation: slide-up 0.4s ease-out forwards;
    opacity: 0;
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
    text-transform: uppercase;
    letter-spacing: 0.1em;
  }

  .fields-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: var(--sp-3);
  }

  .field-card {
    background: var(--glass-bg);
    backdrop-filter: var(--glass-blur);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-md);
    padding: var(--sp-3) var(--sp-4);
    position: relative;
    overflow: hidden;
    transition: all var(--transition-base);
    animation: slide-up 0.4s ease-out forwards;
    opacity: 0;
  }

  .field-card:hover {
    border-color: rgba(255, 255, 255, 0.12);
    transform: translateY(-2px);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
  }

  .field-card.warn {
    border-color: rgba(255, 140, 0, 0.3);
    box-shadow: 0 0 12px var(--accent-orange-dim);
  }

  .field-card.crit {
    border-color: rgba(255, 59, 59, 0.4);
    box-shadow: 0 0 16px var(--accent-red-dim);
    animation: pulse-glow 2s ease-in-out infinite;
  }

  .field-header {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    margin-bottom: var(--sp-2);
  }

  .field-icon {
    font-size: var(--fs-sm);
    opacity: 0.6;
  }

  .field-label {
    font-size: var(--fs-xs);
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-weight: 500;
  }

  .field-value {
    display: flex;
    align-items: baseline;
    gap: var(--sp-2);
  }

  .value {
    font-size: var(--fs-lg);
    font-weight: 600;
    color: var(--text-primary);
    font-family: var(--font-mono);
    letter-spacing: -0.02em;
  }

  .field-card.warn .value { color: var(--accent-orange); }
  .field-card.crit .value { color: var(--accent-red); }

  .unit {
    font-size: var(--fs-xs);
    color: var(--text-tertiary);
    font-weight: 400;
  }

  .alert-bar {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 2px;
  }

  .alert-bar.warn {
    background: linear-gradient(90deg, transparent, var(--accent-orange), transparent);
  }

  .alert-bar.crit {
    background: linear-gradient(90deg, transparent, var(--accent-red), transparent);
    animation: glow 1.5s ease-in-out infinite;
  }

  .mono {
    font-family: var(--font-mono);
  }

  @keyframes pulse-glow {
    0%, 100% { box-shadow: 0 0 8px var(--accent-red-dim); }
    50% { box-shadow: 0 0 20px var(--accent-red-glow); }
  }
</style>
