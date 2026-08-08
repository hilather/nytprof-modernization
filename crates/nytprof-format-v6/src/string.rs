//! Provisional **format v6** length-prefixed string / byte blob (COL-007 runway).
//!
//! Schema: `docs/schemas/v6-string-blob-provisional-v0.md`
//!
//! Composes strict ULEB128 for `string_id` and `byte_length`. Does **not**
//! implement dictionaries or UTF-8 validation.

use crate::varint::{decode_u64, encode_u64, VarintError};

/// Fail-closed upper bound on declared string/blob payload (16 MiB).
pub const MAX_STRING_BYTES: u64 = 16 * 1024 * 1024;

/// Flag bit 0: payload claimed as UTF-8 text (not validated in this MVP).
pub const FLAG_UTF8: u8 = 0x01;

/// Decoded length-prefixed blob (payload borrowed from input).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringBlob<'a> {
    pub id: u64,
    pub flags: u8,
    pub data: &'a [u8],
}

/// Fail-closed string/blob errors.
#[derive(Debug, PartialEq, Eq)]
pub enum StringError {
    Varint(VarintError),
    Truncated { need: usize, got: usize },
    Oversize { len: u64 },
}

impl std::fmt::Display for StringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StringError::Varint(e) => write!(f, "string blob varint: {e}"),
            StringError::Truncated { need, got } => {
                write!(f, "truncated string blob: need {need} bytes, got {got}")
            }
            StringError::Oversize { len } => {
                write!(
                    f,
                    "oversize string blob byte_length {len} (max {MAX_STRING_BYTES})"
                )
            }
        }
    }
}

impl std::error::Error for StringError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StringError::Varint(e) => Some(e),
            _ => None,
        }
    }
}

impl From<VarintError> for StringError {
    fn from(e: VarintError) -> Self {
        StringError::Varint(e)
    }
}

pub type StringResult<T> = std::result::Result<T, StringError>;

/// Canonical encode of a length-prefixed string/blob.
pub fn encode_string_blob(id: u64, flags: u8, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(
        20 + data.len(), // id + len ULEB worst-case-ish + flags + payload
    );
    out.extend_from_slice(&encode_u64(id));
    out.extend_from_slice(&encode_u64(data.len() as u64));
    out.push(flags);
    out.extend_from_slice(data);
    out
}

/// Decode a length-prefixed string/blob starting at `pos` (strict ULEB128).
///
/// Returns `(blob, bytes_consumed)`. Fail-closed on truncated input and
/// oversize `byte_length`. Payload is borrowed from `data` (no large alloc).
pub fn decode_string_blob(data: &[u8], pos: usize) -> StringResult<(StringBlob<'_>, usize)> {
    let start = pos;
    let (id, n_id) = decode_u64(data, pos)?;
    let mut p = pos + n_id;

    let (byte_len, n_len) = decode_u64(data, p)?;
    p += n_len;

    if byte_len > MAX_STRING_BYTES {
        return Err(StringError::Oversize { len: byte_len });
    }

    if p >= data.len() {
        return Err(StringError::Truncated {
            need: p + 1,
            got: data.len(),
        });
    }
    let flags = data[p];
    p += 1;

    let need = p
        .checked_add(byte_len as usize)
        .ok_or(StringError::Oversize { len: byte_len })?;
    if data.len() < need {
        return Err(StringError::Truncated {
            need,
            got: data.len(),
        });
    }

    let payload = &data[p..need];
    Ok((
        StringBlob {
            id,
            flags,
            data: payload,
        },
        need - start,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_empty() {
        let enc = encode_string_blob(0, 0, b"");
        let (blob, n) = decode_string_blob(&enc, 0).expect("empty");
        assert_eq!(n, enc.len());
        assert_eq!(blob.id, 0);
        assert_eq!(blob.flags, 0);
        assert_eq!(blob.data, b"");
    }

    #[test]
    fn roundtrip_non_empty_utf8_flag() {
        let enc = encode_string_blob(1, FLAG_UTF8, b"hi");
        let (blob, n) = decode_string_blob(&enc, 0).expect("hi");
        assert_eq!(n, enc.len());
        assert_eq!(blob.id, 1);
        assert_eq!(blob.flags, FLAG_UTF8);
        assert_eq!(blob.data, b"hi");
        // Documented wire sketch: id 01, len 02, flags 01, 68 69
        assert_eq!(enc, vec![0x01, 0x02, FLAG_UTF8, b'h', b'i']);
    }

    #[test]
    fn roundtrip_binary_payload() {
        let payload = [0u8, 255, 1, 2, 3];
        let enc = encode_string_blob(42, 0, &payload);
        let (blob, n) = decode_string_blob(&enc, 0).unwrap();
        assert_eq!(n, enc.len());
        assert_eq!(blob.id, 42);
        assert_eq!(blob.data, &payload);
    }

    #[test]
    fn trailing_bytes_ok() {
        let mut enc = encode_string_blob(0, 0, b"x");
        enc.push(0xAA);
        let (blob, n) = decode_string_blob(&enc, 0).unwrap();
        assert_eq!(blob.data, b"x");
        assert_eq!(enc[n], 0xAA);
    }

    #[test]
    fn truncated_empty_err() {
        assert!(matches!(
            decode_string_blob(&[], 0),
            Err(StringError::Varint(VarintError::Truncated { .. }))
        ));
    }

    #[test]
    fn truncated_after_header_err() {
        // id=0, len=4, flags=0, but only 1 payload byte.
        let mut enc = encode_string_blob(0, 0, b"abcd");
        enc.truncate(enc.len() - 3); // leave 1 of 4 payload bytes
        match decode_string_blob(&enc, 0) {
            Err(StringError::Truncated { need, got }) => {
                assert!(need > got);
            }
            other => panic!("expected Truncated, got {other:?}"),
        }
    }

    #[test]
    fn truncated_missing_flags_err() {
        // Only id ULEB (0x00) — missing length and flags.
        assert!(matches!(
            decode_string_blob(&[0x00], 0),
            Err(StringError::Varint(VarintError::Truncated { .. }))
        ));
    }

    #[test]
    fn oversize_declared_length_err() {
        // Craft: id=0, length=MAX+1 via raw ULEB, then flags 0 without payload.
        use crate::varint::encode_u64;
        let mut buf = encode_u64(0);
        buf.extend_from_slice(&encode_u64(MAX_STRING_BYTES + 1));
        buf.push(0); // flags — may not be reached if oversize checked after len
        assert_eq!(
            decode_string_blob(&buf, 0),
            Err(StringError::Oversize {
                len: MAX_STRING_BYTES + 1
            })
        );
    }

    #[test]
    fn max_string_bytes_constant() {
        assert_eq!(MAX_STRING_BYTES, 16 * 1024 * 1024);
    }
}
