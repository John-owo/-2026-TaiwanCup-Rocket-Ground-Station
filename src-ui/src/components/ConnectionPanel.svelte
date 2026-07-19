<script lang="ts">
  import { onMount } from 'svelte';
  import { store } from '@/lib/stores.svelte';
  import { BAUD_RATES, BODY_AXES } from '@/lib/settings.js';
  import { listSerialPorts, stopTestMonitoring } from '@/lib/tauri';
  import type { AxisSign, SensorAxis } from '@/lib/types';

  let portPath = $state(store.settings.portPath);
  let baudRate = $state(store.settings.baudRate);
  let availablePorts = $state<string[]>([]);
  let portsLoaded = $state(false);
  let loading = $state(false);
  let errorMsg = $state('');
  let resetArmed = $state(false);
  let resetTimer: ReturnType<typeof setTimeout> | undefined;

  const axisLabels: Record<SensorAxis, string> = {
    x: '火箭 X（滾轉）',
    y: '火箭 Y（俯仰）',
    z: '火箭 Z（偏航）',
  };

  let connected = $derived(store.connected);
  let startDialogOpen = $derived(store.testStartRequest !== null);
  let latestSerialError = $derived(store.errors.at(-1)?.detail ?? '');
  let displayedError = $derived(errorMsg || latestSerialError);
  let savedPortUnavailable = $derived(
    portsLoaded
      && portPath.trim() !== ''
      && !availablePorts.includes(portPath.trim()),
  );

  onMount(() => {
    void refreshPorts();
    return () => {
      if (resetTimer) clearTimeout(resetTimer);
    };
  });

  async function refreshPorts() {
    try {
      availablePorts = await listSerialPorts();
    } catch {
      availablePorts = [];
    } finally {
      portsLoaded = true;
    }
  }

  function persistPort() {
    portPath = portPath.trim();
    store.updateConnectionSettings({ portPath });
  }

  function persistBaudRate() {
    store.updateConnectionSettings({ baudRate });
  }

  function changeAxisSource(bodyAxis: SensorAxis, event: Event) {
    const source = (event.currentTarget as HTMLSelectElement).value as SensorAxis;
    store.updateAxisSource(bodyAxis, source);
  }

  function toggleAxisSign(bodyAxis: SensorAxis, sign: AxisSign) {
    store.updateAxisSign(bodyAxis, sign === 1 ? -1 : 1);
  }

  function handleResetAll() {
    if (!resetArmed) {
      resetArmed = true;
      resetTimer = setTimeout(() => { resetArmed = false; }, 3000);
      return;
    }
    if (resetTimer) clearTimeout(resetTimer);
    store.resetSettings();
    portPath = store.settings.portPath;
    baudRate = store.settings.baudRate;
    resetArmed = false;
  }

  async function handleConnect() {
    if (connected) {
      loading = true;
      errorMsg = '';
      try {
        const status = await stopTestMonitoring();
        store.updateTestSessionStatus(status);
      } catch (error: any) {
        const detail = error?.detail || error?.message || String(error);
        if (detail.includes('no monitoring task running')) {
          store.setConnected(false);
          return;
        }
        errorMsg = detail;
      } finally {
        loading = false;
      }
      return;
    }

    store.clearErrors();
    const selectedPort = portPath.trim();
    if (!selectedPort) {
      errorMsg = '請輸入 COM Port';
      return;
    }

    errorMsg = '';
    store.requestTestStart(selectedPort, baudRate);
  }
</script>

