//! Provisional **format v6** SUMMARY chunk body (codec NONE) (COL-007 runway).
//!
//! Schema: `docs/schemas/v6-summary-body-provisional-v0.md`
//!
//! Ordered summary records: ULEB128 key_id + count + value + optional
//! length-prefixed string-blob label. Composes shipped varint + string-blob.
//! No inflate, no full summary catalog, no C writer.

use crate::string::{decode_string_blob, encode_string_blob, StringBlob, StringError};
use crate::varint::{decode_u64, encode_u64, VarintError};

/// Fail-closed upper bound on total SUMMARY body size (64 MiB).
pub const MAX_SUMMARY_BODY_BYTES: usize = 64 * 1024 * 1024;

/// One decoded SUMMARY record (label borrowed from input).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryRecord<'a> {
    /// Provisional key (e.g. sub id, fid, aggregate bucket).
    pub key_id: u64,
    /// Count (e.g. calls / events).
    pub count: u64,
    /// Ticks or other aggregate value (provisional).
    pub value: u64,
    /// Optional human label (may be empty).
    pub label: StringBlob<'a>,
}

/// Spec for encoding one SUMMARY record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SummaryRecordSpec<'a> {
    pub key_id: u64,
    pub count: u64,
    pub value: u64,
    pub string_id: u64,
    pub string_flags: u8,
    pub label: &'a [u8],
}

/// Fail-closed SUMMARY-body errors.
#[derive(Debug, PartialEq, Eq)]
pub enum SummaryBodyError {
    Varint(VarintError),
    String(StringError),
    Truncated { need: usize, got: usize },
    Oversize { len: usize },
}

impl std::fmt::Display for SummaryBodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SummaryBodyError::Varint(e) => write!(f, "summary-body varint: {e}"),
            SummaryBodyError::String(e) => write!(f, "summary-body string: {e}"),
            SummaryBodyError::Truncated { need, got } => {
                write!(f, "truncated summary-body: need {need} bytes, got {got}")
            }
            SummaryBodyError::Oversize { len } => {
                write!(
                    f,
                    "oversize summary-body {len} bytes (max {MAX_SUMMARY_BODY_BYTES})"
                )
            }
        }
    }
}

impl std::error::Error for SummaryBodyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SummaryBodyError::Varint(e) => Some(e),
            SummaryBodyError::String(e) => Some(e),
            _ => None,
        }
    }
}

impl From<VarintError> for SummaryBodyError {
    fn from(e: VarintError) -> Self {
        SummaryBodyError::Varint(e)
    }
}

impl From<StringError> for SummaryBodyError {
    fn from(e: StringError) -> Self {
        SummaryBodyError::String(e)
    }
}

pub type SummaryBodyResult<T> = std::result::Result<T, SummaryBodyError>;

