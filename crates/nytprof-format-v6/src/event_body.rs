//! Provisional **format v6** event-body opcode codec (COL-007 runway).
//!
//! Schema: `docs/schemas/v6-event-body-provisional-v0.md`
//!
//! Codec **NONE** chunk payloads: ordered records with ULEB128 opcodes + typed
//! fields composed from shipped varint / string-blob primitives.
//! Does **not** inflate zlib/zstd/LZ4, implement full v5 tag parity, or the C writer.

use crate::string::{decode_string_blob, encode_string_blob, StringBlob, StringError};
use crate::varint::{decode_u64, encode_u64, VarintError};

/// Fail-closed upper bound on total event-body size (64 MiB).
pub const MAX_EVENT_BODY_BYTES: usize = 64 * 1024 * 1024;

/// Flag: unknown opcode must fail closed (required opcode).
pub const FLAG_OPCODE_REQUIRED: u8 = 0x01;

/// Provisional event opcodes.
pub mod opcode {
    /// Reserved — always fail closed.
    pub const RESERVED: u64 = 0;
    /// Metadata mark: body is a length-prefixed string/blob.
    pub const MARK: u64 = 1;
    /// Timing-like sample: fid, line, ticks as three ULEB128 u64 fields.
    pub const TIME_LINE: u64 = 2;
}

/// True if `opcode` is a known provisional type (excludes RESERVED).
pub fn is_known_opcode(opcode: u64) -> bool {
    matches!(opcode, opcode::MARK | opcode::TIME_LINE)
}

/// One decoded event-body record (payloads borrowed from input).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventRecord<'a> {
    /// `opcode::MARK` — string/blob label (id/flags from string-blob frame).
    Mark { label: StringBlob<'a> },
    /// `opcode::TIME_LINE` — fid / line / ticks.
    TimeLine { fid: u64, line: u64, ticks: u64 },
}

/// Spec for encoding one event-body record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventRecordSpec<'a> {
    /// MARK with string-blob fields (composes `encode_string_blob`).
    Mark {
        string_id: u64,
        string_flags: u8,
        label: &'a [u8],
    },
    /// TIME_LINE sample.
    TimeLine { fid: u64, line: u64, ticks: u64 },
}

/// Fail-closed event-body errors (never panic on crafted input).
#[derive(Debug, PartialEq, Eq)]
pub enum EventBodyError {
    Varint(VarintError),
    String(StringError),
    Truncated { need: usize, got: usize },
    Oversize { len: usize },
    /// Opcode 0 is reserved.
    ReservedOpcode,
    /// Unknown opcode with `FLAG_OPCODE_REQUIRED` set.
    UnknownRequiredOpcode { opcode: u64 },
    /// Unknown opcode without required flag — still fail closed in this MVP
    /// (bodies of unknown opcodes are not length-prefixed for skip).
    UnknownOpcode { opcode: u64 },
}

impl std::fmt::Display for EventBodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventBodyError::Varint(e) => write!(f, "event-body varint: {e}"),
            EventBodyError::String(e) => write!(f, "event-body string: {e}"),
            EventBodyError::Truncated { need, got } => {
                write!(f, "truncated event-body: need {need} bytes, got {got}")
            }
            EventBodyError::Oversize { len } => {
                write!(
                    f,
                    "oversize event-body {len} bytes (max {MAX_EVENT_BODY_BYTES})"
                )
            }
            EventBodyError::ReservedOpcode => write!(f, "reserved event opcode 0"),
            EventBodyError::UnknownRequiredOpcode { opcode } => {
                write!(f, "unknown required event opcode {opcode}")
            }
            EventBodyError::UnknownOpcode { opcode } => {
                write!(f, "unknown event opcode {opcode}")
            }
        }
    }
}

impl std::error::Error for EventBodyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EventBodyError::Varint(e) => Some(e),
            EventBodyError::String(e) => Some(e),
            _ => None,
        }
    }
}

impl From<VarintError> for EventBodyError {
    fn from(e: VarintError) -> Self {
        EventBodyError::Varint(e)
    }
}

impl From<StringError> for EventBodyError {
    fn from(e: StringError) -> Self {
        EventBodyError::String(e)
    }
}

pub type EventBodyResult<T> = std::result::Result<T, EventBodyError>;

