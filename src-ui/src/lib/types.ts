export interface TelemetryPayload {
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

export interface DbTelemetry extends TelemetryPayload {
  id: number;
  receivedAt: string;
}
