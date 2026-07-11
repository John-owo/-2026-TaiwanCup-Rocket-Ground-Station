# Ground Station Attitude, Settings, and GPS Map Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver a Traditional Chinese Tauri/Svelte ground station with packet-driven MPU6050 six-axis attitude fusion, persistent serial/axis settings, and a right-side Leaflet/OpenStreetMap live GPS map.

**Architecture:** Keep sensor math, settings validation, and GPS track processing in small framework-free JavaScript modules covered by Node tests. The Svelte store owns reactive telemetry, revisions, and validated application settings; UI components consume those interfaces without placing Leaflet instances or attitude estimator internals into reactive state. The Tauri shell only changes CSP and continues transporting the existing 13-field telemetry packet unchanged.

**Tech Stack:** Svelte 5 runes, TypeScript 5.9, plain ESM JavaScript, Node test runner, Tauri 2, Leaflet, OpenStreetMap raster tiles, pnpm.

## Global Constraints

- Telemetry angular velocity is `deg/s`; never multiply packet values by `180 / Math.PI`.
- Default sensor-to-body mapping is X→X, Y→Y, Z→Z with all signs `+1`.
- Apply one mapping to both acceleration and angular velocity.
- Use acceleration correction only when magnitude is between `0.85g` and `1.15g`, with `g = 9.80665 m/s²`.
- Complementary filter time constant is `0.5 s`; use `alpha = tau / (tau + dt)`.
- A packet gap above `250 ms` establishes a new time baseline without integrating stale angular velocity.
- Yaw is relative heading only and normalizes to `0..360`.
- Persist COM Port, Baud Rate, and axis mapping under `rocket-ground-station.settings.v1`; never auto-connect on launch.
- Default Baud Rate is `115200`; default COM Port is blank.
- Use Traditional Chinese for user-facing labels and status; preserve COM, Baud, GPS, IMU, CRC, and engineering units.
- Use `https://tile.openstreetmap.org/{z}/{x}/{y}.png` with visible `© OpenStreetMap contributors` attribution.
- Do not prefetch or offer offline OpenStreetMap tiles.
- Reject invalid GPS values and `(0, 0)`; append track points only at least 2 m apart; retain at most 5,000 points.
- Desktop map is in the upper-right sidebar at 280 px height; narrow layout puts it below telemetry and above attitude.

---

### Task 1: Persistent and Validated Application Settings

**Files:**
- Create: `src-ui/src/lib/settings.js`
- Create: `src-ui/src/lib/settings.test.mjs`
- Modify: `src-ui/src/lib/types.ts`
- Modify: `src-ui/src/lib/stores.svelte.ts`

**Interfaces:**
- Produces: `DEFAULT_SETTINGS`, `SETTINGS_STORAGE_KEY`, `validateSettings(value)`, `loadSettings(storage)`, `saveSettings(storage, settings)`, `swapAxisSource(mapping, bodyAxis, source)`, and `setAxisSign(mapping, bodyAxis, sign)`.
- Produces store getters `settings`, `settingsRevision`, `axisMappingRevision`, and methods `updateConnectionSettings`, `updateAxisSource`, `updateAxisSign`, `resetAxisMapping`, `resetSettings`.
- `AxisMapping` shape: `{ x: { source: 'x'|'y'|'z', sign: 1|-1 }, y: ..., z: ... }`.

- [ ] **Step 1: Write settings tests first**

Create `settings.test.mjs` with real in-memory storage and these assertions:

