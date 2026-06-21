use crate::infrastructures::serial::crc::crc16_ccitt;
use crate::models::response::TelemetryPayload;
use crate::services::serial::{Decoder, Parser, Verificator};

/// 封包位元組序
#[allow(dead_code)]
pub enum PacketOrder {
    BigEndian,
    LittleEndian,
}

/// 解析器狀態機
pub enum ParseState {
    /// 等待 0xAA header byte
    Header,
    /// 蒐集 payload bytes
    Payload,
    /// 蒐集 CRC bytes (目前已蒐集的 byte 數)
    CrcHigh,
    CrcLow(u8),
}

/// 封包解析結果
pub enum ParseResult {
    /// 還在蒐集中
    Incomplete,
    /// 成功解出一筆遙測資料
    Complete(TelemetryPayload),
    /// 解析或驗證失敗
    ParseError(String),
}

/// 封包解析器
/// 狀態機 flow: Header(0xAA) → Payload(N bytes) → CRC(2 bytes) → 驗證 → 輸出
pub struct PacketParser {
    pub state: ParseState,
    pub buffer: Vec<u8>,
    pub expect_packet_length: usize,
    pub verificator: Box<dyn Verificator<VerificationType = u16> + Send + Sync>,
    pub decoder: Box<dyn Decoder<ResultType = TelemetryPayload> + Send + Sync>,
}

// ─── Trait Implementation ────────────────────────────────────────────────────

impl Parser for PacketParser {
    fn new(
        expect_packet_length: usize,
        verificator: Box<dyn Verificator<VerificationType = u16> + Send + Sync>,
        decoder: Box<dyn Decoder<ResultType = TelemetryPayload> + Send + Sync>,
    ) -> Self {
        Self {
            state: ParseState::Header,
            buffer: Vec::with_capacity(expect_packet_length),
            expect_packet_length,
            verificator,
            decoder,
        }
    }

    fn default() -> Self {
        // 預設 52 bytes payload (13 x f32)
        Self {
            state: ParseState::Header,
            buffer: Vec::with_capacity(52),
            expect_packet_length: 52,
            verificator: Box::new(PacketVerificator::new()),
            decoder: Box::new(TelemetryDecoder),
        }
    }

    fn sink(&mut self, byte: u8) -> ParseResult {
        match self.state {
            ParseState::Header => {
                if byte == 0xAA {
                    self.buffer.clear();
                    self.state = ParseState::Payload;
                }
                ParseResult::Incomplete
            }
            ParseState::Payload => {
                self.buffer.push(byte);

                if self.buffer.len() >= self.expect_packet_length {
                    // payload 蒐集完畢，先計算本地 CRC 並設定到 verificator 中
                    let computed_crc = crc16_ccitt(&self.buffer);
                    let _ = self.verificator.set_verification_field(computed_crc);
                    self.state = ParseState::CrcHigh;
                }
                ParseResult::Incomplete
            }
            ParseState::CrcHigh => {
                // 蒐集 CRC 的高位 byte
                self.state = ParseState::CrcLow(byte);
                ParseResult::Incomplete
            }
            ParseState::CrcLow(high_byte) => {
                // 組合 CRC: big-endian (high << 8 | low)
                let received_crc = (high_byte as u16) << 8 | (byte as u16);

                // 重設狀態，準備接收下一個封包
                self.state = ParseState::Header;

                // 使用 verificator 驗證 CRC
                match self.verificator.verify(received_crc) {
                    Ok(true) => {
                        // CRC 驗證通過，解碼 payload
                        match self.decoder.decode(&self.buffer) {
                            Ok(payload) => ParseResult::Complete(payload),
                            Err(e) => ParseResult::ParseError(format!("decode error: {}", e)),
                        }
                    }
                    Ok(false) => {
                        ParseResult::ParseError("CRC verification failed".to_string())
                    }
                    Err(e) => ParseResult::ParseError(e),
                }
            }
        }
    }

    fn parse_to_payload(&self, buffer: &[u8]) -> Result<TelemetryPayload, String> {
        self.decoder.decode(buffer)
    }
}

// ─── CRC Verificator ─────────────────────────────────────────────────────────

/// CRC-16/CCITT-False 驗證器
/// 先透過 set_verification_field 設定從 payload 計算出的 CRC，
/// 再透過 verify 比對接收到的 CRC
pub struct PacketVerificator {
    computed_crc: Option<u16>,
}

impl PacketVerificator {
    pub fn new() -> Self {
        Self { computed_crc: None }
    }
}

impl Verificator for PacketVerificator {
    type VerificationType = u16;

    fn verify(&self, received_crc: Self::VerificationType) -> Result<bool, String> {
        match self.computed_crc {
            Some(computed) => Ok(computed == received_crc),
            None => Err("internal CRC not computed yet".to_string()),
        }
    }

    fn set_verification_field(
        &mut self,
        computed_crc: Self::VerificationType,
    ) -> Result<(), ()> {
        self.computed_crc = Some(computed_crc);
        Ok(())
    }
}

// ─── Telemetry Decoder ───────────────────────────────────────────────────────

/// Big-Endian f32 解碼器
/// 將 52 bytes buffer 解碼成 TelemetryPayload (13 x f32)
pub struct TelemetryDecoder;

impl Decoder for TelemetryDecoder {
    type ResultType = TelemetryPayload;

    fn decode(&self, buffer: &[u8]) -> Result<Self::ResultType, String> {
        const EXPECTED_LEN: usize = 52; // 13 fields × 4 bytes

        if buffer.len() < EXPECTED_LEN {
            return Err(format!(
                "buffer too short: expected {} bytes, got {}",
                EXPECTED_LEN,
                buffer.len()
            ));
        }

        // 按照韌體定義的順序，以 Big-Endian 讀取每個 f32
        let x_acceleration = f32::from_be_bytes(buffer[0..4].try_into().unwrap());
        let y_acceleration = f32::from_be_bytes(buffer[4..8].try_into().unwrap());
        let z_acceleration = f32::from_be_bytes(buffer[8..12].try_into().unwrap());
        let x_angular_velocity = f32::from_be_bytes(buffer[12..16].try_into().unwrap());
        let y_angular_velocity = f32::from_be_bytes(buffer[16..20].try_into().unwrap());
        let z_angular_velocity = f32::from_be_bytes(buffer[20..24].try_into().unwrap());
        let longitude = f32::from_be_bytes(buffer[24..28].try_into().unwrap());
        let latitude = f32::from_be_bytes(buffer[28..32].try_into().unwrap());
        let altitude = f32::from_be_bytes(buffer[32..36].try_into().unwrap());
        let ground_speed = f32::from_be_bytes(buffer[36..40].try_into().unwrap());
        let vertical_velocity = f32::from_be_bytes(buffer[40..44].try_into().unwrap());
        let air_pressure = f32::from_be_bytes(buffer[44..48].try_into().unwrap());
        let temperature = f32::from_be_bytes(buffer[48..52].try_into().unwrap());

        Ok(TelemetryPayload {
            x_acceleration,
            y_acceleration,
            z_acceleration,
            x_angular_velocity,
            y_angular_velocity,
            z_angular_velocity,
            longitude,
            latitude,
            altitude,
            ground_speed,
            vertical_velocity,
            air_pressure,
            temperature,
        })
    }
}