/// Encode one record (opcode ULEB + flags + typed body) into `out`.
fn encode_record_into(out: &mut Vec<u8>, rec: &EventRecordSpec<'_>) {
    match rec {
        EventRecordSpec::Mark {
            string_id,
            string_flags,
            label,
        } => {
            out.extend_from_slice(&encode_u64(opcode::MARK));
            out.push(0); // flags
            out.extend_from_slice(&encode_string_blob(*string_id, *string_flags, label));
        }
        EventRecordSpec::TimeLine { fid, line, ticks } => {
            out.extend_from_slice(&encode_u64(opcode::TIME_LINE));
            out.push(0); // flags
            out.extend_from_slice(&encode_u64(*fid));
            out.extend_from_slice(&encode_u64(*line));
            out.extend_from_slice(&encode_u64(*ticks));
        }
    }
}

/// Encode a provisional event-body (codec NONE payload): ordered records.
///
/// Empty `records` yields an empty body (valid). Pure byte-slice / `Vec` API.
pub fn encode_event_body(records: &[EventRecordSpec<'_>]) -> Vec<u8> {
    let mut out = Vec::new();
    for rec in records {
        encode_record_into(&mut out, rec);
    }
    out
}

/// Decode one record starting at `pos`. Returns `(record, bytes_consumed)`.
fn decode_record<'a>(data: &'a [u8], pos: usize) -> EventBodyResult<(EventRecord<'a>, usize)> {
    if pos >= data.len() {
        return Err(EventBodyError::Truncated {
            need: pos + 1,
            got: data.len(),
        });
    }
    let (op, n_op) = decode_u64(data, pos)?;
    let mut p = pos + n_op;

    // flags byte required after opcode
    if p >= data.len() {
        return Err(EventBodyError::Truncated {
            need: p + 1,
            got: data.len(),
        });
    }
    let flags = data[p];
    p += 1;

    if op == opcode::RESERVED {
        return Err(EventBodyError::ReservedOpcode);
    }

    if !is_known_opcode(op) {
        if (flags & FLAG_OPCODE_REQUIRED) != 0 {
            return Err(EventBodyError::UnknownRequiredOpcode { opcode: op });
        }
        return Err(EventBodyError::UnknownOpcode { opcode: op });
    }

    match op {
        opcode::MARK => {
            let (label, n_str) = decode_string_blob(data, p)?;
            p += n_str;
            Ok((EventRecord::Mark { label }, p - pos))
        }
        opcode::TIME_LINE => {
            let (fid, n1) = decode_u64(data, p)?;
            p += n1;
            let (line, n2) = decode_u64(data, p)?;
            p += n2;
            let (ticks, n3) = decode_u64(data, p)?;
            p += n3;
            Ok((EventRecord::TimeLine { fid, line, ticks }, p - pos))
        }
        _ => unreachable!("is_known_opcode filtered"),
    }
}