```js
import assert from 'node:assert/strict';
import test from 'node:test';
import {
  DEFAULT_SETTINGS,
  SETTINGS_STORAGE_KEY,
  loadSettings,
  saveSettings,
  setAxisSign,
  swapAxisSource,
  validateSettings,
} from './settings.js';

function memoryStorage(initial = {}) {
  const values = new Map(Object.entries(initial));
  return {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => values.set(key, value),
    removeItem: (key) => values.delete(key),
  };
}

test('loads safe defaults when storage is empty or corrupt', () => {
  assert.deepEqual(loadSettings(memoryStorage()), DEFAULT_SETTINGS);
  assert.deepEqual(
    loadSettings(memoryStorage({ [SETTINGS_STORAGE_KEY]: '{bad json' })),
    DEFAULT_SETTINGS,
  );
});

test('round trips valid COM, baud, and axis mapping', () => {
  const storage = memoryStorage();
  const settings = validateSettings({
    version: 1,
    portPath: 'COM7',
    baudRate: 57600,
    axisMapping: {
      x: { source: 'z', sign: -1 },
      y: { source: 'x', sign: 1 },
      z: { source: 'y', sign: 1 },
    },
  });
  saveSettings(storage, settings);
  assert.deepEqual(loadSettings(storage), settings);
});

test('replaces invalid baud and duplicate axes with safe defaults', () => {
  assert.deepEqual(validateSettings({
    version: 1,
    portPath: 123,
    baudRate: 12345,
    axisMapping: {
      x: { source: 'x', sign: 1 },
      y: { source: 'x', sign: -1 },
      z: { source: 'z', sign: 0 },
    },
  }), DEFAULT_SETTINGS);
});

test('falls back per field without discarding valid connection values', () => {
  const validated = validateSettings({
    version: 1,
    portPath: 'COM9',
    baudRate: 12345,
    axisMapping: DEFAULT_SETTINGS.axisMapping,
  });
  assert.equal(validated.portPath, 'COM9');
  assert.equal(validated.baudRate, 115200);
});

test('swaps occupied axis sources and changes signs without mutation', () => {
  const swapped = swapAxisSource(DEFAULT_SETTINGS.axisMapping, 'x', 'y');
  assert.deepEqual(swapped, {
    x: { source: 'y', sign: 1 },
    y: { source: 'x', sign: 1 },
    z: { source: 'z', sign: 1 },
  });
  assert.deepEqual(DEFAULT_SETTINGS.axisMapping.x, { source: 'x', sign: 1 });
  assert.equal(setAxisSign(swapped, 'z', -1).z.sign, -1);
});
```

- [ ] **Step 2: Run the tests and verify RED**

Run: `cd src-ui; pnpm test`

Expected: FAIL because `settings.js` does not exist.

- [ ] **Step 3: Implement the settings module**

Implement exact constants and validation rules:

```js
export const SETTINGS_STORAGE_KEY = 'rocket-ground-station.settings.v1';
export const BAUD_RATES = Object.freeze([9600, 19200, 38400, 57600, 115200]);
export const BODY_AXES = Object.freeze(['x', 'y', 'z']);
export const DEFAULT_SETTINGS = Object.freeze({
  version: 1,
  portPath: '',
  baudRate: 115200,
  axisMapping: Object.freeze({
    x: Object.freeze({ source: 'x', sign: 1 }),
    y: Object.freeze({ source: 'y', sign: 1 }),
    z: Object.freeze({ source: 'z', sign: 1 }),
  }),
});

const cloneDefaults = () => structuredClone(DEFAULT_SETTINGS);

export function validateSettings(value) {
  if (!value || value.version !== 1) return cloneDefaults();
  const portPath = typeof value.portPath === 'string' ? value.portPath : '';
  const baudRate = BAUD_RATES.includes(value.baudRate) ? value.baudRate : 115200;
  const mapping = value.axisMapping;
  const sources = BODY_AXES.map((axis) => mapping?.[axis]?.source);
  const signsValid = BODY_AXES.every((axis) => mapping?.[axis]?.sign === 1 || mapping?.[axis]?.sign === -1);
  const mappingValid = new Set(sources).size === 3
    && sources.every((source) => BODY_AXES.includes(source))
    && signsValid;
  return structuredClone({
    version: 1,
    portPath,
    baudRate,
    axisMapping: mappingValid ? mapping : DEFAULT_SETTINGS.axisMapping,
  });
}

export function loadSettings(storage) {
  try {
    const raw = storage?.getItem(SETTINGS_STORAGE_KEY);
    return raw ? validateSettings(JSON.parse(raw)) : cloneDefaults();
  } catch {
    return cloneDefaults();
  }
}

export function saveSettings(storage, settings) {
  const validated = validateSettings(settings);
  storage?.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(validated));
  return validated;
}

export function swapAxisSource(mapping, bodyAxis, source) {
  const next = structuredClone(mapping);
  const occupiedAxis = BODY_AXES.find((axis) => next[axis].source === source);
  const previousSource = next[bodyAxis].source;
  if (occupiedAxis && occupiedAxis !== bodyAxis) next[occupiedAxis].source = previousSource;
  next[bodyAxis].source = source;
  return next;
}

export function setAxisSign(mapping, bodyAxis, sign) {
  const next = structuredClone(mapping);
  next[bodyAxis].sign = sign === -1 ? -1 : 1;
  return next;
}
```

