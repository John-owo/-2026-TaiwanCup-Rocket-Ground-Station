use crate::infrastructures::serial::parser::ParseResult;
use crate::models::response::TelemetryPayload;

/// 封包解析器 Trait
/// 負責狀態機邏輯：Header → Payload → CRC → 輸出 ParseResult
pub trait Parser {
    fn new(
        expect_packet_length: usize,
        verificator: Box<dyn Verificator<VerificationType = u16> + Send + Sync>,
        decoder: Box<dyn Decoder<ResultType = TelemetryPayload> + Send + Sync>,
    ) -> Self;
    fn default() -> Self;
    fn sink(&mut self, byte: u8) -> ParseResult;
    fn parse_to_payload(&self, buffer: &[u8]) -> Result<TelemetryPayload, String>;
}

/// 序列埠接收器 Trait
pub trait Receiver {
    async fn get_connection(&mut self, path: String, baud_rate: u32) -> Result<(), String>;
    async fn start_receive(&mut self, expect_packet_length: usize) -> Result<String, String>;
    async fn receive_task(&mut self) -> Result<String, String>;
}

/// CRC 驗證器 Trait
pub trait Verificator {
    type VerificationType;

    /// 以外部 CRC 值驗證目前儲存的 internal checksum
    fn verify(&self, external_crc: Self::VerificationType) -> Result<bool, String>;

    /// 設定內部 CRC 欄位（從封包位元組計算而來）
    fn set_verification_field(
        &mut self,
        computed_crc: Self::VerificationType,
    ) -> Result<(), ()>;
}

/// 二進位解碼器 Trait：將 buffer bytes 轉換成 TelemetryPayload
pub trait Decoder {
    type ResultType;

    fn decode(&self, buffer: &[u8]) -> Result<Self::ResultType, String>;
}
