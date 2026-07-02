import assert from 'node:assert/strict';
import test from 'node:test';
import { integrateMpu6050Attitude, radPerSecondToDegreesPerSecond } from './attitude.js';

test('converts MPU6050 gyro radians per second to degrees per second', () => {
  assert.equal(radPerSecondToDegreesPerSecond(Math.PI), 180);
});

test('integrates MPU6050 gyro rates into roll, pitch, and yaw angles', () => {
  const next = integrateMpu6050Attitude(
    { roll: 0, pitch: 0, yaw: 0 },
    { x: Math.PI / 2, y: Math.PI / 4, z: Math.PI },
    1,
  );

  assert.equal(next.roll, 90);
  assert.equal(next.pitch, 45);
  assert.equal(next.yaw, 180);
});

test('normalizes wrapped attitude angles for display', () => {
  const next = integrateMpu6050Attitude(
    { roll: 170, pitch: 85, yaw: 350 },
    { x: Math.PI, y: Math.PI, z: Math.PI },
    1,
  );

  assert.equal(next.roll, -10);
  assert.equal(next.pitch, 90);
  assert.equal(next.yaw, 170);
});