Add matching TypeScript interfaces to `types.ts`. Load settings once in `stores.svelte.ts` using `typeof localStorage === 'undefined' ? undefined : localStorage`; every settings update calls `saveSettings`, clones the result into reactive state, and increments `settingsRevision`. Axis source/sign/reset operations additionally increment `axisMappingRevision`; COM/Baud changes do not. `reset()` must reset telemetry runtime state but preserve persistent settings; only `resetSettings()` clears user settings.

- [ ] **Step 4: Run tests and static check**

Run: `cd src-ui; pnpm test; pnpm check`

Expected: settings tests PASS; existing attitude tests PASS; static check exits 0.

- [ ] **Step 5: Commit Task 1**

Run:

```powershell
git add src-ui/src/lib/settings.js src-ui/src/lib/settings.test.mjs src-ui/src/lib/types.ts src-ui/src/lib/stores.svelte.ts
git commit -m "feat: persist ground station settings"
```

### Task 2: Packet-Driven MPU6050 Six-Axis Attitude Estimator

**Files:**
- Replace: `src-ui/src/lib/attitude.js`
- Replace: `src-ui/src/lib/attitude.test.mjs`
- Modify: `src-ui/src/components/AttitudeIndicator.svelte`

**Interfaces:**
- Consumes: validated `AxisMapping`, telemetry acceleration in `m/s²`, angular velocity in `deg/s`, and monotonic packet timestamps in milliseconds.
- Produces: `mapSensorVector(vector, mapping)`, `integrateGyroDegrees(current, rates, dt)`, `createAttitudeEstimator(options?)`, estimator methods `update(sample, timestampMs, mapping)`, `reset()`, `getAttitude()`.

- [ ] **Step 1: Replace attitude tests with desired behavior**

Test direct degree integration, mapping, gravity correction, acceleration gating, timestamp gaps, normalization, and invalid values:

