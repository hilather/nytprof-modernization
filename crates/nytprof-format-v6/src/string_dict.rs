//! Provisional **format v6** string-dictionary intern table (COL-007 runway).
//!
//! Schema: `docs/schemas/v6-string-dictionary-provisional-v0.md`
//!
//! Maps non-zero `string_id` values to owned byte payloads for intern resolution
//! of length-prefixed string-blobs. Does **not** implement a permanent global
//! string pool, cross-file identity, or wire freeze.

use crate::compressed_profile::OwnedEventRecord;
use crate::event_body::EventRecord;
use crate::string::{StringBlob, MAX_STRING_BYTES};
use crate::varint::{decode_u64, encode_u64, VarintError};
use std::collections::BTreeMap;

/// Fail-closed upper bound on dictionary entry count.
pub const MAX_DICT_ENTRIES: u64 = 1_048_576;

/// Fail-closed upper bound on total dictionary payload bytes (all entries).
pub const MAX_DICT_TOTAL_BYTES: usize = 64 * 1024 * 1024;

/// One dictionary entry (owned payload).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictEntry {
    pub flags: u8,
    pub data: Vec<u8>,
}

/// Provisional string dictionary: non-zero `string_id` → entry.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StringDictionary {
    entries: BTreeMap<u64, DictEntry>,
}

impl StringDictionary {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Insert or replace entry for `id` (must be non-zero).
    pub fn insert(&mut self, id: u64, flags: u8, data: Vec<u8>) -> StringDictResult<()> {
        if id == 0 {
            return Err(StringDictError::ReservedIdZero);
        }
        if data.len() as u64 > MAX_STRING_BYTES {
            return Err(StringDictError::OversizeEntry {
                id,
                len: data.len() as u64,
            });
        }
        self.entries.insert(id, DictEntry { flags, data });
        Ok(())
    }

    pub fn get(&self, id: u64) -> Option<&DictEntry> {
        self.entries.get(&id)
    }

    pub fn contains(&self, id: u64) -> bool {
        self.entries.contains_key(&id)
    }

    /// Resolve blob to owned bytes (`id == 0` → copy inline; non-zero → dictionary payload).
    pub fn resolve_to_owned(&self, blob: &StringBlob<'_>) -> StringDictResult<Vec<u8>> {
        if blob.id == 0 {
            return Ok(blob.data.to_vec());
        }
        self.entries
            .get(&blob.id)
            .map(|e| e.data.clone())
            .ok_or(StringDictError::UnknownId { id: blob.id })
    }

    pub fn iter(&self) -> impl Iterator<Item = (u64, &DictEntry)> {
        self.entries.iter().map(|(k, v)| (*k, v))
    }
}

/// Fail-closed dictionary errors.
#[derive(Debug, PartialEq, Eq)]
pub enum StringDictError {
    Varint(VarintError),
    Truncated { need: usize, got: usize },
    Oversize { len: u64 },
    OversizeEntry { id: u64, len: u64 },
    OversizeTotal { len: usize },
    ReservedIdZero,
    DuplicateId { id: u64 },
    UnknownId { id: u64 },
}

impl std::fmt::Display for StringDictError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StringDictError::Varint(e) => write!(f, "string-dict varint: {e}"),
            StringDictError::Truncated { need, got } => {
                write!(f, "truncated string-dict: need {need} bytes, got {got}")
            }
            StringDictError::Oversize { len } => {
                write!(
                    f,
                    "oversize string-dict entry_count {len} (max {MAX_DICT_ENTRIES})"
                )
            }
            StringDictError::OversizeEntry { id, len } => {
                write!(
                    f,
                    "oversize string-dict entry id={id} len={len} (max {MAX_STRING_BYTES})"
                )
            }
            StringDictError::OversizeTotal { len } => {
                write!(
                    f,
                    "oversize string-dict total payload {len} (max {MAX_DICT_TOTAL_BYTES})"
                )
            }
            StringDictError::ReservedIdZero => {
                write!(f, "string-dict id 0 is reserved for inline-only blobs")
            }
            StringDictError::DuplicateId { id } => {
                write!(f, "duplicate string-dict id {id}")
            }
            StringDictError::UnknownId { id } => {
                write!(f, "unknown string-dict id {id}")
            }
        }
    }
}

