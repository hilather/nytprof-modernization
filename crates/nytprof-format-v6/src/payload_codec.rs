//! Provisional **format v6** chunk payload codecs (COL-007 runway).
//!
//! Schemas:
//! - `docs/schemas/v6-payload-zlib-provisional-v0.md`
//! - `docs/schemas/v6-payload-zstd-provisional-v0.md`
//! - `docs/schemas/v6-payload-lz4-provisional-v0.md`
//!
//! Codec **NONE** = identity (payload already uncompressed).
//! Codec **ZLIB** = zlib-wrapped DEFLATE on the wire.
//! Codec **ZSTD** = zstd frame on the wire.
//! Codec **LZ4** = LZ4 raw block on the wire (size from chunk `uncompressed_len`).
//! Inflate is bounded by declared `uncompressed_len`. Default `parse_chunk_frame`
//! remains non-inflating; call these helpers explicitly.
//! Not dictionaries, not COL-007 C writer, not CLI v6 default.

use std::io::{Read, Write};

use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;

use crate::chunk::{codec, encode_chunk_frame, ChunkFrame, MAX_CHUNK_PAYLOAD};
use crate::crc::compute_payload_crc;

/// Fail-closed upper bound on inflate output (matches chunk payload max).
pub const MAX_INFLATE_BYTES: u32 = MAX_CHUNK_PAYLOAD;

/// Default zstd compression level for provisional encode helpers.
const ZSTD_LEVEL: i32 = 3;

/// Fail-closed payload codec errors.
#[derive(Debug, PartialEq, Eq)]
pub enum PayloadCodecError {
    /// Unsupported or reserved codec id for this MVP path.
    UnsupportedCodec { codec: u8 },
    /// Inflated length does not match declared `uncompressed_len`.
    SizeMismatch { expected: u32, got: usize },
    /// Declared uncompressed length exceeds fail-closed cap.
    Oversize { len: u32 },
    /// zlib inflate/deflate failed (corrupt stream, I/O, etc.).
    Zlib { message: String },
    /// zstd compress/decompress failed (corrupt stream, etc.).
    Zstd { message: String },
    /// LZ4 compress/decompress failed (corrupt block, etc.).
    Lz4 { message: String },
}

impl std::fmt::Display for PayloadCodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PayloadCodecError::UnsupportedCodec { codec } => {
                write!(f, "unsupported payload codec {codec}")
            }
            PayloadCodecError::SizeMismatch { expected, got } => {
                write!(
                    f,
                    "inflated size mismatch: expected {expected} bytes, got {got}"
                )
            }
            PayloadCodecError::Oversize { len } => {
                write!(f, "oversize uncompressed_len {len} (max {MAX_INFLATE_BYTES})")
            }
            PayloadCodecError::Zlib { message } => write!(f, "zlib error: {message}"),
            PayloadCodecError::Zstd { message } => write!(f, "zstd error: {message}"),
            PayloadCodecError::Lz4 { message } => write!(f, "lz4 error: {message}"),
        }
    }
}

impl std::error::Error for PayloadCodecError {}

pub type PayloadCodecResult<T> = std::result::Result<T, PayloadCodecError>;

fn check_uncompressed_cap(len: u32) -> PayloadCodecResult<()> {
    if len > MAX_INFLATE_BYTES {
        return Err(PayloadCodecError::Oversize { len });
    }
    Ok(())
}

fn check_plain_cap(plain: &[u8]) -> PayloadCodecResult<()> {
    if plain.len() as u32 > MAX_INFLATE_BYTES {
        return Err(PayloadCodecError::Oversize {
            len: plain.len() as u32,
        });
    }
    Ok(())
}

// --- ZLIB -------------------------------------------------------------------

/// Deflate `plain` with zlib wrapper (default compression level).
///
/// Pure function over byte slices. Empty input is valid (produces a small zlib stream).
pub fn deflate_zlib(plain: &[u8]) -> PayloadCodecResult<Vec<u8>> {
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    enc.write_all(plain)
        .map_err(|e| PayloadCodecError::Zlib {
            message: e.to_string(),
        })?;
    enc.finish().map_err(|e| PayloadCodecError::Zlib {
        message: e.to_string(),
    })
}

