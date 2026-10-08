<script lang="ts">
  import { store } from '@/lib/stores.svelte';
  import { forceRelease, setTimer } from '@/lib/tauri';
  import { getTelemetryLinkState } from '@/lib/telemetry-link.js';

  let timerSeconds = $state(30);
  let safetyUnlocked = $state(false);
  let busy = $state(false);
  let errorMessage = $state('');
  let nowMs = $state(Date.now());
  let observedSessionId = $state<number | null>(null);

  let telemetry = $derived(store.telemetry);
  let commandStatus = $derived(store.commandStatus);
  let session = $derived(store.testSessionStatus);
  let controlsEnabled = $derived(
    session.phase === 'recording' || session.phase === 'monitoring_unrecorded',
  );
  let airborneLinkState = $derived(
    getTelemetryLinkState(controlsEnabled, store.lastPacketAt, nowMs),
  );
  let forceAvailable = $derived(
    controlsEnabled && airborneLinkState === 'live' && telemetry.deployState !== 1,
  );
  let deployed = $derived(telemetry.deployState === 1);
  let hasTelemetry = $derived(store.telemetryRevision > 0);
  const commandLabels = {
    queued: '排隊中',
    sending: '已送出，等待確認',
    acked: '已確認',
    failed: '失敗',
    ignored_ack: '忽略過期確認',
    cancelled: '已取消',
  } as const;
  let lastPacket = $derived(
    store.lastPacketAt === null
      ? '--'
      : new Date(store.lastPacketAt).toLocaleTimeString('zh-TW', { hour12: false }),
  );

  $effect(() => {
    if (!controlsEnabled) {
      nowMs = Date.now();
      return;
    }
    const interval = setInterval(() => {
      nowMs = Date.now();
    }, 250);
    return () => clearInterval(interval);
  });

  $effect(() => {
    const sessionId = telemetry.sessionId;
    const sessionChanged = observedSessionId !== null && sessionId !== observedSessionId;
    if (airborneLinkState !== 'live' || sessionChanged) {
      safetyUnlocked = false;
    }
    observedSessionId = sessionId || null;
  });

  function errorText(error: unknown): string {
    if (typeof error === 'object' && error !== null) {
      const value = error as { detail?: string; message?: string };
      return value.detail ?? value.message ?? String(error);
    }
    return String(error);
  }

  async function applyTimer() {
    if (!controlsEnabled) return;
    if (telemetry.deployState === 1) {
      errorMessage = '空中端已 DEPLOYED；請冷開機回到 SAFE／UNSET 後再測 timer';
      return;
    }
    if (!Number.isInteger(timerSeconds) || timerSeconds <= 0) {
      errorMessage = '倒數秒數必須是大於 0 的整數';
      return;
    }
    busy = true;
    errorMessage = '';
    try {
      await setTimer(timerSeconds);
    } catch (error) {
      errorMessage = errorText(error);
    } finally {
      busy = false;
    }
  }

  async function releaseNow() {
    if (!forceAvailable || !safetyUnlocked) return;
    busy = true;
    errorMessage = '';
    try {
      await forceRelease();
    } catch (error) {
      errorMessage = errorText(error);
    } finally {
      safetyUnlocked = false;
      busy = false;
    }
  }
</script>