/// Encode a provisional SUMMARY-body (codec NONE chunk payload).
///
/// Each record:
/// `ULEB128 key_id || ULEB128 count || ULEB128 value || string_blob(id, flags, label)`.
/// Empty `records` → empty body. Pure byte-slice / `Vec` API.
pub fn encode_summary_body(records: &[SummaryRecordSpec<'_>]) -> Vec<u8> {
    let mut out = Vec::new();
    for rec in records {
        out.extend_from_slice(&encode_u64(rec.key_id));
        out.extend_from_slice(&encode_u64(rec.count));
        out.extend_from_slice(&encode_u64(rec.value));
        out.extend_from_slice(&encode_string_blob(
            rec.string_id,
            rec.string_flags,
            rec.label,
        ));
    }
    out
}

/// Decode a provisional SUMMARY-body until the buffer is exhausted.
///
/// Empty input → empty list. Fail-closed on truncated mid-record or oversize.
/// Returns `(records, bytes_consumed)` (`bytes_consumed == data.len()` on success).
pub fn decode_summary_body(data: &[u8]) -> SummaryBodyResult<(Vec<SummaryRecord<'_>>, usize)> {
    if data.len() > MAX_SUMMARY_BODY_BYTES {
        return Err(SummaryBodyError::Oversize { len: data.len() });
    }
    let mut pos = 0;
    let mut out = Vec::new();
    while pos < data.len() {
        if pos > MAX_SUMMARY_BODY_BYTES {
            return Err(SummaryBodyError::Oversize { len: pos });
        }
        let (key_id, n1) = decode_u64(data, pos)?;
        pos += n1;
        let (count, n2) = decode_u64(data, pos)?;
        pos += n2;
        let (value, n3) = decode_u64(data, pos)?;
        pos += n3;
        let (label, n4) = decode_string_blob(data, pos)?;
        pos += n4;
        out.push(SummaryRecord {
            key_id,
            count,
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
    fn empty_summary_body_roundtrip() {
        let enc_a = encode_summary_body(&[]);
        let enc_b = encode_summary_body(&[]);
        assert_eq!(enc_a, enc_b);
        assert!(enc_a.is_empty());
        let (recs, n) = decode_summary_body(&enc_a).expect("empty");
        assert_eq!(n, 0);
        assert!(recs.is_empty());
    }

    #[test]
    fn summary_records_roundtrip() {
        let specs = [
            SummaryRecordSpec {
                key_id: 1,
                count: 15,
                value: 1000,
                string_id: 0,
                string_flags: FLAG_UTF8,
                label: b"main::leaf",
            },
            SummaryRecordSpec {
                key_id: 2,
                count: 3,
                value: 0,
                string_id: 0,
                string_flags: 0,
                label: b"",
            },
        ];
        let enc = encode_summary_body(&specs);
        let mut expect = Vec::new();
        expect.extend_from_slice(&encode_u64(1));
        expect.extend_from_slice(&encode_u64(15));
        expect.extend_from_slice(&encode_u64(1000));
        expect.extend_from_slice(&encode_string_blob(0, FLAG_UTF8, b"main::leaf"));
        expect.extend_from_slice(&encode_u64(2));
        expect.extend_from_slice(&encode_u64(3));
        expect.extend_from_slice(&encode_u64(0));
        expect.extend_from_slice(&encode_string_blob(0, 0, b""));
        assert_eq!(enc, expect);

        let (recs, n) = decode_summary_body(&enc).expect("roundtrip");
        assert_eq!(n, enc.len());
        assert_eq!(recs.len(), 2);
        assert_eq!(recs[0].key_id, 1);
        assert_eq!(recs[0].count, 15);
        assert_eq!(recs[0].value, 1000);
        assert_eq!(recs[0].label.data, b"main::leaf");
        assert_eq!(recs[1].count, 3);
        assert!(recs[1].label.data.is_empty());
    }

    #[test]
    fn truncated_mid_record_err() {
        let full = encode_summary_body(&[SummaryRecordSpec {
            key_id: 1,
            count: 2,
            value: 3,
            string_id: 0,
            string_flags: 0,
            label: b"hello",
        }]);
        let trunc = &full[..full.len() - 1];
        match decode_summary_body(trunc) {
            Err(SummaryBodyError::Varint(_))
            | Err(SummaryBodyError::String(_))
            | Err(SummaryBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated mid-record, got {other:?}"),
        }
    }

    #[test]
    fn truncated_after_key_err() {
        let partial = encode_u64(1);
        match decode_summary_body(&partial) {
            Err(SummaryBodyError::Varint(_)) | Err(SummaryBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated after key, got {other:?}"),
        }
    }

    #[test]
    fn never_panic_garbage() {
        assert!(decode_summary_body(&[]).is_ok());
        let _ = decode_summary_body(&[0xFF; 6]);
        let _ = decode_summary_body(b"\x01\x02");
    }

    #[test]
    fn summary_as_codec_none_chunk_payload() {
        let body = encode_summary_body(&[SummaryRecordSpec {
            key_id: 9,
            count: 4,
            value: 99,
            string_id: 0,
            string_flags: 0,
            label: b"x",
        }]);
        let frame = encode_chunk_frame(
            kind::SUMMARY,
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
        assert_eq!(parsed.kind, kind::SUMMARY);
        assert_eq!(parsed.codec, codec::NONE);
        let (recs, n) = decode_summary_body(parsed.payload).expect("body");
        assert_eq!(n, body.len());
        assert_eq!(recs[0].key_id, 9);
        assert_eq!(recs[0].label.data, b"x");
    }
}