/// Inflate zlib-compressed `compressed` bytes.
///
/// Fail-closed when:
/// - `expected_uncompressed_len` exceeds `MAX_INFLATE_BYTES`
/// - inflate fails (corrupt stream)
/// - inflated length ≠ `expected_uncompressed_len`
pub fn inflate_zlib(
    compressed: &[u8],
    expected_uncompressed_len: u32,
) -> PayloadCodecResult<Vec<u8>> {
    check_uncompressed_cap(expected_uncompressed_len)?;
    let need = expected_uncompressed_len as usize;
    let mut dec = ZlibDecoder::new(compressed);
    let mut out = Vec::with_capacity(need);
    let mut tmp = [0u8; 8192];
    loop {
        match dec.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => {
                if out.len().saturating_add(n) > need {
                    return Err(PayloadCodecError::SizeMismatch {
                        expected: expected_uncompressed_len,
                        got: out.len() + n,
                    });
                }
                out.extend_from_slice(&tmp[..n]);
            }
            Err(e) => {
                return Err(PayloadCodecError::Zlib {
                    message: e.to_string(),
                });
            }
        }
    }
    if out.len() != need {
        return Err(PayloadCodecError::SizeMismatch {
            expected: expected_uncompressed_len,
            got: out.len(),
        });
    }
    Ok(out)
}

// --- ZSTD -------------------------------------------------------------------

/// Compress `plain` to a zstd frame (provisional default level).
///
/// Pure function over byte slices. Empty input is valid.
pub fn compress_zstd(plain: &[u8]) -> PayloadCodecResult<Vec<u8>> {
    zstd::bulk::compress(plain, ZSTD_LEVEL).map_err(|e| PayloadCodecError::Zstd {
        message: e.to_string(),
    })
}

/// Decompress a zstd frame into exactly `expected_uncompressed_len` bytes.
///
/// Fail-closed on oversize declaration, corrupt stream, or size mismatch.
pub fn decompress_zstd(
    compressed: &[u8],
    expected_uncompressed_len: u32,
) -> PayloadCodecResult<Vec<u8>> {
    check_uncompressed_cap(expected_uncompressed_len)?;
    let need = expected_uncompressed_len as usize;
    // Bound output capacity to declared length; reject if library would need more.
    let out = zstd::bulk::decompress(compressed, need).map_err(|e| PayloadCodecError::Zstd {
        message: e.to_string(),
    })?;
    if out.len() != need {
        return Err(PayloadCodecError::SizeMismatch {
            expected: expected_uncompressed_len,
            got: out.len(),
        });
    }
    Ok(out)
}

// --- LZ4 --------------------------------------------------------------------

/// Compress `plain` to an LZ4 **raw block** (no independent frame / size prefix).
///
/// Size is carried by the chunk header `uncompressed_len` on the v6 path.
/// Pure function over byte slices. Empty input is valid.
pub fn compress_lz4(plain: &[u8]) -> PayloadCodecResult<Vec<u8>> {
    Ok(lz4_flex::block::compress(plain))
}

/// Decompress an LZ4 raw block into exactly `expected_uncompressed_len` bytes.
///
/// Fail-closed on oversize declaration, corrupt block, or size mismatch.
pub fn decompress_lz4(
    compressed: &[u8],
    expected_uncompressed_len: u32,
) -> PayloadCodecResult<Vec<u8>> {
    check_uncompressed_cap(expected_uncompressed_len)?;
    let need = expected_uncompressed_len as usize;
    let out = lz4_flex::block::decompress(compressed, need).map_err(|e| PayloadCodecError::Lz4 {
        message: e.to_string(),
    })?;
    if out.len() != need {
        return Err(PayloadCodecError::SizeMismatch {
            expected: expected_uncompressed_len,
            got: out.len(),
        });
    }
    Ok(out)
}

// --- Frame helpers ----------------------------------------------------------

/// Decode (inflate if needed) a parsed chunk frame's payload to plain bytes.
///
/// - `codec::NONE`: identity when `uncompressed_len == payload.len()`
/// - `codec::ZLIB` / `ZSTD` / `LZ4`: inflate with `uncompressed_len` bound
/// - other codecs: `UnsupportedCodec`
///
/// Default `parse_chunk_frame` is unchanged (non-inflating).
pub fn decode_chunk_payload(frame: &ChunkFrame<'_>) -> PayloadCodecResult<Vec<u8>> {
    match frame.codec {
        c if c == codec::NONE => {
            if frame.uncompressed_len as usize != frame.payload.len() {
                return Err(PayloadCodecError::SizeMismatch {
                    expected: frame.uncompressed_len,
                    got: frame.payload.len(),
                });
            }
            Ok(frame.payload.to_vec())
        }
        c if c == codec::ZLIB => inflate_zlib(frame.payload, frame.uncompressed_len),
        c if c == codec::ZSTD => decompress_zstd(frame.payload, frame.uncompressed_len),
        c if c == codec::LZ4 => decompress_lz4(frame.payload, frame.uncompressed_len),
        other => Err(PayloadCodecError::UnsupportedCodec { codec: other }),
    }
}