impl std::error::Error for StringDictError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StringDictError::Varint(e) => Some(e),
            _ => None,
        }
    }
}

impl From<VarintError> for StringDictError {
    fn from(e: VarintError) -> Self {
        StringDictError::Varint(e)
    }
}

pub type StringDictResult<T> = std::result::Result<T, StringDictError>;

/// Encode a provisional dictionary table.
///
/// ```text
/// entry_count : ULEB128
/// entry*      : id ULEB128 || flags u8 || byte_length ULEB128 || bytes
/// ```
///
/// Each `id` must be non-zero and unique in `entries`.
pub fn encode_string_dictionary(entries: &[(u64, u8, &[u8])]) -> StringDictResult<Vec<u8>> {
    if entries.len() as u64 > MAX_DICT_ENTRIES {
        return Err(StringDictError::Oversize {
            len: entries.len() as u64,
        });
    }
    let mut seen = BTreeMap::new();
    let mut total = 0usize;
    for (id, _flags, data) in entries {
        if *id == 0 {
            return Err(StringDictError::ReservedIdZero);
        }
        if seen.insert(*id, ()).is_some() {
            return Err(StringDictError::DuplicateId { id: *id });
        }
        if data.len() as u64 > MAX_STRING_BYTES {
            return Err(StringDictError::OversizeEntry {
                id: *id,
                len: data.len() as u64,
            });
        }
        total = total
            .checked_add(data.len())
            .ok_or(StringDictError::OversizeTotal { len: usize::MAX })?;
        if total > MAX_DICT_TOTAL_BYTES {
            return Err(StringDictError::OversizeTotal { len: total });
        }
    }

    let mut out = encode_u64(entries.len() as u64);
    for (id, flags, data) in entries {
        out.extend_from_slice(&encode_u64(*id));
        out.push(*flags);
        out.extend_from_slice(&encode_u64(data.len() as u64));
        out.extend_from_slice(data);
    }
    Ok(out)
}

/// Decode a provisional dictionary table. Returns `(dict, bytes_consumed)`.
pub fn decode_string_dictionary(data: &[u8]) -> StringDictResult<(StringDictionary, usize)> {
    if data.is_empty() {
        return Err(StringDictError::Truncated { need: 1, got: 0 });
    }
    let (count, n_count) = decode_u64(data, 0)?;
    if count > MAX_DICT_ENTRIES {
        return Err(StringDictError::Oversize { len: count });
    }
    let mut p = n_count;
    let mut dict = StringDictionary::new();
    let mut total = 0usize;

    for _ in 0..count {
        let (id, n_id) = decode_u64(data, p)?;
        p += n_id;
        if id == 0 {
            return Err(StringDictError::ReservedIdZero);
        }
        if p >= data.len() {
            return Err(StringDictError::Truncated {
                need: p + 1,
                got: data.len(),
            });
        }
        let flags = data[p];
        p += 1;
        let (byte_len, n_len) = decode_u64(data, p)?;
        p += n_len;
        if byte_len > MAX_STRING_BYTES {
            return Err(StringDictError::OversizeEntry { id, len: byte_len });
        }
        let need = p
            .checked_add(byte_len as usize)
            .ok_or(StringDictError::OversizeEntry { id, len: byte_len })?;
        if data.len() < need {
            return Err(StringDictError::Truncated {
                need,
                got: data.len(),
            });
        }
        total = total
            .checked_add(byte_len as usize)
            .ok_or(StringDictError::OversizeTotal { len: usize::MAX })?;
        if total > MAX_DICT_TOTAL_BYTES {
            return Err(StringDictError::OversizeTotal { len: total });
        }
        if dict.entries.contains_key(&id) {
            return Err(StringDictError::DuplicateId { id });
        }
        dict.entries.insert(
            id,
            DictEntry {
                flags,
                data: data[p..need].to_vec(),
            },
        );
        p = need;
    }
    Ok((dict, p))
}

