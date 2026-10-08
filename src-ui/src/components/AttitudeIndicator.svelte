<script lang="ts">
  import { untrack } from 'svelte';
  import { store } from '@/lib/stores.svelte';
  import { createAttitudeEstimator, mapSensorVector } from '@/lib/attitude.js';

  const estimator = createAttitudeEstimator();
  let gyroRates = $state({ x: 0, y: 0, z: 0 });
  let pitch = $state(0);
  let roll = $state(0);
  let yaw = $state(0);
  let appliedAxisRevision = -1;
  let appliedSessionId: number | null = null;

  $effect(() => {
    const axisRevision = store.axisMappingRevision;
    if (axisRevision === appliedAxisRevision) return;
    appliedAxisRevision = axisRevision;
    const reset = estimator.reset();
    roll = reset.roll;
    pitch = reset.pitch;
    yaw = reset.yaw;
    gyroRates = { x: 0, y: 0, z: 0 };
  });

  $effect(() => {
    const revision = store.telemetryRevision;
    if (revision === 0) return;
    const snapshot = untrack(() => ({
      telemetry: store.telemetry,
      mapping: store.settings.axisMapping,
    }));
    const rawGyro = {
      x: snapshot.telemetry.xAngularVelocity,
      y: snapshot.telemetry.yAngularVelocity,
      z: snapshot.telemetry.zAngularVelocity,
    };
    if (appliedSessionId !== snapshot.telemetry.sessionId) {
      appliedSessionId = snapshot.telemetry.sessionId;
      const reset = estimator.reset();
      roll = reset.roll;
      pitch = reset.pitch;
      yaw = reset.yaw;
    }
    gyroRates = mapSensorVector(rawGyro, snapshot.mapping);
    const next = estimator.update(
      {
        gyro: rawGyro,
        accel: {
          x: snapshot.telemetry.xAcceleration,
          y: snapshot.telemetry.yAcceleration,
          z: snapshot.telemetry.zAcceleration,
        },
      },
      snapshot.telemetry.uptimeMs,
      snapshot.mapping,
    );
    roll = next.roll;
    pitch = next.pitch;
    yaw = next.yaw;
  });

  function zeroAttitude() {
    const reset = estimator.reset();
    roll = reset.roll;
    pitch = reset.pitch;
    yaw = reset.yaw;
  }

  let rocketLeanX = $derived(Math.max(-14, Math.min(14, roll * .2)));
  let rocketLeanY = $derived(Math.max(-10, Math.min(10, -pitch * .2)));
  let rocketTransform = $derived(`translate(${56 + rocketLeanX} ${56 + rocketLeanY}) rotate(${roll})`);
  let headingTransform = $derived(`rotate(${yaw}, 56, 56)`);
</script>

<section class="attitude" aria-label="估算姿態">
  <header>
    <h2>估算姿態</h2>
    <button class="zero-btn" onclick={zeroAttitude}>姿態歸零</button>
  </header>

  <div class="body">
    <svg class="dial" viewBox="0 0 112 112" role="img" aria-label="火箭即時姿態示意">
      <circle cx="56" cy="56" r="52" fill="var(--sheet)" stroke="var(--rule-strong)" />
      <circle cx="56" cy="56" r="32" fill="none" stroke="var(--rule)" stroke-dasharray="2 4" />
      <line x1="4" y1="56" x2="108" y2="56" stroke="var(--rule)" />
      <line x1="56" y1="4" x2="56" y2="108" stroke="var(--rule)" />
      <g transform={headingTransform}>
        <path d="M56 6 L52 14 H60 Z" fill="var(--ink-2)" />
      </g>
      <g transform={rocketTransform} class="rocket-model">
        <path d="M0 -36 C-6 -28 -7 -18 -6 -8 H6 C7 -18 6 -28 0 -36 Z" fill="var(--ink)" />
        <rect x="-6" y="-8" width="12" height="32" rx="2" fill="var(--ink-2)" />
        <path d="M-6 12 L-14 28 L-6 24 Z M6 12 L14 28 L6 24 Z" fill="var(--rocket)" />
      </g>
    </svg>

    <dl class="angles">
      <div><dt>滾轉</dt><span class="rate num">{gyroRates.x.toFixed(1)}°/s</span><dd class="num">{roll.toFixed(1)}°</dd></div>
      <div><dt>俯仰</dt><span class="rate num">{gyroRates.y.toFixed(1)}°/s</span><dd class="num">{pitch.toFixed(1)}°</dd></div>
      <div><dt>偏航</dt><span class="rate num">{gyroRates.z.toFixed(1)}°/s</span><dd class="num">{yaw.toFixed(1)}°</dd></div>
    </dl>
  </div>
  <p class="note">MPU6050 無磁力計；YAW 為相對角度，非絕對航向。</p>
</section>

<style>
  .attitude {
    display: grid;
    grid-template-rows: auto 1fr auto;
    gap: 10px;
    min-width: 0;
    padding: 14px clamp(16px, 1.6vw, 22px) 12px;
    border-right: 1px solid var(--rule);
  }

  header { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  h2 { font-size: 15px; font-weight: 700; }
  .zero-btn {
    color: var(--ink-2);
    font-size: 12.5px;
    text-decoration: underline;
    text-decoration-color: var(--rule-strong);
    text-underline-offset: 3px;
  }
  .zero-btn:hover { color: var(--ink); text-decoration-color: currentColor; }

  .body { display: grid; grid-template-columns: 104px minmax(0, 1fr); align-items: center; gap: 16px; }
  .dial { width: 104px; height: 104px; }
  .rocket-model { transition: transform 160ms linear; }

  .angles { display: grid; gap: 7px; }
  .angles div { display: grid; grid-template-columns: minmax(0, 1fr) auto; align-items: baseline; column-gap: 8px; }
  dt { color: var(--ink-3); font-size: 12px; line-height: 1.3; white-space: nowrap; }
  .rate { color: var(--ink-3); font-size: 12px; font-weight: 500; text-align: right; }
  dd { grid-column: 1 / -1; font-size: 20px; font-weight: 600; line-height: 1.1; }

  .note { color: var(--ink-3); font-size: 12px; }

  @media (max-width: 1180px) {
    .attitude { border-right: 0; border-bottom: 1px solid var(--rule); }
  }
</style>
