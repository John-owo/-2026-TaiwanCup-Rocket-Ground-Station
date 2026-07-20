import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(new URL(path, import.meta.url), 'utf8');

test('flight controls expose timer, guarded force release, and airborne state', () => {
  const source = read('../components/FlightControlPanel.svelte');
  assert.match(source, /await setTimer\(timerSeconds\)/u);
  assert.match(source, /if \(!forceAvailable \|\| !safetyUnlocked\) return;/u);
  assert.match(source, /await forceRelease\(\)/u);
  assert.match(source, /safetyUnlocked = false/u);
  assert.match(source, /空中端剩餘/u);
  assert.match(source, /最後封包/u);
  assert.match(source, /DEPLOYED/u);
  assert.match(source, /空中端已 DEPLOYED/u);
  assert.match(source, /!controlsEnabled \|\| telemetry\.deployState === 1/u);
  assert.match(source, /airborneLinkState !== 'live'/u);
  assert.match(source, /safetyUnlocked = false/u);
  assert.match(source, /observedSessionId/u);
  assert.doesNotMatch(source, /long.?press|長按/iu);
});

test('status bar exposes four link states and separates parse failures from CRC errors', () => {
  const source = read('../components/StatusBar.svelte');
  for (const label of ['待命', '等待資料', '接收中', '失聯', '解析失敗', 'CRC 錯誤']) {
    assert.equal(source.includes(label), true, label);
  }
  assert.match(source, /flightStats = \$derived\(store\.flightStats\)/u);
  assert.match(source, /flightStats\.crcErrors/u);
  assert.doesNotMatch(source, />CRC 失敗</u);
});

test('flight session UI displays backend-owned run status and separate statistics', () => {
  const source = read('../components/FlightControlPanel.svelte');
  for (const token of [
    'testSessionStatus',
    'testRunId',
    'purpose',
    'directory',
    '記錄中',
    'lostPackets',
    'duplicatePackets',
    'crcErrors',
    'linkOutages',
    'maxLinkLossMs',
    'restartCount',
  ]) {
    assert.equal(source.includes(token), true, token);
  }
  assert.doesNotMatch(source, /sessionActive|開始新場次|startFlightSession/u);
});

test('frontend bridge listens for command and flight-stat events', () => {
  const source = read('./tauri.ts');
  assert.match(source, /invoke\('set_timer', \{ durationS \}\)/u);
  assert.match(source, /invoke\('force_release'\)/u);
  assert.match(source, /listen<CommandStatus>\('command-status'/u);
  assert.match(source, /listen<FlightStats>\('flight-stats'/u);
  assert.match(source, /listen<TestSessionStatus>\('test-session-status'/u);
  assert.match(source, /listen<StorageStatus>\('storage-status'/u);
});

test('store timestamps every telemetry packet and preserves command status', () => {
  const source = read('./stores.svelte.ts');
  assert.match(source, /lastPacketAt = Date\.now\(\)/u);
  assert.match(source, /updateCommandStatus\(status: CommandStatus\)/u);
  assert.match(source, /updateFlightStats\(nextStats: FlightStats\)/u);
  assert.match(source, /if \(nextConnected && !connected\)[\s\S]*lastPacketAt = null/u);
});
