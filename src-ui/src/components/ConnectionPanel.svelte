<script lang="ts">
  import { store } from '@/lib/stores.svelte';
  import { startMonitoring, stopMonitoring } from '@/lib/tauri';

  let portPath = $state('COM3');
  let baudRate = $state(115200);
  let loading = $state(false);
  let errorMsg = $state('');

  const baudRates = [9600, 19200, 38400, 57600, 115200];

  let connected = $derived(store.connected);

  async function handleConnect() {
    if (connected) {
      loading = true;
      errorMsg = '';
      try {
        await stopMonitoring();
        store.setConnected(false);
      } catch (e: any) {
        errorMsg = e?.detail || e?.message || String(e);
      } finally {
        loading = false;
      }
    } else {
      if (!portPath.trim()) {
        errorMsg = 'Please enter a COM port path';
        return;
      }
      loading = true;
      errorMsg = '';
      try {
        await startMonitoring(portPath.trim(), baudRate);
        store.setConnected(true);
      } catch (e: any) {
        errorMsg = e?.detail || e?.message || String(e);
        store.setConnected(false);
      } finally {
        loading = false;
      }
    }
  }
</script>

<div class="connection-panel">
  <div class="panel-header">
    <div class="header-icon">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M12 2L2 7l10 5 10-5-10-5z"/>
        <path d="M2 17l10 5 10-5"/>
        <path d="M2 12l10 5 10-5"/>
      </svg>
    </div>
    <h3>串口連接</h3>
    <div class="status-led" class:connected class:disconnected={!connected}>
      <div class="led-glow"></div>
    </div>
  </div>

  <div class="form-group">
    <label for="port-path">COM Port</label>
    <input
      id="port-path"
      type="text"
      bind:value={portPath}
      placeholder="COM3 or /dev/ttyUSB0"
      disabled={connected || loading}
    />
  </div>

  <div class="form-group">
    <label for="baud-rate">Baud Rate</label>
    <select id="baud-rate" bind:value={baudRate} disabled={connected || loading}>
      {#each baudRates as rate}
        <option value={rate}>{rate.toLocaleString()}</option>
      {/each}
    </select>
  </div>

  <button
    class="connect-btn"
    class:connected
    onclick={handleConnect}
    disabled={loading}
  >
    {#if loading}
      <span class="spinner"></span>
      處理中...
    {:else if connected}
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
      斷開連接
    {:else}
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polygon points="5 3 19 12 5 21 5 3"/></svg>
      開始連接
    {/if}
  </button>

  {#if errorMsg}
    <div class="error-msg">{errorMsg}</div>
  {/if}

  <div class="conn-info">
    <span class="info-label">狀態</span>
    <span class="info-value" class:text-green={connected} class:text-red={!connected}>
      {connected ? '已連接' : '未連接'}
    </span>
  </div>
</div>

<style>
  .connection-panel {
    background: var(--glass-bg);
    backdrop-filter: var(--glass-blur);
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-lg);
    padding: var(--sp-5);
    box-shadow: var(--glass-shadow);
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
    animation: slide-up 0.4s ease-out forwards;
  }

  .panel-header {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
  }

  .header-icon {
    color: var(--accent-cyan);
    display: flex;
  }

  .panel-header h3 {
    font-size: var(--fs-md);
    font-weight: 600;
    color: var(--text-primary);
    flex: 1;
  }

  .status-led {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    position: relative;
  }

  .status-led.connected {
    background: var(--accent-green);
    box-shadow: 0 0 8px var(--accent-green-glow);
  }

  .status-led.disconnected {
    background: var(--accent-red);
    box-shadow: 0 0 8px var(--accent-red-glow);
  }

  .status-led .led-glow {
    position: absolute;
    inset: -4px;
    border-radius: 50%;
    animation: pulse 2s ease-in-out infinite;
  }

  .status-led.connected .led-glow {
    background: var(--accent-green-dim);
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: var(--sp-1);
  }

  .form-group label {
    font-size: var(--fs-xs);
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 500;
  }

  .form-group input,
  .form-group select {
    background: var(--surface);
    color: var(--text-primary);
    border: 1px solid var(--surface-border);
    border-radius: var(--radius-sm);
    padding: var(--sp-2) var(--sp-3);
    font-size: var(--fs-base);
    font-family: var(--font-mono);
    transition: border-color var(--transition-fast);
  }

  .form-group input:focus,
  .form-group select:focus {
    border-color: var(--accent-cyan);
    box-shadow: 0 0 0 2px var(--accent-cyan-dim);
  }

  .form-group input:disabled,
  .form-group select:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .connect-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--sp-2);
    padding: var(--sp-3) var(--sp-4);
    border-radius: var(--radius-sm);
    font-size: var(--fs-base);
    font-weight: 600;
    transition: all var(--transition-base);
    background: linear-gradient(135deg, var(--accent-cyan), #0099cc);
    color: #000;
  }

  .connect-btn:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 4px 16px var(--accent-cyan-glow);
  }

  .connect-btn:active:not(:disabled) {
    transform: translateY(0);
  }

  .connect-btn.connected {
    background: linear-gradient(135deg, var(--accent-red), #cc2020);
    color: #fff;
  }

  .connect-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid transparent;
    border-top-color: currentColor;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  .error-msg {
    background: var(--accent-red-dim);
    color: var(--accent-red);
    border: 1px solid rgba(255, 59, 59, 0.2);
    border-radius: var(--radius-sm);
    padding: var(--sp-2) var(--sp-3);
    font-size: var(--fs-sm);
  }

  .conn-info {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-top: var(--sp-2);
    border-top: 1px solid var(--surface-border);
  }

  .info-label {
    font-size: var(--fs-xs);
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .info-value {
    font-size: var(--fs-sm);
    font-weight: 600;
    font-family: var(--font-mono);
  }

  .text-green { color: var(--accent-green); }
  .text-red { color: var(--accent-red); }
</style>