```js
import assert from 'node:assert/strict';
import test from 'node:test';
import { createAttitudeEstimator, integrateGyroDegrees, mapSensorVector } from './attitude.js';

const identity = {
  x: { source: 'x', sign: 1 },
  y: { source: 'y', sign: 1 },
  z: { source: 'z', sign: 1 },
};
const sample = (gyro, accel = { x: 0, y: 0, z: 30 }) => ({ gyro, accel });

test('maps axes and signs for gyro and acceleration vectors', () => {
  const mapping = {
    x: { source: 'z', sign: -1 },
    y: { source: 'x', sign: 1 },
    z: { source: 'y', sign: -1 },
  };
  assert.deepEqual(mapSensorVector({ x: 1, y: 2, z: 3 }, mapping), { x: -3, y: 1, z: -2 });
});

test('integrates degrees per second without radian conversion', () => {
  assert.deepEqual(
    integrateGyroDegrees(
      { roll: 0, pitch: 0, yaw: 0 },
      { x: 90, y: 45, z: 180 },
      1,
    ),
    { roll: 90, pitch: 45, yaw: 180 },
  );
});

test('updates the estimator once per packet interval', () => {
  const estimator = createAttitudeEstimator();
  estimator.update(sample({ x: 90, y: 45, z: 180 }), 1000, identity);
  const result = estimator.update(sample({ x: 90, y: 45, z: 180 }), 1100, identity);
  assert.deepEqual(result, { roll: 9, pitch: 4.5, yaw: 18 });
});

test('uses gravity correction near 1g but rejects powered-flight acceleration', () => {
  const estimator = createAttitudeEstimator({ initialAttitude: { roll: 30, pitch: 20, yaw: 0 } });
  estimator.update(sample({ x: 0, y: 0, z: 0 }, { x: 0, y: 0, z: 9.80665 }), 1000, identity);
  const corrected = estimator.update(sample({ x: 0, y: 0, z: 0 }, { x: 0, y: 0, z: 9.80665 }), 1100, identity);
  assert.ok(Math.abs(corrected.roll) < 30);
  assert.ok(Math.abs(corrected.pitch) < 20);
  const beforeBoost = estimator.getAttitude();
  const boosted = estimator.update(sample({ x: 0, y: 0, z: 0 }, { x: 30, y: 0, z: 30 }), 1200, identity);
  assert.deepEqual(boosted, beforeBoost);
});

test('does not integrate across gaps over 250 ms and normalizes yaw', () => {
  const estimator = createAttitudeEstimator();
  estimator.update(sample({ x: 0, y: 0, z: 200 }), 1000, identity);
  assert.equal(estimator.update(sample({ x: 0, y: 0, z: 200 }), 1400, identity).yaw, 0);
  assert.equal(estimator.update(sample({ x: 0, y: 0, z: 200 }), 1500, identity).yaw, 20);
});

test('ignores non-finite gyro updates', () => {
  const estimator = createAttitudeEstimator();
  estimator.update(sample({ x: 0, y: 0, z: 0 }), 1000, identity);
  assert.deepEqual(estimator.update(sample({ x: NaN, y: 0, z: 0 }), 1100, identity), { roll: 0, pitch: 0, yaw: 0 });
});
```

- [ ] **Step 2: Run tests and verify RED**

Run: `cd src-ui; pnpm test`

Expected: FAIL because current code converts radians and has no estimator/mapping API.

- [ ] **Step 3: Implement the estimator**

Use `G = 9.80665`, gate `0.85 * G..1.15 * G`, gap `250 ms`, `tau = 0.5`, and shortest-angle blending. Do not export or retain the old radian conversion functions. A first packet or long gap only sets `lastTimestampMs`. `reset()` sets `{roll:0,pitch:0,yaw:0}` and clears the timestamp.

In `AttitudeIndicator.svelte`, create one estimator outside effects. The telemetry effect reads `store.telemetryRevision`, the current telemetry object, and current mapping, then calls `estimator.update(..., performance.now(), mapping)` exactly once. It writes returned display state but never reads display Roll/Pitch/Yaw inside the effect. Detect `store.axisMappingRevision` changes, reset the estimator, and zero display values before processing the next packet. COM/Baud changes must not reset attitude. Display mapped gyro values directly as `deg/s`. Add a Chinese `歸零` button and rename heading to `相對航向`.

- [ ] **Step 4: Run tests and checks**

Run: `cd src-ui; pnpm test; pnpm check`

Expected: all Node tests PASS; static check exits 0 without reactive-loop warnings.

- [ ] **Step 5: Commit Task 2**

```powershell
git add src-ui/src/lib/attitude.js src-ui/src/lib/attitude.test.mjs src-ui/src/components/AttitudeIndicator.svelte
git commit -m "fix: fuse MPU6050 attitude per telemetry packet"
```

### Task 3: Telemetry Revision and Persistent Serial/Axis UI