/// Build owned event record with string fields resolved via dictionary.
///
/// `string_id == 0` keeps inline blob bytes; non-zero ids must exist in `dict`.
pub fn owned_event_from_borrowed_resolved(
    r: &EventRecord<'_>,
    dict: &StringDictionary,
) -> StringDictResult<OwnedEventRecord> {
    Ok(match r {
        EventRecord::Mark { label } => OwnedEventRecord::Mark {
            label: dict.resolve_to_owned(label)?,
        },
        EventRecord::TimeLine { fid, line, ticks } => OwnedEventRecord::TimeLine {
            fid: *fid,
            line: *line,
            ticks: *ticks,
        },
        EventRecord::TimeBlock {
            fid,
            line,
            block_line,
            ticks,
        } => OwnedEventRecord::TimeBlock {
            fid: *fid,
            line: *line,
            block_line: *block_line,
            ticks: *ticks,
        },
        EventRecord::SubEntry {
            caller_fid,
            caller_line,
        } => OwnedEventRecord::SubEntry {
            caller_fid: *caller_fid,
            caller_line: *caller_line,
        },
        EventRecord::SubReturn {
            depth,
            incl,
            excl,
            subname,
        } => OwnedEventRecord::SubReturn {
            depth: *depth,
            incl: *incl,
            excl: *excl,
            subname: dict.resolve_to_owned(subname)?,
        },
        EventRecord::SubInfo {
            fid,
            first_line,
            last_line,
            name,
        } => OwnedEventRecord::SubInfo {
            fid: *fid,
            first_line: *first_line,
            last_line: *last_line,
            name: dict.resolve_to_owned(name)?,
        },
        EventRecord::SrcLine { fid, line, text } => OwnedEventRecord::SrcLine {
            fid: *fid,
            line: *line,
            text: dict.resolve_to_owned(text)?,
        },
        EventRecord::NewFid { fid, filename } => OwnedEventRecord::NewFid {
            fid: *fid,
            filename: dict.resolve_to_owned(filename)?,
        },
        EventRecord::PidStart {
            pid,
            ppid,
            start_time,
        } => OwnedEventRecord::PidStart {
            pid: *pid,
            ppid: *ppid,
            start_time: *start_time,
        },
        EventRecord::PidEnd { pid, end_time } => OwnedEventRecord::PidEnd {
            pid: *pid,
            end_time: *end_time,
        },
        EventRecord::SubCallers {
            fid,
            line,
            count,
            incl,
            excl,
            reci,
            rec_depth,
            called,
            caller,
        } => OwnedEventRecord::SubCallers {
            fid: *fid,
            line: *line,
            count: *count,
            incl: *incl,
            excl: *excl,
            reci: *reci,
            rec_depth: *rec_depth,
            called: dict.resolve_to_owned(called)?,
            caller: dict.resolve_to_owned(caller)?,
        },
        EventRecord::Discount => OwnedEventRecord::Discount,
        EventRecord::Attribute { key, value } => OwnedEventRecord::Attribute {
            key: dict.resolve_to_owned(key)?,
            value: dict.resolve_to_owned(value)?,
        },
        EventRecord::Option { key, value } => OwnedEventRecord::Option {
            key: dict.resolve_to_owned(key)?,
            value: dict.resolve_to_owned(value)?,
        },
        EventRecord::Comment { text } => OwnedEventRecord::Comment {
            text: dict.resolve_to_owned(text)?,
        },
        EventRecord::StartDeflate => OwnedEventRecord::StartDeflate,
        EventRecord::Version { major, minor } => OwnedEventRecord::Version {
            major: *major,
            minor: *minor,
        },
    })
}

