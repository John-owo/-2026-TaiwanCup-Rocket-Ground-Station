<script lang="ts">
  import { store } from '@/lib/stores.svelte';
  import type { TelemetryPayload } from '@/lib/types';

  let telemetry: TelemetryPayload = $derived(store.telemetry);

  // 簡易積分：使用角速度推算姿態角
  // 在實際應用中需要互補濾波或卡爾曼濾波
  let pitch = $state(0);  // 俯仰角 (degrees)
  let roll = $state(0);   // 翻滾角 (degrees)
  let yaw = $state(0);    // 偏航角 (degrees)

  // 使用 $effect 追蹤角速度變化，進行簡易積分
  let lastTime = $state(Date.now());

  $effect(() => {
    const now = Date.now();
    const dt = Math.min((now - lastTime) / 1000, 0.1); // cap at 100ms
    lastTime = now;

    // 簡易積分（實際應用需要更好的 AHRS 算法）
    pitch = Math.max(-90, Math.min(90, pitch + telemetry.yAngularVelocity * dt));
    roll = ((roll + telemetry.xAngularVelocity * dt + 180) % 360) - 180;
    yaw = ((yaw + telemetry.zAngularVelocity * dt) % 360 + 360) % 360;
  });

  // Artificial Horizon calculations
  const AH_SIZE = 200;
  const AH_CENTER = AH_SIZE / 2;
  const AH_RADIUS = 85;

  let pitchOffset = $derived(Math.max(-AH_RADIUS, Math.min(AH_RADIUS, pitch * 1.5)));

  let rollRad = $derived((-roll - 90) * Math.PI / 180);
  let ptrX = $derived(AH_CENTER + Math.cos(rollRad) * (AH_RADIUS + 2));
  let ptrY = $derived(AH_CENTER + Math.sin(rollRad) * (AH_RADIUS + 2));

  // Compass calculations
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
</script>

<div class="attitude-container">
  <!-- Artificial Horizon -->
  <div class="attitude-card">
    <div class="card-header">
      <span class="header-label">人工地平儀</span>
      <span class="header-value mono">{pitch.toFixed(1)}° / {roll.toFixed(1)}°</span>
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

        <!-- Outer ring -->
        <circle cx={AH_CENTER} cy={AH_CENTER} r={AH_RADIUS + 4}
                fill="none" stroke="var(--surface-lighter)" stroke-width="2"/>
        <circle cx={AH_CENTER} cy={AH_CENTER} r={AH_RADIUS}
                fill="none" stroke="var(--glass-border)" stroke-width="1"/>

        <!-- Sky + Ground with roll rotation -->
        <g clip-path="url(#ah-clip)" transform="rotate({-roll}, {AH_CENTER}, {AH_CENTER})">
          <!-- Sky -->
          <rect x="0" y={-AH_SIZE} width={AH_SIZE} height={AH_SIZE + AH_CENTER + pitchOffset}
                fill="url(#sky-grad)"/>
          <!-- Ground -->
          <rect x="0" y={AH_CENTER + pitchOffset} width={AH_SIZE} height={AH_SIZE * 2}
                fill="url(#ground-grad)"/>
          <!-- Horizon line -->
          <line x1="0" y1={AH_CENTER + pitchOffset} x2={AH_SIZE} y2={AH_CENTER + pitchOffset}
                stroke="#fff" stroke-width="1.5" opacity="0.8"/>

          <!-- Pitch marks -->
          {#each [-40, -20, -10, 10, 20, 40] as deg}
            {@const markY = AH_CENTER + pitchOffset - deg * 1.5}
            <line x1={AH_CENTER - 20} y1={markY} x2={AH_CENTER + 20} y2={markY}
                  stroke="#fff" stroke-width="1" opacity="0.5"/>
            <text x={AH_CENTER + 24} y={markY + 3} fill="#fff" font-size="8" opacity="0.6">{deg}°</text>
          {/each}
        </g>

        <!-- Fixed aircraft symbol -->
        <line x1={AH_CENTER - 35} y1={AH_CENTER} x2={AH_CENTER - 12} y2={AH_CENTER}
              stroke="var(--accent-cyan)" stroke-width="2.5" stroke-linecap="round"/>
        <line x1={AH_CENTER + 12} y1={AH_CENTER} x2={AH_CENTER + 35} y2={AH_CENTER}
              stroke="var(--accent-cyan)" stroke-width="2.5" stroke-linecap="round"/>
        <circle cx={AH_CENTER} cy={AH_CENTER} r="3" fill="var(--accent-cyan)"/>

        <!-- Roll indicator arc (top) -->
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

        <!-- Roll pointer (triangle at current roll angle) -->
        <circle cx={ptrX} cy={ptrY} r="4" fill="var(--accent-cyan)"/>
      </svg>
    </div>
  </div>

  <!-- Compass / Heading Indicator -->
  <div class="attitude-card">
    <div class="card-header">
      <span class="header-label">航向指示器</span>
      <span class="header-value mono">{yaw.toFixed(1)}°</span>
    </div>
    <div class="compass-wrapper">
      <svg viewBox="0 0 {COMP_SIZE} {COMP_SIZE}" class="compass-svg">
        <defs>
          <radialGradient id="comp-bg">
            <stop offset="0%" stop-color="var(--surface-light)" stop-opacity="0.8"/>
            <stop offset="100%" stop-color="var(--surface)" stop-opacity="0.6"/>
          </radialGradient>
        </defs>

        <!-- Background -->
        <circle cx={COMP_CENTER} cy={COMP_CENTER} r={COMP_RADIUS + 2}
                fill="none" stroke="var(--surface-lighter)" stroke-width="1.5"/>
        <circle cx={COMP_CENTER} cy={COMP_CENTER} r={COMP_RADIUS}
                fill="url(#comp-bg)"/>

        <!-- Rotating compass rose -->
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

        <!-- Fixed heading indicator (triangle at top) -->
        <polygon points="{COMP_CENTER},{COMP_CENTER - COMP_RADIUS - 8} {COMP_CENTER - 5},{COMP_CENTER - COMP_RADIUS + 2} {COMP_CENTER + 5},{COMP_CENTER - COMP_RADIUS + 2}"
                 fill="var(--accent-cyan)"/>

        <!-- Center dot -->
        <circle cx={COMP_CENTER} cy={COMP_CENTER} r="3" fill="var(--accent-cyan)" opacity="0.6"/>
      </svg>
    </div>
  </div>

  <!-- Angular velocity readout -->
  <div class="attitude-card angular-readout">
    <div class="card-header">
      <span class="header-label">角速度</span>
    </div>
    <div class="angular-values">
      <div class="angular-item">
        <span class="angular-label">PITCH</span>
        <span class="angular-val mono">{telemetry.yAngularVelocity.toFixed(1)}</span>
        <span class="angular-unit">°/s</span>
      </div>
      <div class="angular-item">
        <span class="angular-label">ROLL</span>
        <span class="angular-val mono">{telemetry.xAngularVelocity.toFixed(1)}</span>
        <span class="angular-unit">°/s</span>
      </div>
      <div class="angular-item">
        <span class="angular-label">YAW</span>
        <span class="angular-val mono">{telemetry.zAngularVelocity.toFixed(1)}</span>
        <span class="angular-unit">°/s</span>
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

  .horizon-wrapper,
  .compass-wrapper {
    display: flex;
    justify-content: center;
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
