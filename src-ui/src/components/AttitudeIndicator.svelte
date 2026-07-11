<script lang="ts">
  import { untrack } from 'svelte';
  import { store } from '@/lib/stores.svelte';
  import {
    createAttitudeEstimator,
    mapSensorVector,
  } from '@/lib/attitude.js';

  const estimator = createAttitudeEstimator();
  let gyroRates = $state({ x: 0, y: 0, z: 0 });

  let pitch = $state(0);
  let roll = $state(0);
  let yaw = $state(0);
  let appliedAxisRevision = -1;

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
      performance.now(),
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

  const AH_SIZE = 200;
  const AH_CENTER = AH_SIZE / 2;
  const AH_RADIUS = 85;

  let pitchOffset = $derived(Math.max(-AH_RADIUS, Math.min(AH_RADIUS, pitch * 1.5)));
  let rollRad = $derived((-roll - 90) * Math.PI / 180);
  let ptrX = $derived(AH_CENTER + Math.cos(rollRad) * (AH_RADIUS + 2));
  let ptrY = $derived(AH_CENTER + Math.sin(rollRad) * (AH_RADIUS + 2));

  const COMP_SIZE = 140;
  const COMP_CENTER = COMP_SIZE / 2;
  const COMP_RADIUS = 58;

  const compassMarks = Array.from({ length: 36 }, (_, i) => {
    const angle = i * 10;
    const rad = (angle - 90) * Math.PI / 180;
    const isCardinal = angle % 90 === 0;
    const isMajor = angle % 30 === 0;
    const innerR = isCardinal ? COMP_RADIUS - 16 : isMajor ? COMP_RADIUS - 12 : COMP_RADIUS - 8;
    return {
      angle,
      x1: COMP_CENTER + Math.cos(rad) * innerR,
      y1: COMP_CENTER + Math.sin(rad) * innerR,
      x2: COMP_CENTER + Math.cos(rad) * COMP_RADIUS,
      y2: COMP_CENTER + Math.sin(rad) * COMP_RADIUS,
      isCardinal,
      isMajor,
      label: angle === 0 ? 'N' : angle === 90 ? 'E' : angle === 180 ? 'S' : angle === 270 ? 'W' : '',
      labelX: COMP_CENTER + Math.cos(rad) * (COMP_RADIUS - 24),
      labelY: COMP_CENTER + Math.sin(rad) * (COMP_RADIUS - 24),
    };
  });

  let rocketLeanX = $derived(Math.max(-26, Math.min(26, roll * 0.35)));
  let rocketLeanY = $derived(Math.max(-20, Math.min(20, -pitch * 0.35)));
  let rocketTransform = $derived(`translate(${110 + rocketLeanX} ${118 + rocketLeanY}) rotate(${roll})`);
  let noseTransform = $derived(`rotate(${yaw}, 110, 118)`);
</script>

