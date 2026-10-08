<script lang="ts">
  import { onMount } from 'svelte';
  import { store } from '@/lib/stores.svelte';
  import { BAUD_RATES, BODY_AXES } from '@/lib/settings.js';
  import { listSerialPorts, stopTestMonitoring } from '@/lib/tauri';
  import { getTelemetryLinkState } from '@/lib/telemetry-link.js';
  import { currentTheme, setTheme, type Theme } from '@/lib/theme';
  import type { AxisSign, SensorAxis } from '@/lib/types';

  let portPath = $state(store.settings.portPath);
  let baudRate = $state(store.settings.baudRate);
  let availablePorts = $state<string[]>([]);
  let portsLoaded = $state(false);
  let loading = $state(false);
  let errorMsg = $state('');
  let resetArmed = $state(false);
  let resetTimer: ReturnType<typeof setTimeout> | undefined;
  let nowMs = $state(Date.now());
  let theme = $state<Theme>(currentTheme());

  const axisLabels: Record<SensorAxis, string> = {
    x: '火箭 X（滾轉）',
    y: '火箭 Y（俯仰）',
    z: '火箭 Z（偏航）',
  };

  const phaseLabels = {
    disconnected: '待命',
    starting: '啟動中',
    recording: '記錄中',
    monitoring_unrecorded: '僅監控，不記錄',
    finishing: '結束中',
    completed: '已完成',
    interrupted: '未正常完成',
    failed: '啟動失敗',
  } as const;

  const linkLabels = {
    standby: '待命',
    waiting: '等待遙測資料',
    live: '接收中',
    lost: '失聯',
  } as const;

  const storageLabels = {
    initializing: '初始化中',
    healthy: '正常',
    degraded: '降級',
    failed: '失敗',
  } as const;

  let connected = $derived(store.connected);
  let session = $derived(store.testSessionStatus);
  let storage = $derived(store.storageStatus);
  let stats = $derived(store.stats);
  let flightStats = $derived(store.flightStats);
  let startDialogOpen = $derived(store.testStartRequest !== null);
  let latestSerialError = $derived(store.errors.at(-1)?.detail ?? '');
  let displayedError = $derived(errorMsg || latestSerialError);
  let savedPortUnavailable = $derived(
    portsLoaded
      && portPath.trim() !== ''
      && !availablePorts.includes(portPath.trim()),
  );
  let linkState = $derived(getTelemetryLinkState(connected, store.lastPacketAt, nowMs));
  let lastPacketAge = $derived(
    store.lastPacketAt === null ? '--' : `${Math.max(0, (nowMs - store.lastPacketAt) / 1000).toFixed(1)} s 前`,
  );
  let runLabel = $derived(session.testRunId ? session.testRunId.slice(0, 8).toUpperCase() : null);

  onMount(() => {
    void refreshPorts();
    const interval = setInterval(() => { nowMs = Date.now(); }, 250);
    return () => {
      clearInterval(interval);
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

  function toggleTheme() {
    theme = theme === 'dark' ? 'light' : 'dark';
    setTheme(theme);
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

<div class="rail">
  <header class="mark">
    <svg width="30" height="30" viewBox="0 0 30 30" fill="none" aria-hidden="true">
      <circle cx="15" cy="15" r="13" stroke="currentColor" stroke-width="1.5" />
      <path d="M15 5 L19.5 21 L15 17.8 L10.5 21 Z" fill="currentColor" />
    </svg>
    <div>
      <strong>地面站</strong>
      <span>遙測接收與指令</span>
    </div>
  </header>

  <section class="session" aria-label="目前場次">
    <h1>{session.purpose || '尚未開始測試'}</h1>
    {#if runLabel}<span class="run num">RUN {runLabel}</span>{/if}
    {#if session.directory}
      <details class="directory">
        <summary>場次資料夾</summary>
        <p>{session.directory}</p>
      </details>
    {/if}
  </section>

  <section class="link" data-state={linkState} aria-live="polite">
    <div class="link-state">
      <i aria-hidden="true"></i>
      <strong>{linkLabels[linkState]}</strong>
    </div>
    <dl>
      <dt>最後封包</dt><dd class="num">{lastPacketAge}</dd>
      <dt>接收頻率</dt><dd class="num">{stats.packetsPerSecond.toFixed(2)} Hz</dd>
      <dt>遺失 / CRC</dt><dd class="num">{flightStats.lostPackets} / {flightStats.crcErrors}</dd>
      <dt>記錄</dt><dd class:bad={session.phase === 'failed' || session.phase === 'interrupted'} class:warn={session.phase === 'monitoring_unrecorded'}>{phaseLabels[session.phase]}</dd>
      <dt>儲存</dt><dd class:bad={storage.phase === 'failed'} class:warn={storage.phase === 'degraded'}>{storageLabels[storage.phase]}</dd>
    </dl>
  </section>

  <button class="connect-btn" class:connected onclick={handleConnect} disabled={loading || startDialogOpen}>
    {#if loading}
      <span class="spinner" aria-hidden="true"></span>結束中…
    {:else if connected}
      停止監控
    {:else}
      開始監控
    {/if}
  </button>

  {#if displayedError}
    <p class="error-msg" role="alert">{displayedError}</p>
  {/if}

  <section class="port" aria-label="序列埠設定">
    <div class="field">
      <label for="port-path">序列埠</label>
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
        <button class="text-btn" onclick={refreshPorts} disabled={connected || loading}>重新掃描</button>
      </div>
      <datalist id="serial-ports">
        {#each availablePorts as port}
          <option value={port}></option>
        {/each}
      </datalist>
      {#if savedPortUnavailable}
        <span class="field-warning">已保存的序列埠目前不可用，請確認裝置連線</span>
      {/if}
    </div>

    <div class="field">
      <label for="baud-rate">鮑率</label>
      <select id="baud-rate" bind:value={baudRate} onchange={persistBaudRate} disabled={connected || loading}>
        {#each BAUD_RATES as rate}
          <option value={rate}>{rate.toLocaleString()}</option>
        {/each}
      </select>
    </div>
    <p class="protocol">協定 <span class="num">v1 / v2 · 94 B / 63 B</span></p>
  </section>

  <details class="axis-settings">
    <summary>姿態軸向設定</summary>
    <p class="settings-note">變更軸向會立即歸零姿態。</p>
    {#each BODY_AXES as bodyAxis}
      <div class="axis-row">
        <label for="axis-{bodyAxis}">{axisLabels[bodyAxis]}</label>
        <div class="axis-controls">
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
      </div>
    {/each}
    <div class="settings-actions">
      <button class="text-btn" onclick={() => store.resetAxisMapping()}>恢復預設軸向</button>
      <button class="text-btn" class:armed={resetArmed} onclick={handleResetAll}>
        {resetArmed ? '再次點擊確認' : '恢復所有設定'}
      </button>
    </div>
  </details>

  <button class="theme-btn" onclick={toggleTheme} aria-label="切換日間或夜間配色">
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
      {#if theme === 'dark'}
        <circle cx="8" cy="8" r="3.2" stroke="currentColor" stroke-width="1.6" />
        <path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
      {:else}
        <path d="M13.5 9.6A5.6 5.6 0 0 1 6.4 2.5a5.6 5.6 0 1 0 7.1 7.1Z" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round" />
      {/if}
    </svg>
    {theme === 'dark' ? '日間配色' : '夜間配色'}
  </button>
</div>

<style>
  .rail {
    display: flex;
    flex-direction: column;
    gap: 20px;
    height: 100%;
    padding: 22px 20px 18px;
    overflow-y: auto;
    background: var(--rail);
    color: var(--rail-ink);
  }

  .rail::-webkit-scrollbar-thumb { background: var(--rail-rule) padding-box; }

  .mark { display: flex; align-items: center; gap: 10px; }
  .mark svg { flex: none; }
  .mark strong { display: block; font-size: 16px; font-weight: 700; line-height: 1.1; }
  .mark span { display: block; margin-top: 3px; color: var(--rail-ink-2); font-size: 12px; }

  .session { padding-top: 18px; border-top: 1px solid var(--rail-rule); }
  .session h1 {
    font-size: 21px;
    font-weight: 700;
    line-height: 1.28;
    letter-spacing: -.005em;
    overflow-wrap: anywhere;
    text-wrap: balance;
  }
  .run { display: block; margin-top: 6px; color: var(--rail-ink-2); font-size: 13px; font-weight: 500; letter-spacing: .02em; }
  .directory { margin-top: 10px; font-size: 12px; color: var(--rail-ink-2); }
  summary {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    list-style: none;
  }
  summary::-webkit-details-marker { display: none; }
  summary::before {
    content: '';
    width: 6px;
    height: 6px;
    border-right: 1.5px solid var(--rail-ink-2);
    border-bottom: 1.5px solid var(--rail-ink-2);
    transform: rotate(-45deg);
    transition: transform 160ms var(--ease-out);
  }
  details[open] > summary::before { transform: rotate(45deg); }
  .directory p { margin-top: 6px; overflow-wrap: anywhere; color: var(--rail-ink); }

  .link { padding: 14px 14px 12px; border-radius: var(--radius-lg); background: var(--rail-deep); }
  .link-state { display: flex; align-items: center; gap: 10px; }
  .link-state strong { font-size: 20px; font-weight: 700; line-height: 1.1; }
  .link-state i {
    width: 12px;
    height: 12px;
    flex: none;
    border: 2px solid var(--rail-ink-2);
    border-radius: 50%;
  }
  .link[data-state="live"] i { border-color: var(--live-on-rail); background: var(--live-on-rail); outline: 1px solid var(--rail-deep); }
  .link[data-state="waiting"] i { border-style: dashed; }
  .link[data-state="lost"] { background: #8e1f1a; }
  .link[data-state="lost"] i { border-color: #ffd2cc; background: #ffd2cc; }
  .link dl { display: grid; grid-template-columns: auto 1fr; gap: 5px 12px; margin-top: 12px; font-size: 13px; }
  .link dt { color: var(--rail-ink-2); }
  .link dd { text-align: right; font-weight: 600; }
  .link dd.warn { color: #ffd38a; }
  .link dd.bad { color: #ffb4ab; }

  .connect-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-height: 46px;
    border-radius: var(--radius-lg);
    background: var(--rail-ink);
    color: var(--rail);
    font-size: 15px;
    font-weight: 700;
    transition: background-color 160ms var(--ease-out), color 160ms var(--ease-out);
  }
  .connect-btn:hover:not(:disabled) { background: #ffffff; }
  .connect-btn.connected { border: 1.5px solid var(--rail-ink); background: transparent; color: var(--rail-ink); }
  .connect-btn.connected:hover:not(:disabled) { background: rgba(238, 243, 250, .1); }
  .connect-btn:disabled { opacity: .55; }
  .spinner { width: 14px; height: 14px; border: 2px solid transparent; border-top-color: currentColor; border-radius: 50%; animation: spin .6s linear infinite; }

  .error-msg {
    padding: 10px 12px;
    border-radius: var(--radius);
    background: #8e1f1a;
    color: #fff1ef;
    font-size: 12.5px;
    overflow-wrap: anywhere;
  }

  .port { display: grid; gap: 12px; padding-top: 16px; border-top: 1px solid var(--rail-rule); }
  .field { display: grid; gap: 5px; min-width: 0; }
  label { color: var(--rail-ink-2); font-size: 12.5px; }
  .input-with-action { display: flex; gap: 8px; }
  .input-with-action input { width: 0; min-width: 0; flex: 1; }
  input,
  select {
    width: 100%;
    min-width: 0;
    height: 36px;
    padding: 0 10px;
    border: 1px solid var(--rail-rule);
    border-radius: var(--radius);
    background: var(--rail-deep);
    color: var(--rail-ink);
    font-family: var(--font-num);
    font-size: 14px;
    font-weight: 500;
  }
  select option { background: var(--rail-deep); }
  input:focus,
  select:focus { border-color: var(--rail-ink-2); }
  input:disabled,
  select:disabled { opacity: .6; }
  input::placeholder { color: var(--rail-ink-2); opacity: .8; }
  .field-warning { color: #ffd38a; font-size: 12px; }
  .protocol { color: var(--rail-ink-2); font-size: 12.5px; }
  .protocol span { color: var(--rail-ink); font-weight: 600; }

  .text-btn {
    color: var(--rail-ink);
    font-size: 12.5px;
    text-decoration: underline;
    text-decoration-color: var(--rail-rule);
    text-underline-offset: 3px;
  }
  .text-btn:hover:not(:disabled) { text-decoration-color: currentColor; }
  .text-btn:disabled { opacity: .5; }
  .text-btn.armed { color: #ffd38a; }

  .axis-settings { padding-top: 14px; border-top: 1px solid var(--rail-rule); font-size: 13px; }
  .axis-settings summary { font-weight: 600; }
  .settings-note { margin-top: 8px; color: var(--rail-ink-2); font-size: 12px; }
  .axis-row { display: grid; gap: 5px; margin-top: 12px; }
  .axis-controls { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 6px; }
  .sign-btn {
    height: 36px;
    padding: 0 10px;
    border: 1px solid var(--rail-rule);
    border-radius: var(--radius);
    color: var(--rail-ink);
    font-size: 12.5px;
  }
  .sign-btn.negative { border-color: #ffd38a; color: #ffd38a; }
  .settings-actions { display: flex; flex-wrap: wrap; gap: 14px; margin-top: 14px; }

  .theme-btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    margin-top: auto;
    color: var(--rail-ink-2);
    font-size: 12.5px;
  }
  .theme-btn:hover { color: var(--rail-ink); }

  @media (max-height: 820px) {
    .rail { gap: 16px; padding-top: 18px; }
    .session { padding-top: 14px; }
  }

  @media (max-width: 900px) {
    .rail { height: auto; }
  }
</style>