**Files:**
- Modify: `src-ui/src/lib/stores.svelte.ts`
- Modify: `src-ui/src/lib/settings.test.mjs`
- Modify: `src-ui/src/lib/tauri.ts`
- Modify: `src-ui/src/components/ConnectionPanel.svelte`
- Modify: `src-tauri/src/commands/serial.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: settings store methods from Task 1.
- Produces: `telemetryRevision`, incremented once for every `updateTelemetry` call even when payload values are identical.
- Produces: Tauri command and frontend wrapper `listSerialPorts(): Promise<string[]>`.

- [ ] **Step 1: Add an exact source-level revision regression test**

Append this test to `settings.test.mjs`; it guards the packet identity contract without executing Svelte runes in Node:

```js
import { readFileSync } from 'node:fs';

test('increments telemetry revision for every store update', () => {
  const source = readFileSync(new URL('./stores.svelte.ts', import.meta.url), 'utf8');
  const updateBody = source.match(/updateTelemetry\(payload: TelemetryPayload\) \{([\s\S]*?)\n    \},/u)?.[1] ?? '';
  assert.match(updateBody, /telemetryRevision \+= 1/u);
});
```

- [ ] **Step 2: Run test and verify RED**

Run: `cd src-ui; pnpm test`

Expected: FAIL because `telemetryRevision` is not yet incremented.

- [ ] **Step 3: Implement revision and UI persistence**

Initialize `telemetryRevision = $state(0)`, expose its getter, and increment after every valid `updateTelemetry` assignment. Reset it to 0 only in runtime `reset()`.

Add this Rust command and register it in `generate_handler!`:

```rust
#[tauri::command]
pub async fn list_serial_ports() -> InvokeResult<Vec<String>> {
    let mut ports = tokio_serial::available_ports()
        .map_err(|error| InvokeError::SerialError(error.to_string()))?
        .into_iter()
        .map(|port| port.port_name)
        .collect::<Vec<_>>();
    ports.sort();
    Ok(ports)
}
```

Add `listSerialPorts()` to `tauri.ts` using `invoke<string[]>('list_serial_ports')`.

In `ConnectionPanel.svelte`:

- Initialize local port and baud from `store.settings`.
- Load available ports on mount into a datalist without replacing the persisted value.
- If the persisted COM value is non-empty and absent from the available list, show `已保存的 COM Port 目前不可用，請確認裝置連線` while retaining the value.
- Persist trimmed COM Port on input blur and before connecting.
- Persist Baud Rate on change.
- Use `BAUD_RATES` from `settings.js`.
- Add a collapsible `姿態軸向設定` section with rows `火箭 X（Roll）`, `火箭 Y（Pitch）`, `火箭 Z（Yaw）`.
- Each row has a source select and `正向/反向` sign button.
- Calling `store.updateAxisSource` uses swap semantics, never duplicate sources.
- Add `恢復預設軸向` and `恢復所有設定`; require a second click within 3 seconds for the latter, avoiding a browser modal in Tauri.
- Translate errors and buttons: `請輸入 COM Port`, `連線中…`, `中斷連線`, `開始監控`, `序列連線`, `封包格式`.
- Do not auto-connect after settings load.

- [ ] **Step 4: Run tests and checks**

Run: `cd src-ui; pnpm test; pnpm check; cd ../src-tauri; cargo test`

Expected: all tests PASS; Svelte/TypeScript check exits 0.

- [ ] **Step 5: Commit Task 3**

```powershell
git add src-ui/src/lib/stores.svelte.ts src-ui/src/lib/settings.test.mjs src-ui/src/lib/tauri.ts src-ui/src/components/ConnectionPanel.svelte src-tauri/src/commands/serial.rs src-tauri/src/lib.rs
git commit -m "feat: add persistent serial and axis controls"
```

### Task 4: Pure GPS Validation and Track Processing

**Files:**
- Create: `src-ui/src/lib/gps-map.js`
- Create: `src-ui/src/lib/gps-map.test.mjs`

**Interfaces:**
- Produces: `isValidGpsPosition(position)`, `haversineDistanceMeters(a,b)`, `appendTrackPoint(points, position, options?)`.
- Position shape: `{ lat: number, lng: number }`.

- [ ] **Step 1: Write failing GPS tests**

```js
import assert from 'node:assert/strict';
import test from 'node:test';
import { appendTrackPoint, haversineDistanceMeters, isValidGpsPosition } from './gps-map.js';