/// Encode a chunk frame whose on-wire payload is zlib-compressed `plain`.
pub fn encode_chunk_frame_zlib(
    kind: u8,
    flags: u16,
    sequence: u64,
    first_logical_seq: u64,
    logical_event_count: u32,
    plain: &[u8],
) -> PayloadCodecResult<Vec<u8>> {
    check_plain_cap(plain)?;
    let compressed = deflate_zlib(plain)?;
    let checksum = compute_payload_crc(&compressed);
    Ok(encode_chunk_frame(
        kind,
        codec::ZLIB,
        flags,
        sequence,
        first_logical_seq,
        logical_event_count,
        plain.len() as u32,
        &compressed,
        checksum,
    ))
}

/// Encode a chunk frame whose on-wire payload is zstd-compressed `plain`.
pub fn encode_chunk_frame_zstd(
    kind: u8,
    flags: u16,
    sequence: u64,
    first_logical_seq: u64,
    logical_event_count: u32,
    plain: &[u8],
) -> PayloadCodecResult<Vec<u8>> {
    check_plain_cap(plain)?;
    let compressed = compress_zstd(plain)?;
    let checksum = compute_payload_crc(&compressed);
    Ok(encode_chunk_frame(
        kind,
        codec::ZSTD,
        flags,
        sequence,
        first_logical_seq,
        logical_event_count,
        plain.len() as u32,
        &compressed,
        checksum,
    ))
}

