//! NYTProf v5 packed u32/i32 (FileHandle.xs `output_tag_u32` / `read_u32`).

use crate::error::{Error, Result};

/// Encode a bare u32 (no record tag) into the NYTProf packed integer format.
pub fn encode_u32(value: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(5);
    if value < 0x80 {
        out.push(value as u8);
    } else if value < 0x4000 {
        out.push(((value >> 8) | 0x80) as u8);
        out.push(value as u8);
    } else if value < 0x200000 {
        out.push(((value >> 16) | 0xC0) as u8);
        out.push((value >> 8) as u8);
        out.push(value as u8);
    } else if value < 0x1000_0000 {
        out.push(((value >> 24) | 0xE0) as u8);
        out.push((value >> 16) as u8);
        out.push((value >> 8) as u8);
        out.push(value as u8);
    } else {
        out.push(0xFF);
        out.push((value >> 24) as u8);
        out.push((value >> 16) as u8);
        out.push((value >> 8) as u8);
        out.push(value as u8);
    }
    out
}

/// Decode a bare packed u32 from `data` starting at `pos`.
///
/// Returns `(value, new_pos)`.
pub fn decode_u32(data: &[u8], pos: usize) -> Result<(u32, usize)> {
    if pos >= data.len() {
        return Err(Error::UnexpectedEof {
            what: "integer prefix",
            offset: pos as u64,
        });
    }
    let d = data[pos];
    let mut pos = pos + 1;

    if d < 0x80 {
        return Ok((d as u32, pos));
    }

    let (mut newint, length): (u32, usize) = if d < 0xC0 {
        ((d & 0x7F) as u32, 1)
    } else if d < 0xE0 {
        ((d & 0x1F) as u32, 2)
    } else if d < 0xFF {
        ((d & 0x0F) as u32, 3)
    } else {
        (0, 4)
    };

    if pos + length > data.len() {
        return Err(Error::UnexpectedEof {
            what: "integer",
            offset: pos as u64,
        });
    }
    for _ in 0..length {
        newint = (newint << 8) | (data[pos] as u32);
        pos += 1;
    }
    Ok((newint, pos))
}

/// Read a bare packed u32, advancing `pos`.
pub fn read_u32(data: &[u8], pos: &mut usize) -> Result<u32> {
    let (v, p) = decode_u32(data, *pos)?;
    *pos = p;
    Ok(v)
}

/// Read a bare packed i32 (bitcast of packed u32), advancing `pos`.
pub fn read_i32(data: &[u8], pos: &mut usize) -> Result<i32> {
    let u = read_u32(data, pos)?;
    Ok(u as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_boundaries() {
        let samples: &[u32] = &[
            0,
            1,
            0x7F,
            0x80,
            0x3FFF,
            0x4000,
            0x1F_FFFF,
            0x20_0000,
            0x0FFF_FFFF,
            0x1000_0000,
            0xFFFF_FFFF,
            42,
            255,
            256,
            65_535,
            65_536,
            1_000_000,
            0xDEAD_BEEF,
        ];
        for &v in samples {
            let enc = encode_u32(v);
            let (dec, end) = decode_u32(&enc, 0).expect("decode");
            assert_eq!(end, enc.len(), "consumed all for {v:#x}");
            assert_eq!(dec, v, "roundtrip {v:#x}");
        }
    }

    #[test]
    fn encode_matches_documented_prefixes() {
        assert_eq!(encode_u32(0x7F), vec![0x7F]);
        assert_eq!(encode_u32(0x80), vec![0x80, 0x80]);
        // 0x4000 = 16384 → 0xC0 prefix + 2 payload bytes
        assert_eq!(encode_u32(0x4000), vec![0xC0, 0x40, 0x00]);
        assert_eq!(
            encode_u32(0xFFFF_FFFF),
            vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
        );
    }

    #[test]
    fn i32_bitcast_negative() {
        // I32 -1 as U32 is 0xFFFFFFFF
        let enc = encode_u32((-1i32) as u32);
        let mut pos = 0;
        let v = read_i32(&enc, &mut pos).unwrap();
        assert_eq!(v, -1);
    }
}
