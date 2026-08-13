//! Provisional **format v6** header TLV frame + multi-TLV region (COL-007 runway).
//!
//! Schemas:
//! - `docs/schemas/v6-header-tlv-provisional-v0.md`
//! - `docs/schemas/v6-tlv-region-provisional-v0.md`
//!
//! Composes strict ULEB128 for type_id and value_length.

use crate::varint::{decode_u64, encode_u64, VarintError};

/// Fail-closed upper bound on declared TLV value payload (16 MiB).
pub const MAX_TLV_VALUE_BYTES: u64 = 16 * 1024 * 1024;

/// Fail-closed upper bound on total multi-TLV region size including terminator (64 MiB).
pub const MAX_TLV_REGION_BYTES: usize = 64 * 1024 * 1024;

/// Flag: unknown type_id must fail closed (required type).
pub const FLAG_TYPE_REQUIRED: u8 = 0x01;

/// Provisional known header TLV type ids.
pub mod type_id {
    pub const RESERVED: u64 = 0;
    pub const PRODUCER: u64 = 1;
    pub const TICKS_PER_SEC: u64 = 2;
    /// End-of-header-TLV-region terminator (empty value; not emitted as a payload TLV).
    pub const END: u64 = 0x7e;
}

/// Decoded TLV (value borrowed from input).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tlv<'a> {
    pub type_id: u64,
    pub flags: u8,
    /// True when `type_id` is a known provisional type (1..=2).
    pub known_type: bool,
    pub value: &'a [u8],
}

/// Fail-closed TLV errors.
#[derive(Debug, PartialEq, Eq)]
pub enum TlvError {
    Varint(VarintError),
    Truncated {
        need: usize,
        got: usize,
    },
    Oversize {
        len: u64,
    },
    InvalidType,
    UnknownRequiredType {
        type_id: u64,
    },
    /// Multi-TLV region exceeded `MAX_TLV_REGION_BYTES`.
    RegionOversize {
        len: usize,
    },
    /// Multi-TLV region ended without a terminator TLV.
    MissingTerminator,
    /// Terminator TLV must have empty value and no required-unknown semantics.
    InvalidTerminator,
}

impl std::fmt::Display for TlvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TlvError::Varint(e) => write!(f, "tlv varint: {e}"),
            TlvError::Truncated { need, got } => {
                write!(f, "truncated tlv: need {need} bytes, got {got}")
            }
            TlvError::Oversize { len } => {
                write!(
                    f,
                    "oversize tlv value_length {len} (max {MAX_TLV_VALUE_BYTES})"
                )
            }
            TlvError::InvalidType => write!(f, "invalid tlv type_id 0 (reserved)"),
            TlvError::UnknownRequiredType { type_id } => {
                write!(f, "unknown required tlv type_id {type_id}")
            }
            TlvError::RegionOversize { len } => {
                write!(
                    f,
                    "oversize tlv region {len} bytes (max {MAX_TLV_REGION_BYTES})"
                )
            }
            TlvError::MissingTerminator => write!(f, "tlv region missing END terminator"),
            TlvError::InvalidTerminator => {
                write!(f, "invalid END terminator TLV (must be empty value)")
            }
        }
    }
}

impl std::error::Error for TlvError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TlvError::Varint(e) => Some(e),
            _ => None,
        }
    }
}

impl From<VarintError> for TlvError {
    fn from(e: VarintError) -> Self {
        TlvError::Varint(e)
    }
}

pub type TlvResult<T> = std::result::Result<T, TlvError>;

/// True if `type_id` is a known provisional type (includes END terminator).
pub fn is_known_type(type_id: u64) -> bool {
    matches!(
        type_id,
        type_id::PRODUCER | type_id::TICKS_PER_SEC | type_id::END
    )
}

/// Canonical encode of one header TLV.
pub fn encode_tlv(type_id: u64, flags: u8, value: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(20 + value.len());
    out.extend_from_slice(&encode_u64(type_id));
    out.extend_from_slice(&encode_u64(value.len() as u64));
    out.push(flags);
    out.extend_from_slice(value);
    out
}

/// Decode one header TLV starting at `pos` (strict ULEB128).
///
/// Returns `(tlv, bytes_consumed)`. Fail-closed on truncated input, oversize
/// length, reserved type 0, and unknown type when `FLAG_TYPE_REQUIRED` is set.
pub fn decode_tlv(data: &[u8], pos: usize) -> TlvResult<(Tlv<'_>, usize)> {
    let start = pos;
    let (tid, n_tid) = decode_u64(data, pos)?;
    let mut p = pos + n_tid;

    let (value_len, n_len) = decode_u64(data, p)?;
    p += n_len;

    if tid == type_id::RESERVED {
        return Err(TlvError::InvalidType);
    }
    if value_len > MAX_TLV_VALUE_BYTES {
        return Err(TlvError::Oversize { len: value_len });
    }

    if p >= data.len() {
        return Err(TlvError::Truncated {
            need: p + 1,
            got: data.len(),
        });
    }
    let flags = data[p];
    p += 1;

    let known = is_known_type(tid);
    if !known && (flags & FLAG_TYPE_REQUIRED) != 0 {
        return Err(TlvError::UnknownRequiredType { type_id: tid });
    }

    let need = p
        .checked_add(value_len as usize)
        .ok_or(TlvError::Oversize { len: value_len })?;
    if data.len() < need {
        return Err(TlvError::Truncated {
            need,
            got: data.len(),
        });
    }

    let value = &data[p..need];
    Ok((
        Tlv {
            type_id: tid,
            flags,
            known_type: known,
            value,
        },
        need - start,
    ))
}