/// Encode a chunk frame whose on-wire payload is LZ4-block-compressed `plain`.
pub fn encode_chunk_frame_lz4(
    kind: u8,
    flags: u16,
    sequence: u64,
    first_logical_seq: u64,
    logical_event_count: u32,
    plain: &[u8],
) -> PayloadCodecResult<Vec<u8>> {
    check_plain_cap(plain)?;
    let compressed = compress_lz4(plain)?;
    let checksum = compute_payload_crc(&compressed);
    Ok(encode_chunk_frame(
        kind,
        codec::LZ4,
        flags,
        sequence,
        first_logical_seq,
        logical_event_count,
        plain.len() as u32,
        &compressed,
        checksum,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{kind, parse_chunk_frame};
    use crate::crc::verify_chunk_payload_crc;

    #[test]
    fn zlib_empty_and_nonempty_roundtrip() {
        for plain in [b"" as &[u8], b"hello-v6-zlib", &[0u8; 256][..]] {
            let c1 = deflate_zlib(plain).expect("deflate");
            let c2 = deflate_zlib(plain).expect("deflate again");
            assert_eq!(c1, c2);
            let out = inflate_zlib(&c1, plain.len() as u32).expect("inflate");
            assert_eq!(out, plain);
        }
    }

    #[test]
    fn zstd_empty_and_nonempty_roundtrip() {
        for plain in [b"" as &[u8], b"hello-v6-zstd", &[0u8; 256][..]] {
            let c1 = compress_zstd(plain).expect("compress");
            let c2 = compress_zstd(plain).expect("compress again");
            assert_eq!(c1, c2);
            let out = decompress_zstd(&c1, plain.len() as u32).expect("decompress");
            assert_eq!(out, plain);
        }
    }

    #[test]
    fn lz4_empty_and_nonempty_roundtrip() {
        for plain in [b"" as &[u8], b"hello-v6-lz4", &[0u8; 256][..]] {
            let c1 = compress_lz4(plain).expect("compress");
            let c2 = compress_lz4(plain).expect("compress again");
            assert_eq!(c1, c2);
            let out = decompress_lz4(&c1, plain.len() as u32).expect("decompress");
            assert_eq!(out, plain);
        }
    }

    #[test]
    fn none_codec_identity_via_decode_chunk_payload() {
        let plain = b"opaque-none";
        let frame_bytes = encode_chunk_frame(
            kind::EVENT,
            codec::NONE,
            0,
            0,
            0,
            0,
            plain.len() as u32,
            plain,
            compute_payload_crc(plain),
        );
        let frame = parse_chunk_frame(&frame_bytes).expect("parse");
        let got = decode_chunk_payload(&frame).expect("NONE identity");
        assert_eq!(got, plain);
    }

    #[test]
    fn zlib_chunk_composition_roundtrip() {
        let plain = b"event-body-bytes-for-zlib";
        let enc_a = encode_chunk_frame_zlib(kind::EVENT, 0, 1, 0, 0, plain).expect("zlib chunk");
        let enc_b = encode_chunk_frame_zlib(kind::EVENT, 0, 1, 0, 0, plain).expect("zlib chunk 2");
        assert_eq!(enc_a, enc_b);

        let frame = parse_chunk_frame(&enc_a).expect("parse");
        assert_eq!(frame.codec, codec::ZLIB);
        assert_eq!(frame.uncompressed_len, plain.len() as u32);
        assert_eq!(frame.compressed_len as usize, frame.payload.len());
        assert_ne!(frame.payload, plain);
        verify_chunk_payload_crc(&frame).expect("crc of wire payload");

        let got = decode_chunk_payload(&frame).expect("inflate via frame");
        assert_eq!(got, plain);
    }

    #[test]
    fn zstd_chunk_composition_roundtrip() {
        let plain = b"event-body-bytes-for-zstd";
        let enc_a = encode_chunk_frame_zstd(kind::EVENT, 0, 2, 0, 0, plain).expect("zstd chunk");
        let enc_b = encode_chunk_frame_zstd(kind::EVENT, 0, 2, 0, 0, plain).expect("zstd chunk 2");
        assert_eq!(enc_a, enc_b);

        let frame = parse_chunk_frame(&enc_a).expect("parse");
        assert_eq!(frame.codec, codec::ZSTD);
        assert_eq!(frame.uncompressed_len, plain.len() as u32);
        assert_eq!(frame.compressed_len as usize, frame.payload.len());
        assert_ne!(frame.payload, plain);
        // Default parse must leave payload compressed (non-inflating).
        assert_ne!(frame.payload, plain);
        verify_chunk_payload_crc(&frame).expect("crc of wire payload");

        let got = decode_chunk_payload(&frame).expect("decompress via frame");
        assert_eq!(got, plain);
    }

    #[test]
    fn lz4_chunk_composition_roundtrip() {
        let plain = b"event-body-bytes-for-lz4!!";
        let enc_a = encode_chunk_frame_lz4(kind::EVENT, 0, 3, 0, 0, plain).expect("lz4 chunk");
        let enc_b = encode_chunk_frame_lz4(kind::EVENT, 0, 3, 0, 0, plain).expect("lz4 chunk 2");
        assert_eq!(enc_a, enc_b);

        let frame = parse_chunk_frame(&enc_a).expect("parse");
        assert_eq!(frame.codec, codec::LZ4);
        assert_eq!(frame.uncompressed_len, plain.len() as u32);
        assert_eq!(frame.compressed_len as usize, frame.payload.len());
        assert_ne!(frame.payload, plain);
        verify_chunk_payload_crc(&frame).expect("crc of wire payload");

        let got = decode_chunk_payload(&frame).expect("decompress via frame");
        assert_eq!(got, plain);
    }

    #[test]
    fn inflate_size_mismatch_err() {
        let plain = b"abcdef";
        let compressed = deflate_zlib(plain).unwrap();
        match inflate_zlib(&compressed, 3) {
            Err(PayloadCodecError::SizeMismatch { expected, got }) => {
                assert_eq!(expected, 3);
                assert!(got != 3 || got == plain.len());
            }
            other => panic!("expected size mismatch, got {other:?}"),
        }
        match inflate_zlib(&compressed, 100) {
            Err(PayloadCodecError::SizeMismatch {
                expected: 100,
                got: 6,
            }) => {}
            other => panic!("expected shortfall mismatch, got {other:?}"),
        }
    }

    #[test]
    fn zstd_size_mismatch_err() {
        let plain = b"abcdef";
        let compressed = compress_zstd(plain).unwrap();
        match decompress_zstd(&compressed, 100) {
            Err(PayloadCodecError::SizeMismatch {
                expected: 100,
                got: 6,
            })
            | Err(PayloadCodecError::Zstd { .. }) => {}
            other => panic!("expected size/zstd error, got {other:?}"),
        }
        // Declared too small: zstd bulk decompress with capacity 3 should fail closed.
        match decompress_zstd(&compressed, 3) {
            Err(PayloadCodecError::SizeMismatch { expected: 3, .. })
            | Err(PayloadCodecError::Zstd { .. }) => {}
            other => panic!("expected size/zstd error for short declare, got {other:?}"),
        }
    }

    #[test]
    fn lz4_size_mismatch_err() {
        let plain = b"abcdef";
        let compressed = compress_lz4(plain).unwrap();
        match decompress_lz4(&compressed, 100) {
            Err(PayloadCodecError::SizeMismatch { .. }) | Err(PayloadCodecError::Lz4 { .. }) => {}
            other => panic!("expected size/lz4 error, got {other:?}"),
        }
        match decompress_lz4(&compressed, 3) {
            Err(PayloadCodecError::SizeMismatch { expected: 3, .. })
            | Err(PayloadCodecError::Lz4 { .. }) => {}
            other => panic!("expected size/lz4 error for short declare, got {other:?}"),
        }
    }

    #[test]
    fn corrupt_zlib_err() {
        match inflate_zlib(b"not-zlib-data!!!!", 4) {
            Err(PayloadCodecError::Zlib { .. }) | Err(PayloadCodecError::SizeMismatch { .. }) => {}
            other => panic!("expected zlib/size error, got {other:?}"),
        }
    }

    #[test]
    fn corrupt_zstd_err() {
        match decompress_zstd(b"not-zstd-frame!!!!", 4) {
            Err(PayloadCodecError::Zstd { .. }) | Err(PayloadCodecError::SizeMismatch { .. }) => {}
            other => panic!("expected zstd/size error, got {other:?}"),
        }
    }

    #[test]
    fn corrupt_lz4_err() {
        // Non-empty garbage as an LZ4 block with a non-zero declare should fail.
        match decompress_lz4(b"\xff\xff\xff\xffnot-lz4", 16) {
            Err(PayloadCodecError::Lz4 { .. }) | Err(PayloadCodecError::SizeMismatch { .. }) => {}
            other => panic!("expected lz4/size error, got {other:?}"),
        }
    }

    #[test]
    fn oversize_declared_err() {
        assert_eq!(
            inflate_zlib(b"", MAX_INFLATE_BYTES + 1),
            Err(PayloadCodecError::Oversize {
                len: MAX_INFLATE_BYTES + 1
            })
        );
        assert_eq!(
            decompress_zstd(b"", MAX_INFLATE_BYTES + 1),
            Err(PayloadCodecError::Oversize {
                len: MAX_INFLATE_BYTES + 1
            })
        );
        assert_eq!(
            decompress_lz4(b"", MAX_INFLATE_BYTES + 1),
            Err(PayloadCodecError::Oversize {
                len: MAX_INFLATE_BYTES + 1
            })
        );
    }

    #[test]
    fn unsupported_codec_err() {
        let plain = b"x";
        let frame_bytes = encode_chunk_frame(
            kind::EVENT,
            0xFE, // reserved / unknown
            0,
            0,
            0,
            0,
            1,
            plain,
            0,
        );
        let frame = parse_chunk_frame(&frame_bytes).unwrap();
        assert_eq!(
            decode_chunk_payload(&frame),
            Err(PayloadCodecError::UnsupportedCodec { codec: 0xFE })
        );
    }

    #[test]
    fn none_uncompressed_len_mismatch_err() {
        let plain = b"abcd";
        let frame_bytes = encode_chunk_frame(
            kind::EVENT,
            codec::NONE,
            0,
            0,
            0,
            0,
            99,
            plain,
            0,
        );
        let frame = parse_chunk_frame(&frame_bytes).unwrap();
        match decode_chunk_payload(&frame) {
            Err(PayloadCodecError::SizeMismatch {
                expected: 99,
                got: 4,
            }) => {}
            other => panic!("expected size mismatch, got {other:?}"),
        }
    }

    #[test]
    fn codec_ids_match_chunk_frame_table() {
        assert_eq!(codec::NONE, 0);
        assert_eq!(codec::ZLIB, 1);
        assert_eq!(codec::ZSTD, 2);
        assert_eq!(codec::LZ4, 3);
    }
}
