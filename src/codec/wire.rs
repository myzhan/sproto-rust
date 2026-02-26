//! Little-endian wire format utilities for sproto encoding/decoding.

/// Read a 16-bit unsigned integer from a little-endian byte slice.
#[inline]
pub fn read_u16_le(buf: &[u8]) -> u16 {
    u16::from_le_bytes([buf[0], buf[1]])
}

/// Read a 32-bit unsigned integer from a little-endian byte slice.
#[inline]
pub fn read_u32_le(buf: &[u8]) -> u32 {
    u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]])
}

/// Read a 64-bit unsigned integer from a little-endian byte slice.
#[inline]
pub fn read_u64_le(buf: &[u8]) -> u64 {
    u64::from_le_bytes([buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7]])
}

/// Write a 16-bit unsigned integer in little-endian.
#[inline]
pub fn write_u16_le(buf: &mut [u8], val: u16) {
    let bytes = val.to_le_bytes();
    buf[0] = bytes[0];
    buf[1] = bytes[1];
}

/// Write a 32-bit unsigned integer in little-endian.
#[inline]
pub fn write_u32_le(buf: &mut [u8], val: u32) {
    let bytes = val.to_le_bytes();
    buf[..4].copy_from_slice(&bytes);
}

/// Write a 64-bit unsigned integer in little-endian.
#[inline]
pub fn write_u64_le(buf: &mut [u8], val: u64) {
    let bytes = val.to_le_bytes();
    buf[..8].copy_from_slice(&bytes);
}

/// Sign-extend a 32-bit value to 64-bit (matching C `expand64`).
#[inline]
pub fn expand64(v: u32) -> u64 {
    let value = v as u64;
    if value & 0x80000000 != 0 {
        value | (!0u64 << 32)
    } else {
        value
    }
}

/// Header size constant (field count).
pub const SIZEOF_HEADER: usize = 2;
/// Field descriptor size.
pub const SIZEOF_FIELD: usize = 2;
/// Length prefix size.
pub const SIZEOF_LENGTH: usize = 4;
/// Size of a 32-bit integer.
pub const SIZEOF_INT32: usize = 4;
/// Size of a 64-bit integer.
pub const SIZEOF_INT64: usize = 8;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_write_u16() {
        let mut buf = [0u8; 2];
        write_u16_le(&mut buf, 0x1234);
        assert_eq!(buf, [0x34, 0x12]);
        assert_eq!(read_u16_le(&buf), 0x1234);
    }

    #[test]
    fn test_read_write_u32() {
        let mut buf = [0u8; 4];
        write_u32_le(&mut buf, 0x12345678);
        assert_eq!(buf, [0x78, 0x56, 0x34, 0x12]);
        assert_eq!(read_u32_le(&buf), 0x12345678);
    }

    #[test]
    fn test_read_write_u64() {
        let mut buf = [0u8; 8];
        write_u64_le(&mut buf, 0x123456789ABCDEF0);
        assert_eq!(read_u64_le(&buf), 0x123456789ABCDEF0);
    }

    #[test]
    fn test_expand64_positive() {
        assert_eq!(expand64(100), 100u64);
        assert_eq!(expand64(0x7FFFFFFF), 0x7FFFFFFFu64);
    }

    #[test]
    fn test_expand64_negative() {
        // -1 as u32 = 0xFFFFFFFF -> should become 0xFFFFFFFFFFFFFFFF = -1 as i64
        assert_eq!(expand64(0xFFFFFFFF) as i64, -1i64);
        // -10 as u32 = 0xFFFFFFF6 -> should become -10 as i64
        assert_eq!(expand64(0xFFFFFFF6u32) as i64, -10i64);
    }
}
