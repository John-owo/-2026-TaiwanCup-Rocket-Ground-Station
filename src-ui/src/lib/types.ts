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
