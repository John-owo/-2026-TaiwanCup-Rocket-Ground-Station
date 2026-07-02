const RAD_TO_DEG = 180 / Math.PI;

/**
 * @typedef {{ roll: number, pitch: number, yaw: number }} Attitude
 * @typedef {{ x: number, y: number, z: number }} GyroRates
 */

/**
 * MPU6050 gyro events from Adafruit_MPU6050 are radians per second.
 *
 * @param {number} radiansPerSecond
 * @returns {number}
 */
export function radPerSecondToDegreesPerSecond(radiansPerSecond) {
  return radiansPerSecond * RAD_TO_DEG;
}

/**
 * @param {number} value
 * @param {number} min
 * @param {number} max
 * @returns {number}
 */
function clamp(value, min, max) {
  return Math.max(min, Math.min(max, value));
}

/**
 * @param {number} angle
 * @returns {number}
 */
function normalizeSignedDegrees(angle) {
  return ((angle + 180) % 360 + 360) % 360 - 180;
}

/**
 * @param {number} angle
 * @returns {number}
 */
function normalizeCompassDegrees(angle) {
  return ((angle % 360) + 360) % 360;
}

/**
 * Integrates MPU6050 gyro rates into a display attitude.
 * This is gyro-only attitude estimation, so long flights will still drift.
 *
 * @param {Attitude} current
 * @param {GyroRates} gyroRadPerSecond
 * @param {number} deltaSeconds
 * @returns {Attitude}
 */
export function integrateMpu6050Attitude(current, gyroRadPerSecond, deltaSeconds) {
  const dt = Math.max(0, deltaSeconds);
  const rollRate = radPerSecondToDegreesPerSecond(gyroRadPerSecond.x);
  const pitchRate = radPerSecondToDegreesPerSecond(gyroRadPerSecond.y);
  const yawRate = radPerSecondToDegreesPerSecond(gyroRadPerSecond.z);

  return {
    roll: normalizeSignedDegrees(current.roll + rollRate * dt),
    pitch: clamp(current.pitch + pitchRate * dt, -90, 90),
    yaw: normalizeCompassDegrees(current.yaw + yawRate * dt),
  };
}

/**
 * @param {GyroRates} gyroRadPerSecond
 * @returns {GyroRates}
 */
export function mpu6050GyroRatesToDegrees(gyroRadPerSecond) {
  return {
    x: radPerSecondToDegreesPerSecond(gyroRadPerSecond.x),
    y: radPerSecondToDegreesPerSecond(gyroRadPerSecond.y),
    z: radPerSecondToDegreesPerSecond(gyroRadPerSecond.z),
  };
}
