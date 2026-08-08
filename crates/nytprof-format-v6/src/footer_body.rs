//! Provisional **format v6** FOOTER chunk body (codec NONE) (COL-007 runway).
//!
//! Schema: `docs/schemas/v6-footer-body-provisional-v0.md`
//!
//! Ordered footer records: ULEB128 key_id + value + optional length-prefixed
//! string-blob label. Composes shipped varint + string-blob.
//! Empty body remains valid (compat with opaque empty FOOTER).
//! No inflate, no CRC freeze, no C writer.

use crate::string::{decode_string_blob, encode_string_blob, StringBlob, StringError};
use crate::varint::{decode_u64, encode_u64, VarintError};

/// Fail-closed upper bound on total FOOTER body size (64 MiB).
pub const MAX_FOOTER_BODY_BYTES: usize = 64 * 1024 * 1024;

/// One decoded FOOTER record (label borrowed from input).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FooterRecord<'a> {
    /// Provisional key (e.g. total_events, end_marker).
    pub key_id: u64,
    /// Provisional counter/total/value.
    pub value: u64,
    /// Optional human label (may be empty).
    pub label: StringBlob<'a>,
}

/// Spec for encoding one FOOTER record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FooterRecordSpec<'a> {
    pub key_id: u64,
    pub value: u64,
    pub string_id: u64,
    pub string_flags: u8,
    pub label: &'a [u8],
}

/// Fail-closed FOOTER-body errors.
#[derive(Debug, PartialEq, Eq)]
pub enum FooterBodyError {
    Varint(VarintError),
    String(StringError),
    Truncated { need: usize, got: usize },
    Oversize { len: usize },
}

impl std::fmt::Display for FooterBodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FooterBodyError::Varint(e) => write!(f, "footer-body varint: {e}"),
            FooterBodyError::String(e) => write!(f, "footer-body string: {e}"),
            FooterBodyError::Truncated { need, got } => {
                write!(f, "truncated footer-body: need {need} bytes, got {got}")
            }
            FooterBodyError::Oversize { len } => {
                write!(
                    f,
                    "oversize footer-body {len} bytes (max {MAX_FOOTER_BODY_BYTES})"
                )
            }
        }
    }
}

impl std::error::Error for FooterBodyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FooterBodyError::Varint(e) => Some(e),
            FooterBodyError::String(e) => Some(e),
            _ => None,
        }
    }
}

impl From<VarintError> for FooterBodyError {
    fn from(e: VarintError) -> Self {
        FooterBodyError::Varint(e)
    }
}

impl From<StringError> for FooterBodyError {
    fn from(e: StringError) -> Self {
        FooterBodyError::String(e)
    }
}

pub type FooterBodyResult<T> = std::result::Result<T, FooterBodyError>;