/// Encode a multi-TLV header region ending with an END terminator.
///
/// `items` are payload TLVs only (must not include `type_id::END` or reserved 0).
/// Each item is `(type_id, flags, value)`.
pub fn encode_tlv_region(items: &[(u64, u8, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    for &(tid, flags, value) in items {
        out.extend_from_slice(&encode_tlv(tid, flags, value));
    }
    // Terminator: type END, empty value, flags 0.
    out.extend_from_slice(&encode_tlv(type_id::END, 0, b""));
    out
}

/// Decode a multi-TLV header region starting at `pos` until END terminator.
///
/// Returns `(payload_tlvs, bytes_consumed)` where `payload_tlvs` excludes the
/// END terminator. Fail-closed on truncated mid-TLV, missing terminator,
/// oversize region, invalid terminator value, and unknown required types
/// (via single-TLV rules).
pub fn decode_tlv_region(data: &[u8], pos: usize) -> TlvResult<(Vec<Tlv<'_>>, usize)> {
    let start = pos;
    let mut p = pos;
    let mut out = Vec::new();

    loop {
        if p >= data.len() {
            return Err(TlvError::MissingTerminator);
        }
        if p.saturating_sub(start) > MAX_TLV_REGION_BYTES {
            return Err(TlvError::RegionOversize {
                len: p.saturating_sub(start),
            });
        }

        let (tlv, n) = decode_tlv(data, p)?;
        p += n;

        if p.saturating_sub(start) > MAX_TLV_REGION_BYTES {
            return Err(TlvError::RegionOversize {
                len: p.saturating_sub(start),
            });
        }

        if tlv.type_id == type_id::END {
            if !tlv.value.is_empty() {
                return Err(TlvError::InvalidTerminator);
            }
            // END terminates the region; not included in payload list.
            return Ok((out, p - start));
        }

        out.push(tlv);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_empty_producer() {
        let enc = encode_tlv(type_id::PRODUCER, 0, b"");
        let (tlv, n) = decode_tlv(&enc, 0).expect("empty");
        assert_eq!(n, enc.len());
        assert_eq!(tlv.type_id, type_id::PRODUCER);
        assert!(tlv.known_type);
        assert_eq!(tlv.value, b"");
    }

    #[test]
    fn roundtrip_non_empty_producer() {
        let enc = encode_tlv(type_id::PRODUCER, 0, b"nytprof-rust");
        let (tlv, n) = decode_tlv(&enc, 0).unwrap();
        assert_eq!(n, enc.len());
        assert_eq!(tlv.type_id, type_id::PRODUCER);
        assert_eq!(tlv.value, b"nytprof-rust");
    }

    #[test]
    fn roundtrip_ticks_per_sec_opaque() {
        let enc = encode_tlv(type_id::TICKS_PER_SEC, 0, b"10000000");
        let (tlv, n) = decode_tlv(&enc, 0).unwrap();
        assert_eq!(n, enc.len());
        assert_eq!(tlv.type_id, type_id::TICKS_PER_SEC);
        assert!(tlv.known_type);
        assert_eq!(tlv.value, b"10000000");
    }

    #[test]
    fn trailing_bytes_ok() {
        let mut enc = encode_tlv(type_id::PRODUCER, 0, b"x");
        enc.push(0xAA);
        let (tlv, n) = decode_tlv(&enc, 0).unwrap();
        assert_eq!(tlv.value, b"x");
        assert_eq!(enc[n], 0xAA);
    }

    #[test]
    fn truncated_empty_err() {
        assert!(matches!(
            decode_tlv(&[], 0),
            Err(TlvError::Varint(VarintError::Truncated { .. }))
        ));
    }

    #[test]
    fn truncated_payload_err() {
        let mut enc = encode_tlv(type_id::PRODUCER, 0, b"abcd");
        enc.truncate(enc.len() - 3);
        match decode_tlv(&enc, 0) {
            Err(TlvError::Truncated { need, got }) => assert!(need > got),
            other => panic!("expected Truncated, got {other:?}"),
        }
    }

    #[test]
    fn oversize_err() {
        let mut buf = encode_u64(type_id::PRODUCER);
        buf.extend_from_slice(&encode_u64(MAX_TLV_VALUE_BYTES + 1));
        buf.push(0);
        assert_eq!(
            decode_tlv(&buf, 0),
            Err(TlvError::Oversize {
                len: MAX_TLV_VALUE_BYTES + 1
            })
        );
    }

    #[test]
    fn reserved_type_zero_err() {
        let enc = encode_tlv(0, 0, b"");
        assert_eq!(decode_tlv(&enc, 0), Err(TlvError::InvalidType));
    }

    #[test]
    fn unknown_required_type_err() {
        let enc = encode_tlv(99, FLAG_TYPE_REQUIRED, b"");
        assert_eq!(
            decode_tlv(&enc, 0),
            Err(TlvError::UnknownRequiredType { type_id: 99 })
        );
    }

    #[test]
    fn unknown_optional_type_ok() {
        let enc = encode_tlv(99, 0, b"skip-me");
        let (tlv, n) = decode_tlv(&enc, 0).expect("optional unknown");
        assert_eq!(n, enc.len());
        assert!(!tlv.known_type);
        assert_eq!(tlv.type_id, 99);
        assert_eq!(tlv.value, b"skip-me");
    }

    #[test]
    fn max_constant() {
        assert_eq!(MAX_TLV_VALUE_BYTES, 16 * 1024 * 1024);
    }

    // --- multi-TLV region ---

    #[test]
    fn region_empty_terminator_only() {
        let enc = encode_tlv_region(&[]);
        let (tlvs, n) = decode_tlv_region(&enc, 0).expect("empty region");
        assert_eq!(n, enc.len());
        assert!(tlvs.is_empty());
        // Wire is exactly one END TLV.
        let (end, n_end) = decode_tlv(&enc, 0).unwrap();
        assert_eq!(end.type_id, type_id::END);
        assert_eq!(n_end, enc.len());
    }

    #[test]
    fn region_two_tlvs_roundtrip() {
        let items: [(u64, u8, &[u8]); 2] = [
            (type_id::PRODUCER, 0, b"nytprof-rust"),
            (type_id::TICKS_PER_SEC, 0, b"10000000"),
        ];
        let enc = encode_tlv_region(&items);
        let (tlvs, n) = decode_tlv_region(&enc, 0).expect("region");
        assert_eq!(n, enc.len());
        assert_eq!(tlvs.len(), 2);
        assert_eq!(tlvs[0].type_id, type_id::PRODUCER);
        assert_eq!(tlvs[0].value, b"nytprof-rust");
        assert_eq!(tlvs[1].type_id, type_id::TICKS_PER_SEC);
        assert_eq!(tlvs[1].value, b"10000000");
    }

    #[test]
    fn region_trailing_bytes_ok() {
        let mut enc = encode_tlv_region(&[(type_id::PRODUCER, 0, b"x")]);
        enc.push(0xBB);
        let (tlvs, n) = decode_tlv_region(&enc, 0).unwrap();
        assert_eq!(tlvs.len(), 1);
        assert_eq!(enc[n], 0xBB);
    }

    #[test]
    fn region_truncated_mid_tlv_err() {
        let mut enc = encode_tlv_region(&[(type_id::PRODUCER, 0, b"abcd")]);
        // Drop terminator and part of producer payload.
        let end = encode_tlv(type_id::END, 0, b"");
        assert!(enc.ends_with(&end));
        enc.truncate(enc.len() - end.len() - 2);
        match decode_tlv_region(&enc, 0) {
            Err(TlvError::Truncated { .. }) | Err(TlvError::Varint(_)) => {}
            other => panic!("expected truncated mid-tlv, got {other:?}"),
        }
    }

    #[test]
    fn region_missing_terminator_err() {
        // Single producer TLV without END.
        let enc = encode_tlv(type_id::PRODUCER, 0, b"hi");
        assert_eq!(decode_tlv_region(&enc, 0), Err(TlvError::MissingTerminator));
    }

    #[test]
    fn region_unknown_required_err() {
        let enc = encode_tlv_region(&[(99, FLAG_TYPE_REQUIRED, b"")]);
        assert_eq!(
            decode_tlv_region(&enc, 0),
            Err(TlvError::UnknownRequiredType { type_id: 99 })
        );
    }

    #[test]
    fn region_unknown_optional_ok() {
        let enc = encode_tlv_region(&[(99, 0, b"skip")]);
        let (tlvs, n) = decode_tlv_region(&enc, 0).expect("optional unknown in region");
        assert_eq!(n, enc.len());
        assert_eq!(tlvs.len(), 1);
        assert!(!tlvs[0].known_type);
        assert_eq!(tlvs[0].value, b"skip");
    }

    #[test]
    fn region_invalid_terminator_nonempty_err() {
        // Manually craft END with non-empty value.
        let mut enc = encode_tlv(type_id::PRODUCER, 0, b"a");
        enc.extend_from_slice(&encode_tlv(type_id::END, 0, b"x"));
        assert_eq!(decode_tlv_region(&enc, 0), Err(TlvError::InvalidTerminator));
    }

    #[test]
    fn end_is_known_type() {
        assert!(is_known_type(type_id::END));
    }
}
