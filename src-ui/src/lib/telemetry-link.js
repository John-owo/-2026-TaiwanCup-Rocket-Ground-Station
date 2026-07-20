export const TELEMETRY_LINK_LOSS_THRESHOLD_MS = 4_500;

/** @typedef {'standby' | 'waiting' | 'live' | 'lost'} TelemetryLinkState */

/**
 * @param {boolean} connected
 * @param {number | null} lastPacketAt
 * @param {number} nowMs
 * @returns {TelemetryLinkState}
 */
export function getTelemetryLinkState(connected, lastPacketAt, nowMs) {
  if (!connected) return 'standby';
  if (lastPacketAt === null || !Number.isFinite(lastPacketAt)) return 'waiting';
  const ageMs = Math.max(0, nowMs - lastPacketAt);
  return ageMs <= TELEMETRY_LINK_LOSS_THRESHOLD_MS ? 'live' : 'lost';
}
