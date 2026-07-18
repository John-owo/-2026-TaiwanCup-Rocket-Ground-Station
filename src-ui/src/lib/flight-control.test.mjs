import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(new URL(path, import.meta.url), 'utf8');

test('flight controls expose timer, one-click unlocked force release, and airborne state', () => {
  const source = read('../components/FlightControlPanel.svelte');
  assert.match(source, /await setTimer\(timerSeconds\)/u);
  assert.match(source, /if \(!safetyUnlocked\) return;/u);
  assert.match(source, /await forceRelease\(\)/u);
  assert.match(source, /safetyUnlocked = false/u);
  assert.match(source, /空中端剩餘/u);
  assert.match(source, /最後封包/u);
  assert.match(source, /DEPLOYED/u);
  assert.match(source, /空中端已 DEPLOYED/u);
  assert.match(source, /!store\.connected \|\| telemetry\.deployState === 1/u);
  assert.doesNotMatch(source, /long.?press|長按/iu);
});

test('flight session UI collects required metadata and displays separate statistics', () => {
  const source = read('../components/FlightControlPanel.svelte');
  for (const token of [
    'initialBatteryVoltage',
    'location',
    'operator',
    'notes',
    'flightSessionDirectory',
    'lostPackets',
    'duplicatePackets',
    'crcErrors',
    'linkOutages',
    'maxLinkLossMs',
    'restartCount',
  ]) {
    assert.equal(source.includes(token), true, token);
  }
});

test('frontend bridge listens for command and flight-stat events', () => {
  const source = read('./tauri.ts');
  assert.match(source, /invoke\('set_timer', \{ durationS \}\)/u);
  assert.match(source, /invoke\('force_release'\)/u);
  assert.match(source, /listen<CommandStatus>\('command-status'/u);
  assert.match(source, /listen<FlightStats>\('flight-stats'/u);
});

test('store timestamps every telemetry packet and preserves command status', () => {
  const source = read('./stores.svelte.ts');
  assert.match(source, /lastPacketAt = Date\.now\(\)/u);
  assert.match(source, /updateCommandStatus\(status: CommandStatus\)/u);
  assert.match(source, /updateFlightStats\(nextStats: FlightStats\)/u);
});
