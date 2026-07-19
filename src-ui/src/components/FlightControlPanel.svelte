<script lang="ts">
  import { store } from '@/lib/stores.svelte';
  import { forceRelease, setTimer } from '@/lib/tauri';

  let timerSeconds = $state(30);
  let safetyUnlocked = $state(false);
  let busy = $state(false);
  let errorMessage = $state('');

  let telemetry = $derived(store.telemetry);
  let commandStatus = $derived(store.commandStatus);
  let stats = $derived(store.flightStats);
  let session = $derived(store.testSessionStatus);
  let controlsEnabled = $derived(
    session.phase === 'recording' || session.phase === 'monitoring_unrecorded',
  );
  let lastPacket = $derived(
    store.lastPacketAt === null
      ? '--'
      : new Date(store.lastPacketAt).toLocaleTimeString('zh-TW', { hour12: false }),
  );

  const phaseLabels = {
    disconnected: '未連線',
    starting: '啟動中',
    recording: '記錄中',
    monitoring_unrecorded: '僅監控（不記錄）',
    finishing: '結束中',
    completed: '已完成',
    interrupted: '未正常完成',
    failed: '啟動失敗',
  } as const;

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
    if (!controlsEnabled || !safetyUnlocked) return;
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

<section class="flight-control-panel">
  <div class="panel-header">
    <div><span>COMMAND CONSOLE</span><h3>飛行控制</h3></div>
    <span class:deployed={telemetry.deployState === 1} class="deploy-state">
      {telemetry.deployState === 1 ? 'DEPLOYED' : 'SAFE'}
    </span>
  </div>

  <div class="test-run-state" class:recording={session.phase === 'recording'} class:warning={session.phase === 'monitoring_unrecorded' || session.phase === 'interrupted'}>
    <strong>{phaseLabels[session.phase]}</strong>
    {#if session.purpose}<span>目的：{session.purpose}</span>{/if}
    {#if session.testRunId}<span class="mono">場次 ID：{session.testRunId}</span>{/if}
    {#if session.directory}<span class="directory">資料夾：{session.directory}</span>{/if}
    {#if session.detail}<small>{session.detail}</small>{/if}
  </div>

  <div class="air-state">
    <span>空中 Session <strong class="mono">{telemetry.sessionId ? `0x${telemetry.sessionId.toString(16).toUpperCase().padStart(8, '0')}` : '--'}</strong></span>
    <span>空中端剩餘 <strong class="mono">{telemetry.remainingS} s</strong></span>
    <span>最後封包 <strong class="mono">{lastPacket}</strong></span>
  </div>

  <div class="timer-row">
    <label for="timer-seconds">設定倒數（秒）</label>
    <div>
      <input id="timer-seconds" type="number" min="1" step="1" bind:value={timerSeconds} disabled={busy || !controlsEnabled || telemetry.deployState === 1} />
      <button onclick={applyTimer} disabled={busy || !controlsEnabled || telemetry.deployState === 1}>覆蓋 timer</button>
    </div>
  </div>

  <div class="release-controls">
    <button
      class:unlocked={safetyUnlocked}
      class="safety-button"
      onclick={() => { safetyUnlocked = !safetyUnlocked; }}
      disabled={busy || !controlsEnabled || telemetry.deployState === 1}
    >
      {safetyUnlocked ? '安全鎖已解除' : '解除安全鎖'}
    </button>
    <button
      class="release-button"
      onclick={releaseNow}
      disabled={busy || !controlsEnabled || !safetyUnlocked || telemetry.deployState === 1}
    >
      FORCE RELEASE
    </button>
  </div>

  <div class="command-status" class:failed={commandStatus?.status === 'failed'}>
    <span>指令狀態</span>
    <strong>{commandStatus?.status ?? '待命'}</strong>
    {#if commandStatus}
      <small class="mono">{commandStatus.commandType} · ID {commandStatus.commandId ?? '--'} · 第 {commandStatus.attempts} 次</small>
      <small>{commandStatus.detail}</small>
    {/if}
  </div>

  <div class="stats-grid">
    <span>遺失 <strong>{stats.lostPackets}</strong></span>
    <span>重複 <strong>{stats.duplicatePackets}</strong></span>
    <span>CRC <strong>{stats.crcErrors}</strong></span>
    <span>失聯 <strong>{stats.linkOutages}</strong></span>
    <span>最長失聯 <strong>{(stats.maxLinkLossMs / 1000).toFixed(1)} s</strong></span>
    <span>重啟 <strong>{stats.restartCount}</strong></span>
  </div>

  {#if errorMessage}
    <div class="error-message">{errorMessage}</div>
  {/if}
</section>

<style>
  .flight-control-panel {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    padding: var(--sp-5);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-lg);
    background: var(--glass-bg);
    box-shadow: var(--glass-shadow);
  }
  .panel-header,
  .timer-row > div,
  .release-controls {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
  }
  .panel-header { justify-content: space-between; }
  .panel-header > div > span { color: var(--accent-cyan); font-family: var(--font-mono); font-size: 9px; letter-spacing: .13em; }
  h3 { margin-top: 3px; font-size: var(--fs-md); font-weight: 560; }
  .deploy-state { color: var(--accent-green); font-family: var(--font-mono); font-size: var(--fs-xs); }
  .deploy-state.deployed { color: var(--accent-red); }
  .air-state,
  .command-status,
  .test-run-state,
  .stats-grid { display: grid; gap: var(--sp-1); }
  .test-run-state { padding: var(--sp-3); border: 1px solid var(--surface-border); border-radius: var(--radius-sm); background: rgba(5, 13, 18, .3); color: var(--text-secondary); font-size: var(--fs-xs); }
  .test-run-state.recording { border-color: rgba(115, 210, 182, .34); }
  .test-run-state.warning { border-color: rgba(221, 169, 93, .4); }
  .directory { overflow-wrap: anywhere; }
  .air-state { color: var(--text-secondary); font-size: var(--fs-xs); }
  label { color: var(--text-secondary); font-size: var(--fs-xs); }
  input {
    width: 100%;
    padding: var(--sp-2);
    border: 1px solid var(--surface-border);
    border-radius: var(--radius-sm);
    background: var(--bg-field);
    color: var(--text-primary);
  }
  button {
    padding: var(--sp-2) var(--sp-3);
    border-radius: var(--radius-sm);
    border: 1px solid var(--surface-border);
    background: transparent;
    color: var(--text-primary);
  }
  button:disabled { cursor: not-allowed; opacity: 0.4; }
  .safety-button.unlocked { border: 1px solid var(--accent-orange); color: var(--accent-orange); }
  .release-button { border-color: rgba(229, 109, 121, .5); background: var(--accent-red-dim); color: var(--accent-red); font-weight: 700; letter-spacing: .04em; }
  .command-status,
  .stats-grid { padding: var(--sp-3); border-radius: var(--radius-sm); background: var(--bg-field); font-size: var(--fs-xs); }
  .command-status.failed { border: 1px solid var(--accent-red); }
  .stats-grid { grid-template-columns: 1fr 1fr; color: var(--text-secondary); }
  .stats-grid strong { color: var(--text-primary); }
  .error-message { color: var(--accent-red); font-size: var(--fs-xs); }
</style>
