<script lang="ts">
  import { store } from '@/lib/stores.svelte';
  import { startTestMonitoring } from '@/lib/tauri';
  import {
    loadSessionDefaults,
    saveSessionDefaults,
    validateTestMetadata,
  } from '@/lib/session-defaults.js';

  let dialogElement: HTMLDialogElement;
  let purpose = $state('');
  let operator = $state('');
  let location = $state('');
  let initialBatteryVoltage = $state<number | null>(null);
  let notes = $state('');
  let acknowledgedUnrecorded = $state(false);
  let busy = $state(false);
  let errorMessage = $state('');
  let fieldErrors = $state<Record<string, string>>({});
  let lastRequestId = 0;

  let request = $derived(store.testStartRequest);
  let storage = $derived(store.storageStatus);
  let storageFailed = $derived(storage.phase === 'failed');
  let storageInitializing = $derived(storage.phase === 'initializing');

  $effect(() => {
    const next = request;
    if (next && next.id !== lastRequestId) {
      lastRequestId = next.id;
      const defaults = loadSessionDefaults(
        typeof localStorage === 'undefined' ? undefined : localStorage,
      );
      purpose = '';
      operator = defaults.operator;
      location = defaults.location;
      initialBatteryVoltage = null;
      notes = '';
      acknowledgedUnrecorded = false;
      fieldErrors = {};
      errorMessage = '';
      queueMicrotask(() => {
        if (dialogElement && !dialogElement.open) dialogElement.showModal();
      });
    } else if (!next && dialogElement?.open) {
      dialogElement.close();
    }
  });

  function errorText(error: unknown): string {
    if (typeof error === 'object' && error !== null) {
      const value = error as { detail?: string; message?: string };
      return value.detail ?? value.message ?? String(error);
    }
    return String(error);
  }

  function cancel() {
    if (busy) return;
    store.cancelTestStart();
  }

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (!request) return;
    const metadata = {
      purpose: purpose.trim(),
      operator: operator.trim(),
      location: location.trim(),
      initialBatteryVoltage: Number(initialBatteryVoltage),
      notes: notes.trim(),
    };
    fieldErrors = validateTestMetadata(metadata);
    if (Object.keys(fieldErrors).length > 0) return;
    if (storageInitializing) {
      errorMessage = '儲存系統仍在初始化，請稍候';
      return;
    }
    if (storageFailed && !acknowledgedUnrecorded) {
      errorMessage = '必須確認了解資料不會保存，才能進入僅監控模式';
      return;
    }
    busy = true;
    errorMessage = '';
    try {
      const status = await startTestMonitoring(
        request.path,
        request.baudRate,
        metadata,
        storageFailed,
      );
      saveSessionDefaults(
        typeof localStorage === 'undefined' ? undefined : localStorage,
        metadata,
      );
      store.updateTestSessionStatus(status);
    } catch (error) {
      errorMessage = errorText(error);
    } finally {
      busy = false;
    }
  }
</script>

<dialog
  bind:this={dialogElement}
  class="test-session-dialog"
  aria-modal="true"
  aria-labelledby="test-session-title"
  oncancel={(event) => event.preventDefault()}
