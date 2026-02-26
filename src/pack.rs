use crate::error::PackError;

/// Pack (compress) sproto binary data using the zero-packing algorithm.
///
/// Similar to Cap'n Proto packing. Each 8-byte word is reduced to a tag byte
/// followed by non-zero content bytes. Tag 0xFF handles the case where most
/// bytes are non-zero.
pub fn pack(src: &[u8]) -> Vec<u8> {
    let srcsz = src.len();
    if srcsz == 0 {
        return Vec::new();
    }

    let mut result = Vec::with_capacity(srcsz);
    let mut i = 0;

    // State for 0xFF run batching
    let mut ff_src_start: usize = 0;
    let mut ff_des_start: usize = 0;
    let mut ff_n: usize = 0;

    while i < srcsz {
        // Get 8-byte chunk, padding with zeros if needed
        let mut chunk = [0u8; 8];
        let remaining = srcsz - i;
        let copy_len = remaining.min(8);
        chunk[..copy_len].copy_from_slice(&src[i..i + copy_len]);

        let (tag, mut notzero) = compute_tag(&chunk);

        // Promote 6/7 non-zero to 8 ONLY when already in an FF run
        if (notzero == 6 || notzero == 7) && ff_n > 0 {
            notzero = 8;
        }

        if notzero == 8 {
            if ff_n > 0 {
                // Continue FF run: reserve 8 more bytes for raw data
                ff_n += 1;
                result.extend_from_slice(&[0u8; 8]);
                if ff_n == 256 {
                    write_ff_data(src, srcsz, &mut result, ff_src_start, ff_des_start, ff_n);
                    ff_n = 0;
                }
            } else {
                // Start new FF run: reserve 10 bytes (tag + count + 8 data)
                ff_src_start = i;
                ff_des_start = result.len();
                ff_n = 1;
                result.extend_from_slice(&[0u8; 10]);
            }
        } else {
            if ff_n > 0 {
                // Flush pending FF run
                write_ff_data(src, srcsz, &mut result, ff_src_start, ff_des_start, ff_n);
                ff_n = 0;
            }

            // Normal pack: tag byte + non-zero bytes
            result.push(tag);
            for &byte in &chunk {
                if byte != 0 {
                    result.push(byte);
                }
            }
        }

        i += 8;
    }

    if ff_n > 0 {
        write_ff_data(src, srcsz, &mut result, ff_src_start, ff_des_start, ff_n);
    }

    result
}

fn compute_tag(chunk: &[u8; 8]) -> (u8, usize) {
    let mut tag: u8 = 0;
    let mut notzero = 0;
    for (i, &byte) in chunk.iter().enumerate() {
        if byte != 0 {
            notzero += 1;
            tag |= 1 << i;
        }
    }
    (tag, notzero)
}

fn write_ff_data(
    src: &[u8],
    src_len: usize,
    result: &mut Vec<u8>,
    src_start: usize,
    des_start: usize,
    n: usize,
) {
    let total_bytes = n * 8;
    result[des_start] = 0xFF;
    result[des_start + 1] = (n - 1) as u8;

    // Copy actual data, zero-padding if past end of source
    let src_end = (src_start + total_bytes).min(src_len);
    let available = src_end - src_start;
    result[des_start + 2..des_start + 2 + available]
        .copy_from_slice(&src[src_start..src_end]);
    // Zero-fill any remaining
    for b in result[des_start + 2 + available..des_start + 2 + total_bytes].iter_mut() {
        *b = 0;
    }

    // Truncate result to exactly des_start + 2 + total_bytes
    result.truncate(des_start + 2 + total_bytes);
}

/// Unpack (decompress) sproto packed data.
pub fn unpack(src: &[u8]) -> Result<Vec<u8>, PackError> {
    let mut result = Vec::new();
    let mut i = 0;

    while i < src.len() {
        let header = src[i];
        i += 1;

        if header == 0xFF {
            if i >= src.len() {
                return Err(PackError::InvalidData(
                    "0xFF tag at end of data without count byte".into(),
                ));
            }
            let n = (src[i] as usize + 1) * 8;
            i += 1;
            if i + n > src.len() {
                return Err(PackError::InvalidData(format!(
                    "0xFF run needs {} bytes but only {} available",
                    n,
                    src.len() - i
                )));
            }
            result.extend_from_slice(&src[i..i + n]);
            i += n;
        } else {
            for bit in 0..8 {
                if header & (1 << bit) != 0 {
                    if i >= src.len() {
                        return Err(PackError::InvalidData(
                            "truncated packed data in normal segment".into(),
                        ));
                    }
                    result.push(src[i]);
                    i += 1;
                } else {
                    result.push(0);
                }
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_unpack_basic() {
        // Example from README:
        // unpacked: 08 00 00 00 03 00 02 00  19 00 00 00 aa 01 00 00
        // packed:   51 08 03 02  31 19 aa 01
        let unpacked = vec![
            0x08, 0x00, 0x00, 0x00, 0x03, 0x00, 0x02, 0x00,
            0x19, 0x00, 0x00, 0x00, 0xaa, 0x01, 0x00, 0x00,
        ];
        let expected_packed = vec![0x51, 0x08, 0x03, 0x02, 0x31, 0x19, 0xaa, 0x01];

        let packed = pack(&unpacked);
        assert_eq!(packed, expected_packed);

        let roundtrip = unpack(&packed).unwrap();
        assert_eq!(roundtrip, unpacked);
    }

    #[test]
    fn test_pack_unpack_ff() {
        // 30 bytes of 0x8a -> pads to 32 bytes (4 words)
        // First 3 words (24 bytes) all non-zero -> FF run
        // Last word: 8a 8a 8a 8a 8a 8a 00 00 -> normal
        let mut unpacked = vec![0x8a; 30];
        let packed = pack(&unpacked);

        // Should start with FF tag
        assert_eq!(packed[0], 0xFF);

        let roundtrip = unpack(&packed).unwrap();
        // Roundtrip pads to 8-byte boundary
        unpacked.extend_from_slice(&[0, 0]);
        assert_eq!(roundtrip, unpacked);
    }

    #[test]
    fn test_pack_empty() {
        assert_eq!(pack(&[]), Vec::<u8>::new());
    }

    #[test]
    fn test_pack_all_zeros() {
        let zeros = vec![0u8; 16];
        let packed = pack(&zeros);
        // Two 0x00 tag bytes, no data bytes
        assert_eq!(packed, vec![0x00, 0x00]);
        let roundtrip = unpack(&packed).unwrap();
        assert_eq!(roundtrip, zeros);
    }

    #[test]
    fn test_unpack_invalid() {
        // 0xFF at end without count byte
        assert!(unpack(&[0xFF]).is_err());
    }
}