<div class="attitude-container">
  <div class="attitude-card rocket-card">
    <div class="card-header">
      <span class="header-label">火箭姿態</span>
      <button class="zero-btn" onclick={zeroAttitude}>歸零</button>
    </div>
    <div class="rocket-wrapper">
      <svg viewBox="0 0 220 220" class="rocket-svg">
        <defs>
          <radialGradient id="rocket-bg" cx="50%" cy="50%" r="60%">
            <stop offset="0%" stop-color="var(--surface-light)" stop-opacity="0.9"/>
            <stop offset="100%" stop-color="var(--surface)" stop-opacity="0.15"/>
          </radialGradient>
          <linearGradient id="rocket-body" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stop-color="#f4fbff"/>
            <stop offset="100%" stop-color="#7fbce8"/>
          </linearGradient>
        </defs>

        <circle cx="110" cy="118" r="86" fill="url(#rocket-bg)" stroke="var(--glass-border)" stroke-width="1"/>
        <circle cx="110" cy="118" r="58" fill="none" stroke="var(--surface-border)" stroke-width="1"/>
        <line x1="24" y1="118" x2="196" y2="118" stroke="var(--surface-border)" stroke-width="1"/>
        <line x1="110" y1="32" x2="110" y2="204" stroke="var(--surface-border)" stroke-width="1"/>

        <g transform={noseTransform} opacity="0.55">
          <path d="M110 34 L104 52 L116 52 Z" fill="var(--accent-cyan)"/>
        </g>

        <g transform={rocketTransform} class="rocket-model">
          <path d="M0 -72 C-13 -55 -15 -32 -13 -8 L13 -8 C15 -32 13 -55 0 -72 Z"
                fill="url(#rocket-body)" stroke="var(--accent-cyan)" stroke-width="1.4"/>
          <rect x="-12" y="-8" width="24" height="66" rx="8"
                fill="rgba(127,188,232,0.86)" stroke="var(--accent-cyan)" stroke-width="1.2"/>
          <path d="M-12 34 L-34 63 L-10 54 Z" fill="var(--accent-cyan)" opacity="0.72"/>
          <path d="M12 34 L34 63 L10 54 Z" fill="var(--accent-cyan)" opacity="0.72"/>
          <path d="M-7 58 L0 76 L7 58 Z" fill="var(--accent-orange)" opacity="0.85"/>
          <line x1="-8" y1="-38" x2="8" y2="-38" stroke="rgba(10,14,26,0.55)" stroke-width="2"/>
          <line x1="-10" y1="8" x2="10" y2="8" stroke="rgba(10,14,26,0.45)" stroke-width="2"/>
        </g>
      </svg>
    </div>
    <div class="attitude-numbers">
      <span class="mono">Pitch {pitch.toFixed(1)} deg</span>
      <span class="mono">Roll {roll.toFixed(1)} deg</span>
      <span class="mono">Yaw {yaw.toFixed(1)} deg</span>
    </div>
  </div>

  <div class="attitude-card">
    <div class="card-header">
      <span class="header-label">姿態</span>
      <span class="header-value mono">P {pitch.toFixed(1)} deg / R {roll.toFixed(1)} deg</span>
    </div>
    <div class="horizon-wrapper">
      <svg viewBox="0 0 {AH_SIZE} {AH_SIZE}" class="horizon-svg">
        <defs>
          <clipPath id="ah-clip">
            <circle cx={AH_CENTER} cy={AH_CENTER} r={AH_RADIUS}/>
          </clipPath>
          <linearGradient id="sky-grad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stop-color="#0066cc"/>
            <stop offset="100%" stop-color="#0044aa"/>
          </linearGradient>
          <linearGradient id="ground-grad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stop-color="#885522"/>
            <stop offset="100%" stop-color="#664411"/>
          </linearGradient>
        </defs>

        <circle cx={AH_CENTER} cy={AH_CENTER} r={AH_RADIUS + 4}
                fill="none" stroke="var(--surface-lighter)" stroke-width="2"/>
        <circle cx={AH_CENTER} cy={AH_CENTER} r={AH_RADIUS}
                fill="none" stroke="var(--glass-border)" stroke-width="1"/>

        <g clip-path="url(#ah-clip)" transform="rotate({-roll}, {AH_CENTER}, {AH_CENTER})">
          <rect x="0" y={-AH_SIZE} width={AH_SIZE} height={AH_SIZE + AH_CENTER + pitchOffset}
                fill="url(#sky-grad)"/>
          <rect x="0" y={AH_CENTER + pitchOffset} width={AH_SIZE} height={AH_SIZE * 2}
                fill="url(#ground-grad)"/>
          <line x1="0" y1={AH_CENTER + pitchOffset} x2={AH_SIZE} y2={AH_CENTER + pitchOffset}
                stroke="#fff" stroke-width="1.5" opacity="0.8"/>

          {#each [-40, -20, -10, 10, 20, 40] as deg}
            {@const markY = AH_CENTER + pitchOffset - deg * 1.5}
            <line x1={AH_CENTER - 20} y1={markY} x2={AH_CENTER + 20} y2={markY}
                  stroke="#fff" stroke-width="1" opacity="0.5"/>
            <text x={AH_CENTER + 24} y={markY + 3} fill="#fff" font-size="8" opacity="0.6">{deg} deg</text>
          {/each}
        </g>

        <line x1={AH_CENTER - 35} y1={AH_CENTER} x2={AH_CENTER - 12} y2={AH_CENTER}
              stroke="var(--accent-cyan)" stroke-width="2.5" stroke-linecap="round"/>
        <line x1={AH_CENTER + 12} y1={AH_CENTER} x2={AH_CENTER + 35} y2={AH_CENTER}
              stroke="var(--accent-cyan)" stroke-width="2.5" stroke-linecap="round"/>
        <circle cx={AH_CENTER} cy={AH_CENTER} r="3" fill="var(--accent-cyan)"/>

        {#each [-60, -45, -30, -20, -10, 0, 10, 20, 30, 45, 60] as deg}
          {@const rad = (deg - 90) * Math.PI / 180}
          {@const r1 = AH_RADIUS - 2}
          {@const r2 = deg % 30 === 0 ? AH_RADIUS + 8 : AH_RADIUS + 5}
          <line
            x1={AH_CENTER + Math.cos(rad) * r1}
            y1={AH_CENTER + Math.sin(rad) * r1}
            x2={AH_CENTER + Math.cos(rad) * r2}
            y2={AH_CENTER + Math.sin(rad) * r2}
            stroke={deg === 0 ? 'var(--accent-cyan)' : 'var(--text-tertiary)'}
            stroke-width={deg % 30 === 0 ? 2 : 1}
          />
        {/each}

        <circle cx={ptrX} cy={ptrY} r="4" fill="var(--accent-cyan)"/>
      </svg>
    </div>
  </div>

  <div class="attitude-card">
    <div class="card-header">
      <span class="header-label">相對航向</span>
      <span class="header-value mono">{yaw.toFixed(1)} deg</span>
    </div>
    <div class="compass-wrapper">
      <svg viewBox="0 0 {COMP_SIZE} {COMP_SIZE}" class="compass-svg">
        <defs>
          <radialGradient id="comp-bg">
            <stop offset="0%" stop-color="var(--surface-light)" stop-opacity="0.8"/>
            <stop offset="100%" stop-color="var(--surface)" stop-opacity="0.6"/>
          </radialGradient>
        </defs>

        <circle cx={COMP_CENTER} cy={COMP_CENTER} r={COMP_RADIUS + 2}
                fill="none" stroke="var(--surface-lighter)" stroke-width="1.5"/>
        <circle cx={COMP_CENTER} cy={COMP_CENTER} r={COMP_RADIUS}
                fill="url(#comp-bg)"/>

        <g transform="rotate({-yaw}, {COMP_CENTER}, {COMP_CENTER})">
          {#each compassMarks as mark}
            <line x1={mark.x1} y1={mark.y1} x2={mark.x2} y2={mark.y2}
                  stroke={mark.isCardinal ? 'var(--accent-cyan)' : 'var(--text-tertiary)'}
                  stroke-width={mark.isCardinal ? 2 : mark.isMajor ? 1.5 : 0.8}/>
            {#if mark.label}
              <text x={mark.labelX} y={mark.labelY + 4} text-anchor="middle"
                    fill={mark.label === 'N' ? 'var(--accent-red)' : 'var(--accent-cyan)'}
                    font-size="11" font-weight="700" font-family="var(--font-mono)">
                {mark.label}
              </text>
            {/if}
          {/each}
        </g>

        <polygon points="{COMP_CENTER},{COMP_CENTER - COMP_RADIUS - 8} {COMP_CENTER - 5},{COMP_CENTER - COMP_RADIUS + 2} {COMP_CENTER + 5},{COMP_CENTER - COMP_RADIUS + 2}"
                 fill="var(--accent-cyan)"/>
        <circle cx={COMP_CENTER} cy={COMP_CENTER} r="3" fill="var(--accent-cyan)" opacity="0.6"/>
      </svg>
    </div>
  </div>

  <div class="attitude-card angular-readout">
    <div class="card-header">
      <span class="header-label">角速度</span>
    </div>
    <div class="angular-values">
      <div class="angular-item">
        <span class="angular-label">俯仰</span>
        <span class="angular-val mono">{gyroRates.y.toFixed(1)}</span>
        <span class="angular-unit">deg/s</span>
      </div>
      <div class="angular-item">
        <span class="angular-label">滾轉</span>
        <span class="angular-val mono">{gyroRates.x.toFixed(1)}</span>
        <span class="angular-unit">deg/s</span>
      </div>
      <div class="angular-item">
        <span class="angular-label">偏航</span>
        <span class="angular-val mono">{gyroRates.z.toFixed(1)}</span>
        <span class="angular-unit">deg/s</span>
      </div>
    </div>
  </div>
</div>

<style>
  .attitude-container {
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
  }

  .attitude-card {
    background: var(--glass-bg);
    backdrop-filter: var(--glass-blur);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-lg);
    padding: var(--sp-4);
    box-shadow: var(--glass-shadow);
    animation: slide-up 0.5s ease-out forwards;
    opacity: 0;
    animation-delay: 300ms;
  }

  .rocket-card {
    animation-delay: 220ms;
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--sp-3);
  }

  .header-label {
    font-size: var(--fs-xs);
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .header-value {
    font-size: var(--fs-sm);
    color: var(--accent-cyan);
    font-weight: 600;
  }

  .zero-btn {
    padding: var(--sp-1) var(--sp-3);
    border: 1px solid var(--accent-cyan-dim);
    border-radius: var(--radius-full);
    background: var(--surface);
    color: var(--accent-cyan);
    font-size: var(--fs-xs);
  }

  .horizon-wrapper,
  .compass-wrapper,
  .rocket-wrapper {
    display: flex;
    justify-content: center;
  }

  .rocket-svg {
    width: 220px;
    height: 220px;
  }

  .rocket-model {
    filter: drop-shadow(0 0 12px var(--accent-cyan-glow));
    transition: transform 120ms linear;
  }

  .attitude-numbers {
    display: grid;
    grid-template-columns: 1fr;
    gap: var(--sp-1);
    margin-top: var(--sp-3);
    color: var(--text-secondary);
    font-size: var(--fs-xs);
  }

  .horizon-svg {
    width: 200px;
    height: 200px;
  }

  .compass-svg {
    width: 140px;
    height: 140px;
  }

  .angular-readout {
    animation-delay: 400ms;
  }

  .angular-values {
    display: flex;
    gap: var(--sp-4);
    justify-content: space-around;
  }

  .angular-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }

  .angular-label {
    font-size: 9px;
    font-weight: 600;
    color: var(--text-tertiary);
    letter-spacing: 0.1em;
  }

  .angular-val {
    font-size: var(--fs-lg);
    font-weight: 700;
    color: var(--text-primary);
    font-family: var(--font-mono);
  }

  .angular-unit {
    font-size: var(--fs-xs);
    color: var(--text-tertiary);
  }

  .mono {
    font-family: var(--font-mono);
  }
</style>
