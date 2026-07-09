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
        const detail = e?.detail || e?.message || String(e);
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
    <h3>Serial Link</h3>
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

  <button class="connect-btn" class:connected onclick={handleConnect} disabled={loading}>
    {#if loading}
      <span class="spinner"></span>
      Connecting...
    {:else if connected}
      Disconnect
    {:else}
      Start Monitoring
    {/if}
  </button>

  {#if errorMsg}
    <div class="error-msg">{errorMsg}</div>
  {/if}

  <div class="conn-info">
    <span class="info-label">Packet</span>
    <span class="info-value mono">0xAA + 52B + CRC16</span>
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
    backdrop-filter: var(--glass-blur);
    box-shadow: var(--glass-shadow);
    animation: slide-up 0.4s ease-out forwards;
  }

  .panel-header {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
  }

  .header-icon {
    display: flex;
    color: var(--accent-cyan);
  }

  .panel-header h3 {
    flex: 1;
    color: var(--text-primary);
    font-size: var(--fs-md);
    font-weight: 600;
  }

  .status-led {
    position: relative;
    width: 10px;
    height: 10px;
    border-radius: 50%;
  }

  .status-led.connected {
    background: var(--accent-green);
    box-shadow: 0 0 8px var(--accent-green-glow);
  }

  .status-led.disconnected {
    background: var(--accent-red);
    box-shadow: 0 0 8px var(--accent-red-glow);
  }

  .led-glow {
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
    color: var(--text-secondary);
    font-size: var(--fs-xs);
    font-weight: 500;
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }

  .form-group input,
  .form-group select {
    padding: var(--sp-2) var(--sp-3);
    border: 1px solid var(--surface-border);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: var(--fs-base);
    transition: border-color var(--transition-fast), box-shadow var(--transition-fast);
  }

  .form-group input:focus,
  .form-group select:focus {
    border-color: var(--accent-cyan);
    box-shadow: 0 0 0 2px var(--accent-cyan-dim);
  }

  .form-group input:disabled,
  .form-group select:disabled {
    opacity: 0.5;
  }

  .connect-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--sp-2);
    padding: var(--sp-3) var(--sp-4);
    border-radius: var(--radius-sm);
    background: linear-gradient(135deg, var(--accent-cyan), #0099cc);
    color: #000;
    font-size: var(--fs-base);
    font-weight: 700;
    transition: transform var(--transition-base), box-shadow var(--transition-base);
  }

  .connect-btn:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 4px 16px var(--accent-cyan-glow);
  }

  .connect-btn.connected {
    background: linear-gradient(135deg, var(--accent-red), #cc2020);
    color: #fff;
  }

  .connect-btn:disabled {
    opacity: 0.6;
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
    padding: var(--sp-2) var(--sp-3);
    border: 1px solid rgba(255, 59, 59, 0.2);
    border-radius: var(--radius-sm);
    background: var(--accent-red-dim);
    color: var(--accent-red);
    font-size: var(--fs-sm);
  }

  .conn-info {
    display: flex;
    flex-direction: column;
    gap: var(--sp-1);
    padding-top: var(--sp-2);
    border-top: 1px solid var(--surface-border);
  }

  .info-label {
    color: var(--text-secondary);
    font-size: var(--fs-xs);
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }

  .info-value {
    color: var(--text-tertiary);
    font-size: var(--fs-xs);
  }
</style>