<section class="control" aria-label="飛行控制">
  <header>
    <h2>飛行控制</h2>
    <span class="deploy-state" class:deployed>
      <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true">
        {#if deployed}
          <path d="M7 1.5v6M3.2 4.2a5 5 0 1 0 7.6 0" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" />
        {:else}
          <path d="M7 1 12 3v4c0 3-2.5 5-5 6-2.5-1-5-3-5-6V3Z" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round" />
        {/if}
      </svg>
      {deployed ? 'DEPLOYED' : 'SAFE'}
    </span>
  </header>

  <div class="timer">
    <div class="remaining">
      <span>空中端剩餘倒數</span>
      <strong class="num">{hasTelemetry ? telemetry.remainingS : '--'}<small>s</small></strong>
    </div>
    <div class="set">
      <label for="timer-seconds" class="visually-hidden">設定倒數秒數</label>
      <input id="timer-seconds" class="num" type="number" min="1" step="1" bind:value={timerSeconds} disabled={busy || !controlsEnabled || telemetry.deployState === 1} />
      <button class="apply" onclick={applyTimer} disabled={busy || !controlsEnabled || telemetry.deployState === 1}>覆蓋倒數</button>
    </div>
  </div>

  <div class="release">
    <button
      class="arm"
      class:unlocked={safetyUnlocked}
      onclick={() => { safetyUnlocked = !safetyUnlocked; }}
      disabled={busy || !forceAvailable}
      aria-pressed={safetyUnlocked}
    >
      <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
        <rect x="3" y="7" width="10" height="7" rx="1" fill="none" stroke="currentColor" stroke-width="1.6" />
        <path d={safetyUnlocked ? 'M5 7V5a3 3 0 0 1 5.8-1' : 'M5 7V5a3 3 0 0 1 6 0v2'} fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
      </svg>
      {safetyUnlocked ? '重新上鎖' : '解除安全鎖'}
    </button>
    <button class="fire num" class:ready={safetyUnlocked && forceAvailable} onclick={releaseNow} disabled={busy || !forceAvailable || !safetyUnlocked}>
      FORCE RELEASE
    </button>
  </div>

  <p class="gate">
    {#if deployed}
      空中端已 DEPLOYED，不再接受倒數與強制釋放。
    {:else if forceAvailable}
      即時遙測正常 · 最後封包 <span class="num">{lastPacket}</span>。解鎖後單擊才會送出一次。
    {:else}
      強制釋放已鎖定：需要 4.5 秒內的即時遙測，且場次未變更。
    {/if}
  </p>

  {#if commandStatus}
    <p class="command" class:failed={commandStatus.status === 'failed'}>
      最近指令 <b>{commandStatus.commandType} #{commandStatus.commandId ?? '--'}</b>
      · {commandLabels[commandStatus.status] ?? commandStatus.status} · 第 <span class="num">{commandStatus.attempts}</span> 次傳送
      {#if commandStatus.status === 'failed'}<span class="detail">{commandStatus.detail}</span>{/if}
    </p>
  {/if}

  {#if errorMessage}
    <p class="error-message" role="alert">{errorMessage}</p>
  {/if}
</section>

<style>
  .control {
    display: grid;
    gap: 12px;
    padding: 16px clamp(16px, 1.6vw, 22px) 16px;
    background: var(--sheet);
  }

  header { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  h2 { font-size: 15px; font-weight: 700; }

  .deploy-state {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 5px 11px;
    border: 1.5px solid var(--live);
    border-radius: var(--radius);
    color: var(--live);
    font-family: var(--font-num);
    font-size: 15px;
    font-weight: 700;
    letter-spacing: .06em;
  }
  .deploy-state.deployed { border-color: var(--danger); background: var(--danger); color: #fff; }

  .timer {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 12px;
    padding: 11px 13px;
    border: 1px solid var(--rule);
    border-radius: var(--radius);
    background: var(--paper);
  }
  .remaining span { display: block; color: var(--ink-3); font-size: 12px; }
  .remaining strong { font-size: 34px; font-weight: 600; line-height: 1; }
  .remaining small { margin-left: 3px; color: var(--ink-3); font-size: 15px; }

  .set { display: flex; gap: 6px; }
  .set input {
    width: 66px;
    height: 38px;
    border: 1px solid var(--rule-strong);
    border-radius: var(--radius);
    background: var(--field);
    font-size: 16px;
    font-weight: 600;
    text-align: center;
  }
  .set input:focus { border-color: var(--ink); }
  .apply {
    height: 38px;
    padding: 0 12px;
    border-radius: var(--radius);
    background: var(--ink);
    color: var(--paper);
    font-size: 13.5px;
    font-weight: 600;
  }
  .apply:hover:not(:disabled) { background: var(--ink-2); }
  .set input:disabled,
  .apply:disabled { opacity: .45; }

  .release { display: grid; grid-template-columns: 1fr 1.25fr; gap: 8px; }
  .release button { min-height: 50px; border-radius: var(--radius); font-size: 14.5px; font-weight: 700; }
  .arm {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    border: 1.5px solid var(--danger);
    color: var(--danger);
  }
  .arm:hover:not(:disabled) { background: var(--danger-soft); }
  .arm.unlocked { background: var(--danger-soft); }
  .arm:disabled { border-color: var(--rule-strong); color: var(--ink-3); }
  .fire {
    background: repeating-linear-gradient(-45deg, var(--paper-2) 0 8px, var(--rule) 8px 16px);
    color: var(--ink-3);
    letter-spacing: .05em;
  }
  .fire.ready { background: var(--danger); color: #fff; box-shadow: 0 4px 14px rgba(179, 38, 30, .35); }
  .fire.ready:hover { filter: brightness(1.08); }

  .gate { color: var(--ink-3); font-size: 12px; line-height: 1.5; }
  .command { color: var(--ink-3); font-size: 12px; }
  .command b { color: var(--ink-2); font-weight: 600; }
  .command.failed,
  .command.failed b { color: var(--danger); }
  .command .detail { display: block; margin-top: 2px; }
  .error-message { color: var(--danger); font-size: 12.5px; }

  @media (max-height: 820px) {
    .control { gap: 10px; padding-top: 13px; padding-bottom: 13px; }
    .release button { min-height: 44px; }
  }
</style>
