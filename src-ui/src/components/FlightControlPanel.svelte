<script lang="ts">
  import { store } from '@/lib/stores.svelte';
  import {
    forceRelease,
    setTimer,
    startFlightSession,
    stopFlightSession,
  } from '@/lib/tauri';

  let timerSeconds = $state(30);
  let safetyUnlocked = $state(false);
  let busy = $state(false);
  let errorMessage = $state('');
  let sessionActive = $state(false);
  let initialBatteryVoltage = $state(8.2);
  let location = $state('');
  let operator = $state('');
  let notes = $state('');

  let telemetry = $derived(store.telemetry);
  let commandStatus = $derived(store.commandStatus);
  let stats = $derived(store.flightStats);
  let lastPacket = $derived(
    store.lastPacketAt === null
      ? '--'
      : new Date(store.lastPacketAt).toLocaleTimeString('zh-TW', { hour12: false }),
  );

  function errorText(error: unknown): string {
    if (typeof error === 'object' && error !== null) {
      const value = error as { detail?: string; message?: string };
      return value.detail ?? value.message ?? String(error);
    }
    return String(error);
  }

  async function applyTimer() {
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
    if (!safetyUnlocked) return;
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

  async function toggleSession() {
    busy = true;
    errorMessage = '';
    try {
      if (sessionActive) {
        const directory = await stopFlightSession();
        store.setFlightSessionDirectory(directory);
        sessionActive = false;
      } else {
        if (!Number.isFinite(initialBatteryVoltage) || initialBatteryVoltage <= 0) {
          throw new Error('請輸入有效的起始電池電壓');
        }
        if (!location.trim() || !operator.trim()) {
          throw new Error('地點與操作者為必填');
        }
        const directory = await startFlightSession({
          initialBatteryVoltage,
          location: location.trim(),
          operator: operator.trim(),
          notes: notes.trim(),
        });
        store.setFlightSessionDirectory(directory);
        sessionActive = true;
      }
    } catch (error) {
      errorMessage = errorText(error);
    } finally {
      busy = false;
    }
  }
</script>

<section class="flight-control-panel">
  <div class="panel-header">
    <h3>飛行控制</h3>
    <span class:deployed={telemetry.deployState === 1} class="deploy-state">
      {telemetry.deployState === 1 ? 'DEPLOYED' : 'SAFE'}
    </span>
  </div>

  <div class="air-state">
    <span>Session <strong class="mono">{telemetry.sessionId ? `0x${telemetry.sessionId.toString(16).toUpperCase().padStart(8, '0')}` : '--'}</strong></span>
    <span>空中端剩餘 <strong class="mono">{telemetry.remainingS} s</strong></span>
    <span>最後封包 <strong class="mono">{lastPacket}</strong></span>
  </div>

  <div class="timer-row">
    <label for="timer-seconds">設定倒數（秒）</label>
    <div>
      <input id="timer-seconds" type="number" min="1" step="1" bind:value={timerSeconds} disabled={busy || telemetry.deployState === 1} />
      <button onclick={applyTimer} disabled={busy || !store.connected || telemetry.deployState === 1}>覆蓋 timer</button>
    </div>
  </div>

  <div class="release-controls">
    <button
      class:unlocked={safetyUnlocked}
      class="safety-button"
      onclick={() => { safetyUnlocked = !safetyUnlocked; }}
      disabled={busy || telemetry.deployState === 1}
    >
      {safetyUnlocked ? '安全鎖已解除' : '解除安全鎖'}
    </button>
    <button
      class="release-button"
      onclick={releaseNow}
      disabled={busy || !store.connected || !safetyUnlocked || telemetry.deployState === 1}
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

  <details class="session-form" open>
    <summary>場次資料</summary>
    <label for="battery">起始電池電壓（V）</label>
    <input id="battery" type="number" min="0.1" step="0.1" bind:value={initialBatteryVoltage} disabled={sessionActive} />
    <label for="location">地點</label>
    <input id="location" bind:value={location} disabled={sessionActive} />
    <label for="operator">操作者</label>
    <input id="operator" bind:value={operator} disabled={sessionActive} />
    <label for="notes">備註</label>
    <textarea id="notes" bind:value={notes} disabled={sessionActive}></textarea>
    <button class:active={sessionActive} onclick={toggleSession} disabled={busy}>
      {sessionActive ? '結束並完成場次摘要' : '開始新場次'}
    </button>
    {#if store.flightSessionDirectory}
      <small class="directory">{store.flightSessionDirectory}</small>
    {/if}
  </details>

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
    margin-top: var(--sp-4);
    padding: var(--sp-4);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-lg);
    background: var(--glass-bg);
  }
  .panel-header,
  .timer-row > div,
  .release-controls {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
  }
  .panel-header { justify-content: space-between; }
  h3 { font-size: var(--fs-md); }
  .deploy-state { color: var(--accent-green); font-family: var(--font-mono); font-size: var(--fs-xs); }
  .deploy-state.deployed { color: var(--accent-red); }
  .air-state,
  .command-status,
  .session-form,
  .stats-grid { display: grid; gap: var(--sp-1); }
  .air-state { color: var(--text-secondary); font-size: var(--fs-xs); }
  label { color: var(--text-secondary); font-size: var(--fs-xs); }
  input,
  textarea {
    width: 100%;
    padding: var(--sp-2);
    border: 1px solid var(--surface-border);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text-primary);
  }
  textarea { min-height: 52px; resize: vertical; }
  button {
    padding: var(--sp-2) var(--sp-3);
    border-radius: var(--radius-sm);
    background: var(--surface-light);
    color: var(--text-primary);
  }
  button:disabled { cursor: not-allowed; opacity: 0.4; }
  .safety-button.unlocked { border: 1px solid var(--accent-orange); color: var(--accent-orange); }
  .release-button { background: var(--accent-red); color: white; font-weight: 700; }
  .command-status,
  .stats-grid { padding: var(--sp-2); border-radius: var(--radius-sm); background: var(--bg-field); font-size: var(--fs-xs); }
  .command-status.failed { border: 1px solid var(--accent-red); }
  .session-form { border-top: 1px solid var(--surface-border); padding-top: var(--sp-2); }
  .session-form summary { cursor: pointer; font-weight: 600; }
  .session-form button.active { background: var(--accent-orange); color: #111; }
  .directory { overflow-wrap: anywhere; color: var(--text-tertiary); }
  .stats-grid { grid-template-columns: 1fr 1fr; color: var(--text-secondary); }
  .stats-grid strong { color: var(--text-primary); }
  .error-message { color: var(--accent-red); font-size: var(--fs-xs); }
</style>
