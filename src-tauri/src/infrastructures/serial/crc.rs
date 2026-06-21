/* Algorithm: CRC-16/CCITT-False
    arguments:
        polynomial: 0x1021
        width: 16
        initial: 0xFFFF
        XORout: 0x00
        RefIn: False (MSB first)
        RefOut: False
*/
pub fn crc16_ccitt(bit_stream: &[u8]) -> u16 {
    let mut crc16: u16 = 0xFFFF;
    const POLY: u16 = 0x1021;

    for &byte in bit_stream {
        crc16 ^= (byte as u16) << 8;

        for _ in 0..8 {
            if (crc16 & 0x8000) != 0 {
                crc16 = (crc16 << 1) ^ POLY;
            } else {
                crc16 <<= 1;
            }
        }
    }

    crc16
}
