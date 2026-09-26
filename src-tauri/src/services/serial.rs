use crate::infrastructures::link::TransportConfig;
use crate::infrastructures::serial::parser::ParseResult;
use crate::models::response::TelemetryPayload;

/// 封包解析器 Trait
/// 負責 Protocol v1/v2 stream framing、CRC 與 payload 解碼。
pub trait Parser {
    fn default() -> Self;
    fn sink(&mut self, byte: u8) -> ParseResult;
    fn parse_to_payload(&self, frame: &[u8]) -> Result<TelemetryPayload, String>;
}

/// 鏈路接收器 Trait
///
/// 接收器只依賴 `infrastructures::link` 提供的位元組管線，不知道底下是
/// COM 埠、TCP bridge 還是測試用的記憶體管線。
pub trait Receiver {
    async fn connect(&mut self, config: TransportConfig) -> Result<(), String>;
    async fn start_receive(&mut self) -> Result<String, String>;
}

/// 二進位解碼器 Trait：將完整 Protocol v1/v2 frame 轉換成 TelemetryPayload。
pub trait Decoder {
    type ResultType;

    fn decode(&self, buffer: &[u8]) -> Result<Self::ResultType, String>;
}
