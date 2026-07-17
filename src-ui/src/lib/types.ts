export interface TelemetryPayload {
  protocolVersion: number;
  sessionId: number;
  frameSeq: number;
  uptimeMs: number;
  restartReason: number;
  timerState: number;
  deployState: number;
  sensorFlags: number;
  remainingS: number;
  lastAckCommandId: number;
  lastAckResult: number;
  xAcceleration: number;
  yAcceleration: number;
  zAcceleration: number;
  xAngularVelocity: number;
  yAngularVelocity: number;
  zAngularVelocity: number;
  longitude: number;
  latitude: number;
  altitude: number;
  groundSpeed: number;
  verticalVelocity: number;
  airPressure: number;
  temperature: number;
}

export interface AirborneSessionChange {
  previousSessionId: number | null;
  sessionId: number;
  restartReason: number;
}

export interface CommandStatus {
  commandId: number | null;
  commandType: string;
  status: 'waiting_session' | 'queued' | 'sending' | 'acked' | 'failed' | 'ignored_ack' | 'cancelled';
  attempts: number;
  result: number | null;
  detail: string;
}

export interface FlightStats {
  telemetryPackets: number;
  expectedPackets: number;
  lostPackets: number;
  duplicatePackets: number;
  crcErrors: number;
  linkOutages: number;
  maxLinkLossMs: number;
  restartCount: number;
}

export interface FlightSessionMetadata {
  initialBatteryVoltage: number;
  location: string;
  operator: string;
  notes: string;
}

export type SensorAxis = 'x' | 'y' | 'z';
export type AxisSign = 1 | -1;

export interface AxisRule {
  source: SensorAxis;
  sign: AxisSign;
}

export interface AxisMapping {
  x: AxisRule;
  y: AxisRule;
  z: AxisRule;
}

export interface AppSettings {
  version: 1;
  portPath: string;
  baudRate: number;
  axisMapping: AxisMapping;
}

export interface PacketStats {
  totalPackets: number;
  failedPackets: number;
  packetsPerSecond: number;
}

export interface SerialError {
  errorType: string;
  detail: string;
}

export interface DbTelemetry {
  id: number;
  receivedAt: string;
  xAcceleration: number;
  yAcceleration: number;
  zAcceleration: number;
  xAngularVelocity: number;
  yAngularVelocity: number;
  zAngularVelocity: number;
  longitude: number;
  latitude: number;
  altitude: number;
  groundSpeed: number;
  verticalVelocity: number;
  airPressure: number;
  temperature: number;
}
