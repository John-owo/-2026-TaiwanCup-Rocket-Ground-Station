use crate::infrastructures::serial::parser::ParseResult;
use crate::models::response::TelemetryPayload;

/// 封包解析器 Trait
/// 負責 Protocol v1/v2 stream framing、CRC 與 payload 解碼。
pub trait Parser {
    fn default() -> Self;
    fn sink(&mut self, byte: u8) -> ParseResult;
    fn parse_to_payload(&self, frame: &[u8]) -> Result<TelemetryPayload, String>;
}

/// 序列埠接收器 Trait
pub trait Receiver {
    async fn get_connection(&mut self, path: String, baud_rate: u32) -> Result<(), String>;
    async fn start_receive(&mut self) -> Result<String, String>;
    async fn receive_task(&mut self) -> Result<String, String>;
}

/// 二進位解碼器 Trait：將完整 Protocol v1/v2 frame 轉換成 TelemetryPayload。
pub trait Decoder {
    type ResultType;

    fn decode(&self, buffer: &[u8]) -> Result<Self::ResultType, String>;
}