/// Encode a provisional FOOTER-body (codec NONE chunk payload).
///
/// Each record: `ULEB128 key_id || ULEB128 value || string_blob(id, flags, label)`.
/// Empty `records` → empty body (valid last FOOTER). Pure byte-slice / `Vec` API.
pub fn encode_footer_body(records: &[FooterRecordSpec<'_>]) -> Vec<u8> {
    let mut out = Vec::new();
    for rec in records {
        out.extend_from_slice(&encode_u64(rec.key_id));
        out.extend_from_slice(&encode_u64(rec.value));
        out.extend_from_slice(&encode_string_blob(
            rec.string_id,
            rec.string_flags,
            rec.label,
        ));
    }
    out
}

/// Decode a provisional FOOTER-body until the buffer is exhausted.
///
/// Empty input → empty list. Fail-closed on truncated mid-record or oversize.
/// Returns `(records, bytes_consumed)` (`bytes_consumed == data.len()` on success).
pub fn decode_footer_body(data: &[u8]) -> FooterBodyResult<(Vec<FooterRecord<'_>>, usize)> {
    if data.len() > MAX_FOOTER_BODY_BYTES {
        return Err(FooterBodyError::Oversize { len: data.len() });
    }
    let mut pos = 0;
    let mut out = Vec::new();
    while pos < data.len() {
        if pos > MAX_FOOTER_BODY_BYTES {
            return Err(FooterBodyError::Oversize { len: pos });
        }
        let (key_id, n1) = decode_u64(data, pos)?;
        pos += n1;
        let (value, n2) = decode_u64(data, pos)?;
        pos += n2;
        let (label, n3) = decode_string_blob(data, pos)?;
        pos += n3;
        out.push(FooterRecord {
            key_id,
            value,
            label,
        });
    }
    Ok((out, pos))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{codec, encode_chunk_frame, kind};
    use crate::string::FLAG_UTF8;

    #[test]
    fn empty_footer_body_roundtrip() {
        let enc_a = encode_footer_body(&[]);
        let enc_b = encode_footer_body(&[]);
        assert_eq!(enc_a, enc_b);
        assert!(enc_a.is_empty());
        let (recs, n) = decode_footer_body(&enc_a).expect("empty");
        assert_eq!(n, 0);
        assert!(recs.is_empty());
    }

    #[test]
    fn footer_records_roundtrip() {
        let specs = [
            FooterRecordSpec {
                key_id: 1,
                value: 2474,
                string_id: 0,
                string_flags: FLAG_UTF8,
                label: b"total_events",
            },
            FooterRecordSpec {
                key_id: 2,
                value: 0,
                string_id: 0,
                string_flags: 0,
                label: b"",
            },
        ];
        let enc = encode_footer_body(&specs);
        let mut expect = Vec::new();
        expect.extend_from_slice(&encode_u64(1));
        expect.extend_from_slice(&encode_u64(2474));
        expect.extend_from_slice(&encode_string_blob(0, FLAG_UTF8, b"total_events"));
        expect.extend_from_slice(&encode_u64(2));
        expect.extend_from_slice(&encode_u64(0));
        expect.extend_from_slice(&encode_string_blob(0, 0, b""));
        assert_eq!(enc, expect);

        let (recs, n) = decode_footer_body(&enc).expect("roundtrip");
        assert_eq!(n, enc.len());
        assert_eq!(recs.len(), 2);
        assert_eq!(recs[0].key_id, 1);
        assert_eq!(recs[0].value, 2474);
        assert_eq!(recs[0].label.data, b"total_events");
        assert_eq!(recs[1].key_id, 2);
        assert!(recs[1].label.data.is_empty());
    }

    #[test]
    fn truncated_mid_record_err() {
        let full = encode_footer_body(&[FooterRecordSpec {
            key_id: 1,
            value: 2,
            string_id: 0,
            string_flags: 0,
            label: b"hello",
        }]);
        let trunc = &full[..full.len() - 1];
        match decode_footer_body(trunc) {
            Err(FooterBodyError::Varint(_))
            | Err(FooterBodyError::String(_))
            | Err(FooterBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated mid-record, got {other:?}"),
        }
    }

    #[test]
    fn truncated_after_key_err() {
        let partial = encode_u64(1);
        match decode_footer_body(&partial) {
            Err(FooterBodyError::Varint(_)) | Err(FooterBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated after key, got {other:?}"),
        }
    }

    #[test]
    fn never_panic_garbage() {
        assert!(decode_footer_body(&[]).is_ok());
        let _ = decode_footer_body(&[0xFF; 6]);
        let _ = decode_footer_body(b"\x01\x02");
    }

    #[test]
    fn footer_as_codec_none_chunk_payload() {
        let body = encode_footer_body(&[FooterRecordSpec {
            key_id: 9,
            value: 99,
            string_id: 0,
            string_flags: 0,
            label: b"end",
        }]);
        let frame = encode_chunk_frame(
            kind::FOOTER,
            codec::NONE,
            0,
            0,
            0,
            1,
            body.len() as u32,
            &body,
            0,
        );
        let parsed = crate::parse_chunk_frame(&frame).expect("chunk");
        assert_eq!(parsed.kind, kind::FOOTER);
        assert_eq!(parsed.codec, codec::NONE);
        let (recs, n) = decode_footer_body(parsed.payload).expect("body");
        assert_eq!(n, body.len());
        assert_eq!(recs[0].key_id, 9);
        assert_eq!(recs[0].label.data, b"end");
    }
}