>
  <form onsubmit={submit} novalidate>
    <header>
      <div>
        <h2 id="test-session-title">開始本次測試</h2>
        <p class="connection-target mono">{request?.path ?? '--'} · {request?.baudRate?.toLocaleString() ?? '--'} Baud</p>
      </div>
      <span class:failed={storageFailed} class:degraded={storage.phase === 'degraded'} class="storage-state">
        {storage.phase === 'healthy' ? '儲存正常' : storage.phase === 'degraded' ? '儲存降級' : storage.phase === 'failed' ? '儲存失敗' : '初始化中'}
      </span>
    </header>

    {#if storage.phase !== 'healthy'}
      <div class:critical={storageFailed} class="storage-warning">
        <strong>{storage.lastError ?? (storageInitializing ? '正在準備 SQLite 與場次資料夾' : '儲存系統目前不是正常狀態')}</strong>
        {#if storage.dataPath}<small>{storage.dataPath}</small>{/if}
      </div>
    {/if}

    <label for="test-purpose">測試目的<span>*</span></label>
    <textarea id="test-purpose" bind:value={purpose} disabled={busy} placeholder="例如：30 分鐘半雙工穩定測試"></textarea>
    {#if fieldErrors.purpose}<small class="field-error">{fieldErrors.purpose}</small>{/if}

    <div class="field-grid">
      <div>
        <label for="test-operator">操作者<span>*</span></label>
        <input id="test-operator" bind:value={operator} disabled={busy} />
        {#if fieldErrors.operator}<small class="field-error">{fieldErrors.operator}</small>{/if}
      </div>
      <div>
        <label for="test-location">測試地點<span>*</span></label>
        <input id="test-location" bind:value={location} disabled={busy} />
        {#if fieldErrors.location}<small class="field-error">{fieldErrors.location}</small>{/if}
      </div>
    </div>

    <label for="test-battery">起始電池電壓（V）<span>*</span></label>
    <input id="test-battery" type="number" min="0.1" step="0.1" bind:value={initialBatteryVoltage} disabled={busy} />
    {#if fieldErrors.initialBatteryVoltage}<small class="field-error">{fieldErrors.initialBatteryVoltage}</small>{/if}

    <label for="test-notes">備註</label>
    <textarea id="test-notes" bind:value={notes} disabled={busy}></textarea>

    {#if storageFailed}
      <label class="unrecorded-confirm">
        <input type="checkbox" bind:checked={acknowledgedUnrecorded} disabled={busy} />
        <span>我了解本次資料不會永久保存，仍要進入僅監控模式</span>
      </label>
    {/if}

    {#if errorMessage}<div class="dialog-error">{errorMessage}</div>{/if}

    <footer>
      <button type="button" class="cancel" onclick={cancel} disabled={busy}>取消</button>
      <button
        type="submit"
        class:danger={storageFailed}
        disabled={busy || storageInitializing || (storageFailed && !acknowledgedUnrecorded)}
      >
        {busy ? '啟動中…' : storageFailed ? '僅監控，不記錄' : '開始監控並記錄'}
      </button>
    </footer>
  </form>
</dialog>

<style>
  .test-session-dialog {
    width: min(680px, calc(100vw - 32px));
    max-height: calc(100vh - 32px);
    margin: auto;
    padding: 0;
    border: 1px solid var(--rule-strong);
    border-radius: var(--radius-lg);
    background: var(--sheet);
    color: var(--ink);
    box-shadow: 0 24px 60px rgba(15, 27, 45, .28);
  }
  .test-session-dialog::backdrop { background: rgba(15, 27, 45, .55); }
  form { display: grid; gap: 10px; padding: clamp(20px, 3vw, 30px); overflow-y: auto; }
  header, footer { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  header { margin-bottom: 6px; padding-bottom: 16px; border-bottom: 1px solid var(--rule); }
  h2 { font-size: 22px; font-weight: 700; }
  .connection-target { margin-top: 3px; color: var(--ink-3); font-family: var(--font-num); font-size: 13px; font-weight: 500; }
  .storage-state { color: var(--live); font-size: 13px; font-weight: 700; }
  .storage-state.degraded { color: var(--warn); }
  .storage-state.failed { color: var(--danger); }
  .storage-warning, .dialog-error {
    display: grid; gap: 4px; padding: 10px 12px;
    border: 1px solid var(--warn); border-radius: var(--radius);
    background: var(--warn-soft); color: var(--warn); font-size: 12.5px;
  }
  .storage-warning.critical, .dialog-error { border-color: var(--danger); background: var(--danger-soft); color: var(--danger); }
  label { margin-top: 4px; color: var(--ink-2); font-size: 13px; }
  label span { margin-left: 2px; color: var(--danger); }
  input, textarea {
    width: 100%; padding: 9px 11px; border: 1px solid var(--rule-strong);
    border-radius: var(--radius); background: var(--field); color: var(--ink);
  }
  input[type="number"] { font-family: var(--font-num); font-weight: 600; }
  input:focus, textarea:focus { border-color: var(--rail); box-shadow: 0 0 0 3px color-mix(in srgb, var(--rail) 18%, transparent); }
  textarea { min-height: 70px; resize: vertical; }
  .field-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  .field-grid > div { display: grid; gap: 6px; }
  .field-error { color: var(--danger); font-size: 12px; }
  .unrecorded-confirm { display: flex; align-items: flex-start; gap: 8px; padding: 10px 12px; border: 1px solid var(--danger); border-radius: var(--radius); }
  .unrecorded-confirm input { width: auto; margin-top: 3px; accent-color: var(--danger); }
  .unrecorded-confirm span { color: var(--ink); }
  footer { margin-top: 10px; padding-top: 16px; border-top: 1px solid var(--rule); justify-content: flex-end; }
  button { min-height: 40px; padding: 0 18px; border-radius: var(--radius); background: var(--rail); color: var(--rail-ink); font-weight: 700; }
  button:hover:not(:disabled) { background: var(--rail-deep); }
  button.cancel { border: 1px solid var(--rule-strong); background: transparent; color: var(--ink); }
  button.cancel:hover:not(:disabled) { background: var(--paper); }
  button.danger { background: var(--danger); color: #fff; }
  button:disabled { opacity: .45; }
  @media (max-width: 560px) { .field-grid { grid-template-columns: 1fr; } }
</style>
