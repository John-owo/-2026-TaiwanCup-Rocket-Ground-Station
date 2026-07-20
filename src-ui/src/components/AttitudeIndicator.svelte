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

  let rocketLeanX = $derived(Math.max(-18, Math.min(18, roll * .24)));
  let rocketLeanY = $derived(Math.max(-14, Math.min(14, -pitch * .24)));
  let rocketTransform = $derived(`translate(${82 + rocketLeanX} ${71 + rocketLeanY}) rotate(${roll})`);
  let headingTransform = $derived(`rotate(${yaw}, 82, 71)`);
</script>

<article class="attitude-strip">
  <header>
    <div>
      <span class="eyebrow">ATTITUDE ESTIMATE</span>
      <h2>估算姿態</h2>
      <p class="estimate-note">MPU6050 無磁力計；YAW 為相對角度，非絕對航向</p>
    </div>
    <button class="zero-btn" onclick={zeroAttitude}>姿態歸零</button>
  </header>

  <div class="attitude-visual" aria-label="火箭即時姿態示意">
    <svg viewBox="0 0 164 142" role="img">
      <circle cx="82" cy="71" r="55" fill="none" stroke="var(--surface-border)" />
      <circle cx="82" cy="71" r="34" fill="none" stroke="var(--surface-border)" stroke-dasharray="2 5" />
      <line x1="16" y1="71" x2="148" y2="71" stroke="var(--surface-border)" />
      <line x1="82" y1="5" x2="82" y2="137" stroke="var(--surface-border)" />
      <g transform={headingTransform} opacity=".7">
        <path d="M82 9 L77 21 H87 Z" fill="var(--accent-cyan)" />
      </g>
      <g transform={rocketTransform} class="rocket-model">
        <path d="M0 -36 C-8 -27 -9 -14 -8 2 H8 C9 -14 8 -27 0 -36 Z" fill="#d9e7eb" />
        <rect x="-8" y="1" width="16" height="34" rx="5" fill="#7599a5" />
        <path d="M-8 22 L-18 36 L-6 31 Z M8 22 L18 36 L6 31 Z" fill="var(--accent-cyan)" />
        <path d="M-4 35 L0 44 L4 35 Z" fill="var(--accent-orange)" />
      </g>
    </svg>
  </div>

  <dl class="angles">
    <div><dt>滾轉 ROLL</dt><dd class="mono">{roll.toFixed(1)}°</dd></div>
    <div><dt>俯仰 PITCH</dt><dd class="mono">{pitch.toFixed(1)}°</dd></div>
    <div><dt>偏航 YAW</dt><dd class="mono">{yaw.toFixed(1)}°</dd></div>
  </dl>

  <dl class="rates">
    <div><dt>ROLL RATE</dt><dd class="mono">{gyroRates.x.toFixed(1)}°/s</dd></div>
    <div><dt>PITCH RATE</dt><dd class="mono">{gyroRates.y.toFixed(1)}°/s</dd></div>
    <div><dt>YAW RATE</dt><dd class="mono">{gyroRates.z.toFixed(1)}°/s</dd></div>
  </dl>
</article>

<style>
  .attitude-strip {
    display: grid;
    grid-template-columns: 150px 146px minmax(300px, 1fr) minmax(230px, .7fr);
    align-items: center;
    min-height: 148px;
    overflow: hidden;
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-lg);
    background: var(--glass-bg);
    box-shadow: var(--glass-shadow);
  }

  header { align-self: stretch; padding: var(--sp-5); border-right: 1px solid var(--border-muted); }
  .eyebrow { color: var(--accent-cyan); font-family: var(--font-mono); font-size: 9px; letter-spacing: .13em; }
  h2 { margin-top: 5px; font-size: var(--fs-lg); font-weight: 560; }
  .estimate-note { margin-top: 4px; color: var(--text-tertiary); font-size: 9px; line-height: 1.35; }
  .zero-btn {
    margin-top: var(--sp-5);
    padding: 7px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-full);
    background: transparent;
    color: var(--text-secondary);
    font-size: var(--fs-xs);
  }
  .zero-btn:hover { border-color: var(--accent-cyan); color: var(--accent-cyan); }

  .attitude-visual { display: grid; height: 142px; place-items: center; border-right: 1px solid var(--border-muted); }
  .attitude-visual svg { width: 142px; height: 132px; }
  .rocket-model { transition: transform 120ms linear; }

  dl { display: grid; margin: 0; }
  dl > div { min-width: 0; }
  dt { color: var(--text-tertiary); font-size: 9px; letter-spacing: .06em; }
  dd { margin-top: 2px; color: var(--text-primary); }
  .angles { grid-template-columns: repeat(3, 1fr); gap: var(--sp-4); padding: var(--sp-5); }
  .angles dd { font-size: clamp(18px, 1.6vw, 25px); }
  .rates { gap: var(--sp-2); align-self: stretch; padding: var(--sp-4) var(--sp-5); border-left: 1px solid var(--border-muted); background: rgba(5, 13, 18, .3); }
  .rates > div { display: flex; align-items: center; justify-content: space-between; gap: var(--sp-3); }
  .rates dd { font-size: var(--fs-sm); }

  @media (max-width: 1100px) {
    .attitude-strip { grid-template-columns: 140px 138px 1fr; }
    .rates { grid-column: 1 / -1; grid-template-columns: repeat(3, 1fr); border-top: 1px solid var(--border-muted); border-left: 0; }
  }

  @media (max-width: 680px) {
    .attitude-strip { grid-template-columns: 1fr 1fr; }
    header { border-bottom: 1px solid var(--border-muted); }
    .attitude-visual { border-right: 0; border-bottom: 1px solid var(--border-muted); }
    .angles { grid-column: 1 / -1; }
  }
</style>