/// Resolve all records in a borrowed event-body decode against `dict`.
pub fn resolve_event_records(
    recs: &[EventRecord<'_>],
    dict: &StringDictionary,
) -> StringDictResult<Vec<OwnedEventRecord>> {
    let mut out = Vec::with_capacity(recs.len());
    for r in recs {
        out.push(owned_event_from_borrowed_resolved(r, dict)?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_body::{decode_event_body, encode_event_body, EventRecordSpec};
    use crate::string::FLAG_UTF8;

    #[test]
    fn dictionary_roundtrip_two_entries() {
        let wire = encode_string_dictionary(&[(1, FLAG_UTF8, b"hello"), (2, 0, b"world")])
            .expect("encode");
        let (dict, n) = decode_string_dictionary(&wire).expect("decode");
        assert_eq!(n, wire.len());
        assert_eq!(dict.len(), 2);
        assert_eq!(dict.get(1).unwrap().data, b"hello");
        assert_eq!(dict.get(1).unwrap().flags, FLAG_UTF8);
        assert_eq!(dict.get(2).unwrap().data, b"world");
    }

    #[test]
    fn resolve_nonzero_id_from_dict() {
        let mut dict = StringDictionary::new();
        dict.insert(7, 0, b"interned-mark".to_vec()).unwrap();
        let blob = StringBlob {
            id: 7,
            flags: 0,
            data: b"", // empty inline intern ref
        };
        assert_eq!(dict.resolve_to_owned(&blob).unwrap(), b"interned-mark");
    }

    #[test]
    fn resolve_id_zero_uses_inline() {
        let dict = StringDictionary::new();
        let blob = StringBlob {
            id: 0,
            flags: 0,
            data: b"inline-only",
        };
        assert_eq!(dict.resolve_to_owned(&blob).unwrap(), b"inline-only");
    }

    #[test]
    fn unknown_id_fail_closed() {
        let dict = StringDictionary::new();
        let blob = StringBlob {
            id: 99,
            flags: 0,
            data: b"",
        };
        assert_eq!(
            dict.resolve_to_owned(&blob),
            Err(StringDictError::UnknownId { id: 99 })
        );
    }

    #[test]
    fn truncated_dictionary_err() {
        let mut wire = encode_string_dictionary(&[(1, 0, b"ab")]).unwrap();
        wire.truncate(wire.len() - 1);
        match decode_string_dictionary(&wire) {
            Err(StringDictError::Truncated { .. }) => {}
            other => panic!("expected Truncated, got {other:?}"),
        }
    }

    #[test]
    fn reserved_id_zero_encode_err() {
        assert_eq!(
            encode_string_dictionary(&[(0, 0, b"x")]),
            Err(StringDictError::ReservedIdZero)
        );
    }

    #[test]
    fn duplicate_id_encode_err() {
        assert_eq!(
            encode_string_dictionary(&[(1, 0, b"a"), (1, 0, b"b")]),
            Err(StringDictError::DuplicateId { id: 1 })
        );
    }

    #[test]
    fn event_body_mark_and_comment_resolve_from_dict() {
        let dict_wire =
            encode_string_dictionary(&[(1, FLAG_UTF8, b"dict-label"), (2, 0, b"# dict comment")])
                .unwrap();
        let (dict, _) = decode_string_dictionary(&dict_wire).unwrap();

        // Non-zero string_id with empty inline payload → interned.
        let body = encode_event_body(&[
            EventRecordSpec::Mark {
                string_id: 1,
                string_flags: 0,
                label: b"",
            },
            EventRecordSpec::Comment {
                string_id: 2,
                string_flags: 0,
                text: b"",
            },
            EventRecordSpec::Mark {
                string_id: 0,
                string_flags: 0,
                label: b"inline-mark",
            },
        ]);
        let (recs, n) = decode_event_body(&body).unwrap();
        assert_eq!(n, body.len());
        let owned = resolve_event_records(&recs, &dict).expect("resolve");
        assert_eq!(owned.len(), 3);
        match &owned[0] {
            OwnedEventRecord::Mark { label } => assert_eq!(label, b"dict-label"),
            other => panic!("{other:?}"),
        }
        match &owned[1] {
            OwnedEventRecord::Comment { text } => assert_eq!(text, b"# dict comment"),
            other => panic!("{other:?}"),
        }
        match &owned[2] {
            OwnedEventRecord::Mark { label } => assert_eq!(label, b"inline-mark"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn event_body_unknown_string_id_fail_closed() {
        let dict = StringDictionary::new();
        let body = encode_event_body(&[EventRecordSpec::Mark {
            string_id: 5,
            string_flags: 0,
            label: b"",
        }]);
        let (recs, _) = decode_event_body(&body).unwrap();
        assert_eq!(
            resolve_event_records(&recs, &dict),
            Err(StringDictError::UnknownId { id: 5 })
        );
    }
}
