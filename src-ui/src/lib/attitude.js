const GRAVITY_METERS_PER_SECOND_SQUARED = 9.80665;
const ACCEL_MIN = GRAVITY_METERS_PER_SECOND_SQUARED * 0.85;
const ACCEL_MAX = GRAVITY_METERS_PER_SECOND_SQUARED * 1.15;
const FILTER_TIME_CONSTANT_SECONDS = 0.5;
// Telemetry normally arrives around every 500 ms and can occasionally take 1000 ms.
// Multi-second gaps still reset the time baseline instead of integrating downtime.
const MAX_PACKET_GAP_MS = 1500;

/** @typedef {'x' | 'y' | 'z'} SensorAxis */
/** @typedef {{ x: number, y: number, z: number }} Vector3 */
/** @typedef {{ roll: number, pitch: number, yaw: number }} Attitude */
/** @typedef {{ source: SensorAxis, sign: 1 | -1 }} AxisRule */
/** @typedef {{ x: AxisRule, y: AxisRule, z: AxisRule }} AxisMapping */
/** @typedef {{ gyro: Vector3, accel: Vector3 }} ImuSample */

/** @param {number} value @param {number} min @param {number} max */
function clamp(value, min, max) {
  return Math.max(min, Math.min(max, value));
}

/** @param {number} angle */
function normalizeSignedDegrees(angle) {
  return ((angle + 180) % 360 + 360) % 360 - 180;
}

/** @param {number} angle */
function normalizeCompassDegrees(angle) {
  return ((angle % 360) + 360) % 360;
}

/** @param {Vector3} vector */
function isFiniteVector(vector) {
  return Number.isFinite(vector.x) && Number.isFinite(vector.y) && Number.isFinite(vector.z);
}

/**
 * @param {Vector3} vector
 * @param {AxisMapping} mapping
 * @returns {Vector3}
 */
export function mapSensorVector(vector, mapping) {
  return {
    x: vector[mapping.x.source] * mapping.x.sign,
    y: vector[mapping.y.source] * mapping.y.sign,
    z: vector[mapping.z.source] * mapping.z.sign,
  };
}

/**
 * @param {Attitude} current
 * @param {Vector3} degreesPerSecond
 * @param {number} deltaSeconds
 * @returns {Attitude}
 */
export function integrateGyroDegrees(current, degreesPerSecond, deltaSeconds) {
  const dt = Math.max(0, deltaSeconds);
  return {
    roll: normalizeSignedDegrees(current.roll + degreesPerSecond.x * dt),
    pitch: clamp(current.pitch + degreesPerSecond.y * dt, -90, 90),
    yaw: normalizeCompassDegrees(current.yaw + degreesPerSecond.z * dt),
  };
}

/** @param {number} predicted @param {number} observed @param {number} alpha */
function blendSignedAngle(predicted, observed, alpha) {
  const correction = normalizeSignedDegrees(observed - predicted);
  return normalizeSignedDegrees(predicted + correction * (1 - alpha));
}

/** @param {Vector3} acceleration */
function accelerationAngles(acceleration) {
  const roll = Math.atan2(acceleration.y, acceleration.z) * 180 / Math.PI;
  const pitch = Math.atan2(
    -acceleration.x,
    Math.hypot(acceleration.y, acceleration.z),
  ) * 180 / Math.PI;
  return { roll, pitch };
}

/** @param {Vector3} acceleration */
function canUseAcceleration(acceleration) {
  if (!isFiniteVector(acceleration)) return false;
  const magnitude = Math.hypot(acceleration.x, acceleration.y, acceleration.z);
  return magnitude >= ACCEL_MIN && magnitude <= ACCEL_MAX;
}

/**
 * @param {{ initialAttitude?: Attitude }} [options]
 */
export function createAttitudeEstimator(options = {}) {
  /** @type {Attitude} */
  let attitude = structuredClone(options.initialAttitude ?? { roll: 0, pitch: 0, yaw: 0 });
  /** @type {number | null} */
  let lastTimestampMs = null;

  return {
    /**
     * @param {ImuSample} sample
     * @param {number} timestampMs
     * @param {AxisMapping} mapping
     * @returns {Attitude}
     */
    update(sample, timestampMs, mapping) {
      if (!Number.isFinite(timestampMs)) return structuredClone(attitude);

      const gyro = mapSensorVector(sample.gyro, mapping);
      if (!isFiniteVector(gyro)) return structuredClone(attitude);

      if (lastTimestampMs === null) {
        lastTimestampMs = timestampMs;
        return structuredClone(attitude);
      }

      const gapMs = timestampMs - lastTimestampMs;
      lastTimestampMs = timestampMs;
      if (gapMs <= 0 || gapMs > MAX_PACKET_GAP_MS) return structuredClone(attitude);

      const dt = gapMs / 1000;
      const predicted = integrateGyroDegrees(attitude, gyro, dt);
      const acceleration = mapSensorVector(sample.accel, mapping);

      if (canUseAcceleration(acceleration)) {
        const observed = accelerationAngles(acceleration);
        const alpha = FILTER_TIME_CONSTANT_SECONDS / (FILTER_TIME_CONSTANT_SECONDS + dt);
        attitude = {
          roll: blendSignedAngle(predicted.roll, observed.roll, alpha),
          pitch: clamp(blendSignedAngle(predicted.pitch, observed.pitch, alpha), -90, 90),
          yaw: predicted.yaw,
        };
      } else {
        attitude = predicted;
      }

      return structuredClone(attitude);
    },

    /** @returns {Attitude} */
    getAttitude() {
      return structuredClone(attitude);
    },

    /** @returns {Attitude} */
    reset() {
      attitude = { roll: 0, pitch: 0, yaw: 0 };
      lastTimestampMs = null;
      return structuredClone(attitude);
    },
  };
}