test('validates finite coordinates and treats zero-zero as no fix', () => {
  assert.equal(isValidGpsPosition({ lat: 25.033, lng: 121.5654 }), true);
  assert.equal(isValidGpsPosition({ lat: 0, lng: 0 }), false);
  assert.equal(isValidGpsPosition({ lat: 91, lng: 121 }), false);
  assert.equal(isValidGpsPosition({ lat: NaN, lng: 121 }), false);
});

test('computes useful Haversine distance', () => {
  const meters = haversineDistanceMeters(
    { lat: 25.033, lng: 121.5654 },
    { lat: 25.03309, lng: 121.5654 },
  );
  assert.ok(meters > 9 && meters < 11);
});

test('filters sub-two-meter jitter and keeps at most 5000 points', () => {
  const first = { lat: 25.033, lng: 121.5654 };
  assert.deepEqual(appendTrackPoint([first], { lat: 25.033001, lng: 121.5654 }), [first]);
  const points = Array.from({ length: 5000 }, (_, index) => ({ lat: 20 + index * 0.0001, lng: 121 }));
  const next = appendTrackPoint(points, { lat: 25.1, lng: 121 });
  assert.equal(next.length, 5000);
  assert.deepEqual(next.at(-1), { lat: 25.1, lng: 121 });
});
```

- [ ] **Step 2: Run tests and verify RED**

Run: `cd src-ui; pnpm test`

Expected: FAIL because `gps-map.js` does not exist.

- [ ] **Step 3: Implement minimal GPS utilities**

Use Earth radius `6_371_000`, default minimum distance `2`, and default maximum points `5_000`. Return the existing array unchanged for invalid or too-close points; return a new sliced array only when appending.

- [ ] **Step 4: Run tests**

Run: `cd src-ui; pnpm test`

Expected: all GPS, settings, and attitude tests PASS.

- [ ] **Step 5: Commit Task 4**

```powershell
git add src-ui/src/lib/gps-map.js src-ui/src/lib/gps-map.test.mjs
git commit -m "feat: validate GPS tracks"
```

### Task 5: Leaflet/OpenStreetMap Live GPS Component

**Files:**
- Create: `src-ui/src/components/GpsMap.svelte`
- Modify: `src-ui/src/main.ts`
- Modify: `src-ui/package.json`
- Create: `src-ui/pnpm-lock.yaml`
- Modify: `src-tauri/tauri.conf.json`

**Interfaces:**
- Consumes: `store.telemetry`, `store.telemetryRevision`, and GPS helpers from Task 4.
- Owns: Leaflet map, tile layer, one marker, one polyline, resize observer, and event cleanup.

- [ ] **Step 1: Install Leaflet dependencies**

Run: `cd src-ui; pnpm add leaflet; pnpm add -D @types/leaflet`

Expected: `package.json` contains `leaflet`; `devDependencies` contains `@types/leaflet`; `pnpm-lock.yaml` is created.

- [ ] **Step 2: Add Leaflet CSS and a component shell**

Add `import 'leaflet/dist/leaflet.css';` to `main.ts`. Create `GpsMap.svelte` with a bound container, a 280 px map body, Chinese status row, and buttons `自動跟隨`, `定位火箭`, `清除軌跡`. Use a `divIcon` containing a cyan rocket glyph so no external marker PNG path is required.

- [ ] **Step 3: Implement map lifecycle**

On mount:

```ts
map = L.map(container, { zoomControl: true }).setView([23.7, 121.0], 7);
tileLayer = L.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', {
  maxZoom: 19,
  attribution: '&copy; OpenStreetMap contributors',
}).addTo(map);
trackLine = L.polyline([], { color: '#00d4ff', weight: 3 }).addTo(map);
map.on('dragstart', disableFollowing);
resizeObserver = new ResizeObserver(() => map?.invalidateSize());
resizeObserver.observe(container);
```

The telemetry effect depends on `store.telemetryRevision`, validates `{lat: latitude, lng: longitude}`, moves or creates one marker, calls `appendTrackPoint`, updates the polyline only when the track array changes, and centers with `map.setView(position, firstFix ? 16 : map.getZoom(), { animate: false })` only while following. Tile errors set the Chinese map-error banner but never mutate telemetry or connection state.

Cleanup removes listeners, disconnects the observer, calls `map.remove()`, and clears references. `定位火箭` re-enables follow and centers the last valid fix. `清除軌跡` clears the array and polyline but preserves marker.

- [ ] **Step 4: Add Tauri CSP**

Replace `csp: null` with a Tauri-compatible policy that keeps IPC and local assets working while allowing only OSM tiles externally:

```json
"csp": "default-src 'self'; connect-src ipc: http://ipc.localhost https://tile.openstreetmap.org; img-src 'self' asset: https://asset.localhost https://tile.openstreetmap.org data: blob:; style-src 'self' 'unsafe-inline'; font-src 'self' data:"
```

If Tauri dev mode reports CSP violations for the Vite WebSocket, add a `devCsp` containing `ws://localhost:8000` without widening production CSP.

