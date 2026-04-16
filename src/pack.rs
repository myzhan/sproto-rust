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
    let mut ff_n: usize = 0;

    while i < srcsz {
        let word = load_word(src, i);
        let (tag, notzero) = compute_tag_word(word);

        // Promote 6/7 non-zero to 8 ONLY when already in an FF run
        let effective = if (notzero == 6 || notzero == 7) && ff_n > 0 {
            8
        } else {
            notzero
        };

        if effective == 8 {
            if ff_n == 0 {
                ff_src_start = i;
            }
            ff_n += 1;
            if ff_n == 256 {
                flush_ff(src, &mut result, ff_src_start, ff_n);
                ff_n = 0;
            }
        } else {
            if ff_n > 0 {
                flush_ff(src, &mut result, ff_src_start, ff_n);
                ff_n = 0;
            }

            // Normal pack: tag byte + non-zero bytes in one batch
            result.push(tag);
            if tag != 0 {
                pack_nonzero(&mut result, word, tag);
            }
        }

        i += 8;
    }

    if ff_n > 0 {
        flush_ff(src, &mut result, ff_src_start, ff_n);
    }

    result
}

/// Load an 8-byte word from src at offset, zero-padding if past end.
#[inline(always)]
fn load_word(src: &[u8], offset: usize) -> u64 {
    if offset + 8 <= src.len() {
        let bytes: [u8; 8] = src[offset..offset + 8].try_into().unwrap();
        u64::from_le_bytes(bytes)
    } else {
        let mut buf = [0u8; 8];
        buf[..src.len() - offset].copy_from_slice(&src[offset..]);
        u64::from_le_bytes(buf)
    }
}

/// Compute tag byte and non-zero count from an 8-byte word (branchless).
#[inline(always)]
fn compute_tag_word(word: u64) -> (u8, u32) {
    if word == 0 {
        return (0, 0);
    }
    let b = word.to_le_bytes();
    let tag = ((b[0] != 0) as u8)
        | (((b[1] != 0) as u8) << 1)
        | (((b[2] != 0) as u8) << 2)
        | (((b[3] != 0) as u8) << 3)
        | (((b[4] != 0) as u8) << 4)
        | (((b[5] != 0) as u8) << 5)
        | (((b[6] != 0) as u8) << 6)
        | (((b[7] != 0) as u8) << 7);
    (tag, tag.count_ones())
}

/// Write non-zero bytes from a word into result, guided by tag bits.
#[inline(always)]
fn pack_nonzero(result: &mut Vec<u8>, word: u64, tag: u8) {
    let b = word.to_le_bytes();
    let mut buf = [0u8; 8];
    let mut n = 0usize;
    if tag & 0x01 != 0 { buf[n] = b[0]; n += 1; }
    if tag & 0x02 != 0 { buf[n] = b[1]; n += 1; }
    if tag & 0x04 != 0 { buf[n] = b[2]; n += 1; }
    if tag & 0x08 != 0 { buf[n] = b[3]; n += 1; }
    if tag & 0x10 != 0 { buf[n] = b[4]; n += 1; }
    if tag & 0x20 != 0 { buf[n] = b[5]; n += 1; }
    if tag & 0x40 != 0 { buf[n] = b[6]; n += 1; }
    if tag & 0x80 != 0 { buf[n] = b[7]; n += 1; }
    result.extend_from_slice(&buf[..n]);
}

/// Flush an 0xFF run: write tag, count, and raw source data directly.
fn flush_ff(src: &[u8], result: &mut Vec<u8>, src_start: usize, n: usize) {
    let total_bytes = n * 8;
    result.push(0xFF);
    result.push((n - 1) as u8);
    let src_end = (src_start + total_bytes).min(src.len());
    result.extend_from_slice(&src[src_start..src_end]);
    // Zero-pad if source was shorter than total_bytes (last partial word)
    let padding = total_bytes - (src_end - src_start);
    if padding > 0 {
        result.resize(result.len() + padding, 0);
    }
}

/// Unpack (decompress) sproto packed data.
pub fn unpack(src: &[u8]) -> Result<Vec<u8>, PackError> {
    let len = src.len();
    // Pre-allocate: each tag byte produces 8 output bytes
    let mut result = Vec::with_capacity(len.saturating_mul(2));
    let mut i = 0;

    while i < len {
        let header = src[i];
        i += 1;

        if header == 0xFF {
            if i >= len {
                return Err(PackError::InvalidData(
                    "0xFF tag at end of data without count byte".into(),
                ));
            }
            let n = (src[i] as usize + 1) * 8;
            i += 1;
            if i + n > len {
                return Err(PackError::InvalidData(format!(
                    "0xFF run needs {} bytes but only {} available",
                    n,
                    len - i
                )));
            }
            result.extend_from_slice(&src[i..i + n]);
            i += n;
        } else if header == 0x00 {
            // All-zero word: write 8 zeros at once
            result.extend_from_slice(&[0u8; 8]);
        } else {
            // Single bounds check: count non-zero bytes needed from source
            let notzero = header.count_ones() as usize;
            if i + notzero > len {
                return Err(PackError::InvalidData(
                    "truncated packed data in normal segment".into(),
                ));
            }
            // Write 8 bytes: zero-initialized, then fill non-zero positions
            let out_start = result.len();
            result.resize(out_start + 8, 0);
            let out = &mut result[out_start..];
            let data = &src[i..i + notzero];
            let mut si = 0;
            if header & 0x01 != 0 { out[0] = data[si]; si += 1; }
            if header & 0x02 != 0 { out[1] = data[si]; si += 1; }
            if header & 0x04 != 0 { out[2] = data[si]; si += 1; }
            if header & 0x08 != 0 { out[3] = data[si]; si += 1; }
            if header & 0x10 != 0 { out[4] = data[si]; si += 1; }
            if header & 0x20 != 0 { out[5] = data[si]; si += 1; }
            if header & 0x40 != 0 { out[6] = data[si]; si += 1; }
            if header & 0x80 != 0 { out[7] = data[si]; }
            i += notzero;
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
