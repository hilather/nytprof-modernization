//! Provisional **format v6** chunk payload codecs (COL-007 runway).
//!
//! Schema: `docs/schemas/v6-payload-zlib-provisional-v0.md`
//!
//! Codec **NONE** = identity (payload already uncompressed).
//! Codec **ZLIB** = zlib-wrapped DEFLATE on the wire; inflate bounded by
//! declared `uncompressed_len`. Default `parse_chunk_frame` remains
//! non-inflating; call these helpers explicitly.
//! Not zstd/LZ4, not COL-007 C writer.

use std::io::{Read, Write};

use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;

use crate::chunk::{codec, encode_chunk_frame, ChunkFrame, MAX_CHUNK_PAYLOAD};
use crate::crc::compute_payload_crc;

/// Fail-closed upper bound on inflate output (matches chunk payload max).
pub const MAX_INFLATE_BYTES: u32 = MAX_CHUNK_PAYLOAD;

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
        }
    }
}

impl std::error::Error for PayloadCodecError {}

pub type PayloadCodecResult<T> = std::result::Result<T, PayloadCodecError>;

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
///
/// Allocation is bounded: inflate into a buffer of capacity
/// `expected_uncompressed_len` and reject overshoot via size check after finish.
pub fn inflate_zlib(
    compressed: &[u8],
    expected_uncompressed_len: u32,
) -> PayloadCodecResult<Vec<u8>> {
    if expected_uncompressed_len > MAX_INFLATE_BYTES {
        return Err(PayloadCodecError::Oversize {
            len: expected_uncompressed_len,
        });
    }
    let need = expected_uncompressed_len as usize;
    let mut dec = ZlibDecoder::new(compressed);
    let mut out = Vec::with_capacity(need);
    // Read with a hard stop: if zlib would produce more than `need`, we still
    // collect only until EOF then compare lengths (fail closed on mismatch).
    // Use read_to_end with a limited max by reading into a fixed buffer loop.
    let mut tmp = [0u8; 8192];
    loop {
        match dec.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => {
                if out.len().saturating_add(n) > need {
                    // Drain remaining? We already know it's oversize.
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

/// Decode (inflate if needed) a parsed chunk frame's payload to plain bytes.
///
/// - `codec::NONE`: returns `frame.payload` as owned bytes if
///   `uncompressed_len == compressed_len == payload.len()`; fail-closed if
///   `uncompressed_len` does not match payload length.
/// - `codec::ZLIB`: inflate payload with `frame.uncompressed_len` as bound.
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
        other => Err(PayloadCodecError::UnsupportedCodec { codec: other }),
    }
}

/// Encode a chunk frame whose on-wire payload is zlib-compressed `plain`.
///
/// Sets `codec = ZLIB`, `uncompressed_len = plain.len()`, `compressed_len` from
/// deflate output, and `payload_checksum = CRC32(on-wire compressed bytes)`
/// (provisional: checksum covers wire payload, same as CRC preflight).
pub fn encode_chunk_frame_zlib(
    kind: u8,
    flags: u16,
    sequence: u64,
    first_logical_seq: u64,
    logical_event_count: u32,
    plain: &[u8],
) -> PayloadCodecResult<Vec<u8>> {
    if plain.len() as u32 > MAX_INFLATE_BYTES {
        return Err(PayloadCodecError::Oversize {
            len: plain.len() as u32,
        });
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{parse_chunk_frame, kind};
    use crate::crc::verify_chunk_payload_crc;

    #[test]
    fn zlib_empty_and_nonempty_roundtrip() {
        for plain in [b"" as &[u8], b"hello-v6-zlib", &[0u8; 256][..]] {
            let c1 = deflate_zlib(plain).expect("deflate");
            let c2 = deflate_zlib(plain).expect("deflate again");
            // Dual deflate stability (deterministic compressor).
            assert_eq!(c1, c2);
            let out = inflate_zlib(&c1, plain.len() as u32).expect("inflate");
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
        // Wire payload is compressed, not plain.
        assert_ne!(frame.payload, plain);
        verify_chunk_payload_crc(&frame).expect("crc of wire payload");

        let got = decode_chunk_payload(&frame).expect("inflate via frame");
        assert_eq!(got, plain);
    }

    #[test]
    fn inflate_size_mismatch_err() {
        let plain = b"abcdef";
        let compressed = deflate_zlib(plain).unwrap();
        // Wrong declared length.
        match inflate_zlib(&compressed, 3) {
            Err(PayloadCodecError::SizeMismatch { expected, got }) => {
                assert_eq!(expected, 3);
                assert!(got != 3 || got == plain.len()); // either overshoot mid-read or final mismatch
            }
            other => panic!("expected size mismatch, got {other:?}"),
        }
        match inflate_zlib(&compressed, 100) {
            Err(PayloadCodecError::SizeMismatch { expected: 100, got: 6 }) => {}
            other => panic!("expected shortfall mismatch, got {other:?}"),
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
    fn oversize_declared_err() {
        assert_eq!(
            inflate_zlib(b"", MAX_INFLATE_BYTES + 1),
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
            codec::ZSTD,
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
            Err(PayloadCodecError::UnsupportedCodec {
                codec: codec::ZSTD
            })
        );
    }

    #[test]
    fn none_uncompressed_len_mismatch_err() {
        let plain = b"abcd";
        // Lie about uncompressed_len.
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
}