<div class="connection-panel">
  <div class="panel-header">
    <div><span>LINK SETUP</span><h3>序列連線</h3></div>
    <div class="status-led" class:connected class:disconnected={!connected}>
      <div class="led-glow"></div>
    </div>
  </div>

  <div class="form-group">
    <label for="port-path">COM Port</label>
    <div class="input-with-action">
      <input
        id="port-path"
        type="text"
        list="serial-ports"
        bind:value={portPath}
        onblur={persistPort}
        placeholder="例如 COM3"
        disabled={connected || loading}
      />
      <button class="small-btn" onclick={refreshPorts} disabled={connected || loading}>重新掃描</button>
    </div>
    <datalist id="serial-ports">
      {#each availablePorts as port}
        <option value={port}></option>
      {/each}
    </datalist>
    {#if savedPortUnavailable}
      <span class="field-warning">已保存的 COM Port 目前不可用，請確認裝置連線</span>
    {/if}
  </div>

  <div class="form-group">
    <label for="baud-rate">Baud Rate</label>
    <select
      id="baud-rate"
      bind:value={baudRate}
      onchange={persistBaudRate}
      disabled={connected || loading}
    >
      {#each BAUD_RATES as rate}
        <option value={rate}>{rate.toLocaleString()}</option>
      {/each}
    </select>
  </div>

  <button class="connect-btn" class:connected onclick={handleConnect} disabled={loading || startDialogOpen}>
    {#if loading}
      <span class="spinner"></span>
      結束中…
    {:else if connected}
      中斷連線
    {:else}
      開始監控
    {/if}
  </button>

  {#if displayedError}
    <div class="error-msg">{displayedError}</div>
  {/if}

  <details class="axis-settings">
    <summary>姿態軸向設定</summary>
    <p class="settings-note">變更軸向會立即歸零姿態。</p>
    {#each BODY_AXES as bodyAxis}
      <div class="axis-row">
        <label for="axis-{bodyAxis}">{axisLabels[bodyAxis]}</label>
        <select
          id="axis-{bodyAxis}"
          value={store.settings.axisMapping[bodyAxis].source}
          onchange={(event) => changeAxisSource(bodyAxis, event)}
        >
          {#each BODY_AXES as source}
            <option value={source}>感測器 {source.toUpperCase()}</option>
          {/each}
        </select>
        <button
          class:negative={store.settings.axisMapping[bodyAxis].sign === -1}
          class="sign-btn"
          onclick={() => toggleAxisSign(bodyAxis, store.settings.axisMapping[bodyAxis].sign)}
        >
          {store.settings.axisMapping[bodyAxis].sign === 1 ? '正向 +' : '反向 −'}
        </button>
      </div>
    {/each}
    <div class="settings-actions">
      <button class="small-btn" onclick={() => store.resetAxisMapping()}>恢復預設軸向</button>
      <button class="small-btn danger" onclick={handleResetAll}>
        {resetArmed ? '再次點擊確認' : '恢復所有設定'}
      </button>
    </div>
  </details>

  <div class="conn-info">
    <span class="info-label">封包格式</span>
    <span class="info-value mono">Protocol v1/v2 · 94B/63B TELEMETRY</span>
  </div>
</div>

<style>
  .connection-panel {
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
    padding: var(--sp-5);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-lg);
    background: var(--glass-bg);
    box-shadow: var(--glass-shadow);
  }

  .panel-header,
  .input-with-action,
  .settings-actions {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
  }

  .panel-header > div { flex: 1; }
  .panel-header span { color: var(--accent-cyan); font-family: var(--font-mono); font-size: 9px; letter-spacing: .13em; }
  .panel-header h3 { margin-top: 3px; color: var(--text-primary); font-size: var(--fs-md); font-weight: 560; }
  .status-led { position: relative; width: 10px; height: 10px; border-radius: 50%; }
  .status-led.connected { background: var(--accent-green); }
  .status-led.disconnected { background: var(--accent-red); }
  .led-glow { position: absolute; inset: -4px; border-radius: 50%; animation: pulse 2s ease-in-out infinite; }
  .status-led.connected .led-glow { background: var(--accent-green-dim); }

  .form-group { display: flex; flex-direction: column; gap: var(--sp-1); }
  .form-group > label,
  .axis-row label { color: var(--text-secondary); font-size: var(--fs-xs); font-weight: 500; }
  .input-with-action input { min-width: 0; flex: 1; }
  input,
  select {
    padding: var(--sp-2) var(--sp-3);
    border: 1px solid var(--surface-border);
    border-radius: var(--radius-sm);
    background: var(--bg-field);
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: var(--fs-base);
  }
  input:focus,
  select:focus { border-color: var(--accent-cyan); box-shadow: 0 0 0 2px var(--accent-cyan-dim); }
  input:disabled,
  select:disabled { opacity: 0.5; }

  .small-btn,
  .sign-btn {
    padding: var(--sp-2) var(--sp-3);
    border: 1px solid var(--surface-border);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-secondary);
    font-size: var(--fs-xs);
  }
  .small-btn:hover,
  .sign-btn:hover { border-color: var(--accent-cyan); color: var(--accent-cyan); }
  .small-btn.danger:hover { border-color: var(--accent-red); color: var(--accent-red); }
  .sign-btn.negative { border-color: var(--accent-orange-dim); color: var(--accent-orange); }

  .connect-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--sp-2);
    padding: var(--sp-3) var(--sp-4);
    border-radius: var(--radius-sm);
    border: 1px solid rgba(115, 210, 182, .42);
    background: var(--accent-cyan);
    color: #071016;
    font-size: var(--fs-base);
    font-weight: 700;
  }
  .connect-btn.connected { border-color: rgba(229, 109, 121, .5); background: var(--accent-red-dim); color: var(--accent-red); }
  .connect-btn:disabled { opacity: 0.6; }
  .spinner { width: 14px; height: 14px; border: 2px solid transparent; border-top-color: currentColor; border-radius: 50%; animation: spin 0.6s linear infinite; }

  .error-msg,
  .field-warning {
    color: var(--accent-red);
    font-size: var(--fs-xs);
  }
  .error-msg { padding: var(--sp-2) var(--sp-3); border: 1px solid rgba(255, 59, 59, 0.2); border-radius: var(--radius-sm); background: var(--accent-red-dim); }

  .axis-settings { padding-top: var(--sp-2); border-top: 1px solid var(--surface-border); }
  .axis-settings summary { cursor: pointer; color: var(--text-primary); font-size: var(--fs-sm); font-weight: 600; }
  .settings-note { margin: var(--sp-2) 0; color: var(--text-tertiary); font-size: var(--fs-xs); }
  .axis-row { display: grid; grid-template-columns: 1fr; gap: var(--sp-1); margin-top: var(--sp-3); }
  .axis-row select,
  .axis-row button { width: 100%; }
  .settings-actions { flex-wrap: wrap; margin-top: var(--sp-4); }

  .conn-info { display: flex; flex-direction: column; gap: var(--sp-1); padding-top: var(--sp-2); border-top: 1px solid var(--surface-border); }
  .info-label { color: var(--text-secondary); font-size: var(--fs-xs); }
  .info-value { color: var(--text-tertiary); font-size: var(--fs-xs); }
</style>