- [ ] **Step 5: Run tests, check, and build**

Run: `cd src-ui; pnpm test; pnpm check; pnpm build`

Expected: all commands exit 0; build output includes Leaflet CSS/assets without unresolved marker images.

- [ ] **Step 6: Commit Task 5**

```powershell
git add src-ui/src/components/GpsMap.svelte src-ui/src/main.ts src-ui/package.json src-ui/pnpm-lock.yaml src-tauri/tauri.conf.json
git commit -m "feat: add live OpenStreetMap GPS tracking"
```

### Task 6: Right-Side Layout and Traditional Chinese UI

**Files:**
- Modify: `src-ui/src/App.svelte`
- Modify: `src-ui/src/app.css`
- Modify: `src-ui/src/components/TelemetryGrid.svelte`
- Modify: `src-ui/src/components/TelemetryCharts.svelte`
- Modify: `src-ui/src/components/StatusBar.svelte`
- Modify: `src-ui/src/components/AttitudeIndicator.svelte`
- Modify: `src-ui/src/components/ConnectionPanel.svelte`

**Interfaces:**
- Consumes: `GpsMap` and all prior store interfaces.
- Produces: desktop right-upper map, right-lower attitude, and one-column narrow layout.

- [ ] **Step 1: Add source-level localization regression test**

Create `src-ui/src/lib/ui-copy.test.mjs` that reads the five UI files and asserts the old user-facing strings are absent and required Chinese strings are present:

```js
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path) => readFileSync(new URL(path, import.meta.url), 'utf8');

test('monitoring UI uses Traditional Chinese labels', () => {
  const text = [
    read('../App.svelte'),
    read('../components/TelemetryGrid.svelte'),
    read('../components/TelemetryCharts.svelte'),
    read('../components/StatusBar.svelte'),
    read('../components/ConnectionPanel.svelte'),
  ].join('\n');
  for (const oldLabel of ['Ground Speed', 'Vertical Velocity', 'Waiting for telemetry', 'Start Monitoring', 'Disconnect']) {
    assert.equal(text.includes(oldLabel), false, oldLabel);
  }
  for (const label of ['地面速度', '垂直速度', '等待遙測資料', '開始監控', '中斷連線']) {
    assert.equal(text.includes(label), true, label);
  }
});
```

- [ ] **Step 2: Run test and verify RED**

Run: `cd src-ui; pnpm test`

Expected: localization test FAIL on existing English strings.

- [ ] **Step 3: Localize visible UI**

Translate:

- App header: `火箭地面站監控`, `相對高度`, `封包`, `已連線/未連線`.
- Telemetry categories: `IMU 感測器`, `飛行／定位`, `環境感測`.
- Fields: `X/Y/Z 軸加速度`, `X/Y/Z 軸角速度`, `經度`, `緯度`, `相對高度`, `地面速度`, `垂直速度`, `氣壓`, `溫度`.
- Charts: `相對高度`, `垂直速度`, `等待遙測資料…`.
- Status: `接收中/待命`, `封包`, `CRC 失敗`, `頻率`, `運行時間`.
- Attitude: `火箭姿態`, `姿態`, `相對航向`, `角速度`, `俯仰`, `滾轉`, `偏航`, `歸零`.

Do not translate engineering units or COM/Baud/GPS/IMU/CRC abbreviations.

- [ ] **Step 4: Implement responsive layout**

Import `GpsMap` in `App.svelte` and render it before `AttitudeIndicator` inside `.sidebar-right`. Keep desktop columns `260px minmax(0,1fr) 300px`. At `max-width: 1180px`, use CSS grid areas so DOM order renders connection, telemetry, GPS map, then attitude. Ensure `.sidebar-right` scrolls on desktop and the map component itself remains 280 px high. Let `GpsMap`'s ResizeObserver call `invalidateSize()` after responsive changes.

- [ ] **Step 5: Run tests, check, and build**

Run: `cd src-ui; pnpm test; pnpm check; pnpm build`

Expected: localization tests PASS; all checks and build exit 0.

- [ ] **Step 6: Commit Task 6**

```powershell
git add src-ui/src/App.svelte src-ui/src/app.css src-ui/src/components src-ui/src/lib/ui-copy.test.mjs
git commit -m "feat: localize monitoring layout"
```

### Task 7: Documentation and End-to-End Verification

**Files:**
- Modify: `README.md`

**Interfaces:**
- Documents the final telemetry contract, controls, limitations, network use, and field calibration procedure.

- [ ] **Step 1: Update README**

Document:

- Angular velocity packet unit is `deg/s`.
- Roll/Pitch use gated MPU6050 accelerometer correction; Yaw is relative and drifts without a magnetometer.
- Axis mapping, COM, and Baud are adjustable and persistent in UI; startup never auto-connects.
- NEO-6M map uses Leaflet/OpenStreetMap, requires internet, shows live marker and 5,000-point track, and does not offer offline tile downloads.
- OSM attribution remains visible.
- Calibration procedure: zero while stationary, rotate one physical axis at a time, then swap/invert axes in UI.

- [ ] **Step 2: Run complete verification**

Run:

```powershell
Set-Location src-ui
pnpm test
pnpm check
pnpm build
Set-Location ..
git diff --check
git status --short
```

Expected:

- Node tests report zero failures.
- Svelte/TypeScript check exits 0.
- Vite build exits 0 and creates `src-ui/dist`.
- `git diff --check` emits no errors.
- `git status --short` shows only the intended README change before the final commit.

- [ ] **Step 3: Manual Tauri smoke test with simulated or live telemetry**

Run: `cd src-ui; pnpm dev` and launch the Tauri development app from another terminal if the Rust toolchain is available.

Verify:

1. Saved COM/Baud/axis settings survive restart; no automatic connection occurs.
2. Two identical telemetry packets increment revision and update attitude once each.
3. `90 deg/s` physical/simulated rotation is not amplified by 57.3.
4. Axis swaps and sign changes immediately zero and remap attitude.
5. Valid NEO-6M coordinates update one marker; manual map drag disables follow; locating rocket restores it.
6. Map is upper-right at desktop size and above attitude on narrow layout.
7. Disconnecting network leaves GPS numbers and other telemetry functional with a Chinese map error.
8. No old English monitoring labels remain.

- [ ] **Step 4: Commit documentation**

```powershell
git add README.md
git commit -m "docs: document attitude and GPS monitoring"
```

- [ ] **Step 5: Final clean verification**

Run: `git status --short; git log -8 --oneline`

Expected: clean working tree and one focused commit per task.