/// Decode a provisional event-body until the buffer is exhausted.
///
/// Empty input → empty record list. Fail-closed on truncated mid-record,
/// reserved opcode 0, unknown opcode (required or not — MVP cannot skip).
/// Returns `(records, bytes_consumed)` (`bytes_consumed == data.len()` on success).
pub fn decode_event_body(data: &[u8]) -> EventBodyResult<(Vec<EventRecord<'_>>, usize)> {
    if data.len() > MAX_EVENT_BODY_BYTES {
        return Err(EventBodyError::Oversize { len: data.len() });
    }
    let mut pos = 0;
    let mut out = Vec::new();
    while pos < data.len() {
        if pos > MAX_EVENT_BODY_BYTES {
            return Err(EventBodyError::Oversize { len: pos });
        }
        let (rec, n) = decode_record(data, pos)?;
        pos += n;
        out.push(rec);
    }
    Ok((out, pos))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{codec, encode_chunk_frame, kind, parse_chunk_frame};
    use crate::string::FLAG_UTF8;

    #[test]
    fn empty_body_roundtrip() {
        let enc_a = encode_event_body(&[]);
        let enc_b = encode_event_body(&[]);
        assert_eq!(enc_a, enc_b);
        assert!(enc_a.is_empty());
        let (recs, n) = decode_event_body(&enc_a).expect("empty");
        assert_eq!(n, 0);
        assert!(recs.is_empty());
        // Dual decode stability.
        let (recs2, n2) = decode_event_body(&enc_a).unwrap();
        assert_eq!(n2, n);
        assert_eq!(recs2, recs);
    }

    #[test]
    fn mark_and_time_line_roundtrip() {
        let specs = [
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: FLAG_UTF8,
                label: b"main::leaf",
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 5,
                ticks: 42,
            },
        ];
        let enc = encode_event_body(&specs);
        // Length must equal encode of parts (no detached golden).
        let mut expect = Vec::new();
        encode_record_into(&mut expect, &specs[0]);
        encode_record_into(&mut expect, &specs[1]);
        assert_eq!(enc, expect);

        let (recs, n) = decode_event_body(&enc).expect("roundtrip");
        assert_eq!(n, enc.len());
        assert_eq!(recs.len(), 2);
        match &recs[0] {
            EventRecord::Mark { label } => {
                assert_eq!(label.id, 0);
                assert_eq!(label.flags, FLAG_UTF8);
                assert_eq!(label.data, b"main::leaf");
            }
            other => panic!("expected Mark, got {other:?}"),
        }
        match &recs[1] {
            EventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!(*fid, 1);
                assert_eq!(*line, 5);
                assert_eq!(*ticks, 42);
            }
            other => panic!("expected TimeLine, got {other:?}"),
        }
    }

    #[test]
    fn reserved_opcode_zero_err() {
        // Craft opcode 0 + flags 0 manually via encode_u64.
        let mut bad = encode_u64(opcode::RESERVED);
        bad.push(0);
        assert_eq!(
            decode_event_body(&bad),
            Err(EventBodyError::ReservedOpcode)
        );
    }

    #[test]
    fn unknown_required_opcode_err() {
        let mut bad = encode_u64(99);
        bad.push(FLAG_OPCODE_REQUIRED);
        assert_eq!(
            decode_event_body(&bad),
            Err(EventBodyError::UnknownRequiredOpcode { opcode: 99 })
        );
    }

    #[test]
    fn unknown_optional_opcode_still_err_mvp() {
        // Without required flag, MVP still fails closed (cannot skip unknown body).
        let mut bad = encode_u64(99);
        bad.push(0);
        assert_eq!(
            decode_event_body(&bad),
            Err(EventBodyError::UnknownOpcode { opcode: 99 })
        );
    }

    #[test]
    fn truncated_mid_time_line_err() {
        let full = encode_event_body(&[EventRecordSpec::TimeLine {
            fid: 1,
            line: 2,
            ticks: 3,
        }]);
        // Drop last byte of last varint.
        assert!(full.len() > 3);
        let trunc = &full[..full.len() - 1];
        match decode_event_body(trunc) {
            Err(EventBodyError::Varint(_)) | Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated mid-record, got {other:?}"),
        }
    }

    #[test]
    fn truncated_after_opcode_before_flags_err() {
        let mut partial = encode_u64(opcode::TIME_LINE);
        // no flags byte
        match decode_event_body(&partial) {
            Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated flags, got {other:?}"),
        }
        // flags present but no body fields
        partial.push(0);
        match decode_event_body(&partial) {
            Err(EventBodyError::Varint(_)) | Err(EventBodyError::Truncated { .. }) => {}
            other => panic!("expected truncated body, got {other:?}"),
        }
    }

    #[test]
    fn never_panic_on_garbage() {
        assert!(decode_event_body(&[]).is_ok());
        let _ = decode_event_body(&[0xFF; 8]);
        let _ = decode_event_body(b"\x01"); // MARK opcode incomplete
    }

    #[test]
    fn codec_none_chunk_payload_is_event_body() {
        // Optional composition smoke: EVENT chunk + codec NONE carries event-body bytes.
        let body = encode_event_body(&[
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 5,
                ticks: 100,
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"x",
            },
        ]);
        let frame = encode_chunk_frame(
            kind::EVENT,
            codec::NONE,
            0,
            0,
            0,
            2,
            body.len() as u32,
            &body,
            0,
        );
        let parsed = parse_chunk_frame(&frame).expect("chunk");
        assert_eq!(parsed.codec, codec::NONE);
        assert_eq!(parsed.payload, body.as_slice());
        let (recs, n) = decode_event_body(parsed.payload).expect("body from chunk");
        assert_eq!(n, body.len());
        assert_eq!(recs.len(), 2);
        match &recs[0] {
            EventRecord::TimeLine { fid, line, ticks } => {
                assert_eq!((*fid, *line, *ticks), (1, 5, 100));
            }
            other => panic!("expected TimeLine, got {other:?}"),
        }
    }
}
