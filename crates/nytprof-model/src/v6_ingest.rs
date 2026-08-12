//! Product v6 → logical [`Event`] adapter for [`ProfileModel`] ingest.
//!
//! Maps always-inflate `OwnedEventRecord` streams (expanded packing / resolved
//! FOOTER dict) onto dump-aligned ReadStream tags used by A1–A9 aggregation.
//! Not a wire freeze; not full multi-kind SOURCE/INDEX/SUMMARY product path.

use nytprof_format_v6::compressed_profile::OwnedEventRecord;
use nytprof_format_v6::{
    detect_profile_wire_kind, product_decode_v6_event_profile, ProfileWireKind,
};
use nytprof_types::{tags, Event};
use serde_json::Value;

use crate::{ModelError, Result};

/// Decode product profile bytes (v5 or v6) into an ordered logical event stream.
///
/// Dual dispatch:
/// - `NYTPROF6` magic → always-inflate EVENT (+ optional FOOTER dict) → Events
/// - `NYTProf <maj> <min>` text header → existing v5 decoder
/// - otherwise [`ModelError::UnsupportedProfile`]
pub fn decode_events_from_bytes(bytes: &[u8]) -> Result<Vec<Event>> {
    match detect_profile_wire_kind(bytes) {
        ProfileWireKind::V6 => decode_v6_events(bytes),
        ProfileWireKind::V5 => {
            nytprof_format_v5::decode_all(bytes).map_err(ModelError::Decode)
        }
        ProfileWireKind::Unknown => Err(ModelError::UnsupportedProfile {
            detail: "neither NYTPROF6 magic nor NYTProf v5 text header".into(),
        }),
    }
}

/// Read a path and decode via [`decode_events_from_bytes`].
pub fn decode_events_from_path(path: impl AsRef<std::path::Path>) -> Result<Vec<Event>> {
    let path = path.as_ref();
    let bytes = std::fs::read(path).map_err(|e| ModelError::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    decode_events_from_bytes(&bytes)
}

fn decode_v6_events(bytes: &[u8]) -> Result<Vec<Event>> {
    let decoded = product_decode_v6_event_profile(bytes, true)
        .map_err(|e| ModelError::DecodeV6(e.to_string()))?;
    // Fail closed if trailing garbage remains after a complete product profile.
    if decoded.bytes_consumed != bytes.len() {
        return Err(ModelError::DecodeV6(format!(
            "trailing bytes after v6 profile: consumed {} of {}",
            decoded.bytes_consumed,
            bytes.len()
        )));
    }
    owned_records_to_events(&decoded.profile.records, &decoded.profile.sequences)
}

/// Convert expanded/resolved owned v6 records to dump-aligned logical events.
///
/// **Dump `seq` is stream order (0..n-1)** after always-inflate expansion and
/// auto-VERSION inject. Packing `FLAG_HAS_SEQ` values live on the decoded profile
/// for E3 equality; reusing them here collided with injected VERSION (`None` → 0)
/// when body packing also starts at 0. Dump schema assigns monotonic dumper seq.
pub fn owned_records_to_events(
    records: &[OwnedEventRecord],
    sequences: &[Option<u64>],
) -> Result<Vec<Event>> {
    let _ = sequences; // packing wire seq retained on DecodedEventProfile, not dump Event.seq
    let mut out = Vec::with_capacity(records.len());
    for (i, rec) in records.iter().enumerate() {
        out.push(owned_record_to_event(i as u64, rec)?);
    }
    Ok(out)
}

fn bytes_to_string(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

fn num_u64(v: u64) -> Value {
    Value::from(v)
}

fn num_i64(v: i64) -> Value {
    Value::from(v)
}

fn num_f64(v: f64) -> Value {
    // Prefer integer JSON when exact (matches v5 dump style for whole ticks/times).
    if v.is_finite() && v.fract() == 0.0 && v >= 0.0 && v <= (u64::MAX as f64) {
        Value::from(v as u64)
    } else if v.is_finite() && v.fract() == 0.0 && v >= (i64::MIN as f64) && v <= (i64::MAX as f64)
    {
        Value::from(v as i64)
    } else {
        serde_json::Number::from_f64(v)
            .map(Value::Number)
            .unwrap_or(Value::Null)
    }
}

fn owned_record_to_event(seq: u64, rec: &OwnedEventRecord) -> Result<Event> {
    let ev = match rec {
        OwnedEventRecord::Version { major, minor } => Event::new(
            seq,
            tags::VERSION,
            vec![num_u64(*major), num_u64(*minor)],
        ),
        OwnedEventRecord::Comment { text } => {
            Event::new(seq, tags::COMMENT, vec![Value::String(bytes_to_string(text))])
        }
        // MARK is not a ReadStream tag; surface as COMMENT for dump visibility.
        OwnedEventRecord::Mark { label } => {
            Event::new(seq, tags::COMMENT, vec![Value::String(bytes_to_string(label))])
        }
        OwnedEventRecord::Attribute { key, value } => Event::new(
            seq,
            tags::ATTRIBUTE,
            vec![
                Value::String(bytes_to_string(key)),
                Value::String(bytes_to_string(value)),
            ],
        ),
        OwnedEventRecord::Option { key, value } => Event::new(
            seq,
            tags::OPTION,
            vec![
                Value::String(bytes_to_string(key)),
                Value::String(bytes_to_string(value)),
            ],
        ),
        OwnedEventRecord::StartDeflate => Event::new(seq, tags::START_DEFLATE, vec![]),
        OwnedEventRecord::PidStart {
            pid,
            ppid,
            start_time,
        } => Event::new(
            seq,
            tags::PID_START,
            vec![num_u64(*pid), num_u64(*ppid), num_u64(*start_time)],
        ),
        OwnedEventRecord::PidEnd { pid, end_time } => Event::new(
            seq,
            tags::PID_END,
            vec![num_u64(*pid), num_u64(*end_time)],
        ),
        OwnedEventRecord::NewFid { fid, filename } => {
            // ReadStream shape: fid, eval_fid, eval_line, flags, size, mtime, name.
            // v6 EVENT body carries fid+filename only; pad intermediate fields with 0.
            Event::new(
                seq,
                tags::NEW_FID,
                vec![
                    num_u64(*fid),
                    num_u64(0),
                    num_u64(0),
                    num_u64(0),
                    num_u64(0),
                    num_u64(0),
                    Value::String(bytes_to_string(filename)),
                ],
            )
        }
        OwnedEventRecord::TimeLine { fid, line, ticks } => {
            // ticks may exceed i64; model as_i64 accepts u64-in-range via Number.
            let ticks_v = if *ticks <= i64::MAX as u64 {
                num_i64(*ticks as i64)
            } else {
                num_u64(*ticks)
            };
            Event::new(
                seq,
                tags::TIME_LINE,
                vec![ticks_v, num_u64(*fid), num_u64(*line)],
            )
        }
        OwnedEventRecord::TimeBlock {
            fid,
            line,
            block_line,
            ticks,
        } => {
            // v5 args include sub_line; v6 absolute body has no sub_line — emit 0.
            let ticks_v = if *ticks <= i64::MAX as u64 {
                num_i64(*ticks as i64)
            } else {
                num_u64(*ticks)
            };
            Event::new(
                seq,
                tags::TIME_BLOCK,
                vec![
                    ticks_v,
                    num_u64(*fid),
                    num_u64(*line),
                    num_u64(*block_line),
                    num_u64(0),
                ],
            )
        }
        OwnedEventRecord::Discount => Event::new(seq, tags::DISCOUNT, vec![]),
        OwnedEventRecord::SubEntry {
            caller_fid,
            caller_line,
        } => Event::new(
            seq,
            tags::SUB_ENTRY,
            vec![num_u64(*caller_fid), num_u64(*caller_line)],
        ),
        OwnedEventRecord::SubReturn {
            depth,
            incl,
            excl,
            subname,
        } => Event::new(
            seq,
            tags::SUB_RETURN,
            vec![
                num_u64(*depth),
                num_f64(*incl as f64),
                num_f64(*excl as f64),
                Value::String(bytes_to_string(subname)),
            ],
        ),
        OwnedEventRecord::SubInfo {
            fid,
            first_line,
            last_line,
            name,
        } => Event::new(
            seq,
            tags::SUB_INFO,
            vec![
                num_u64(*fid),
                num_u64(*first_line),
                num_u64(*last_line),
                Value::String(bytes_to_string(name)),
            ],
        ),
        OwnedEventRecord::SubCallers {
            fid,
            line,
            count,
            incl,
            excl,
            reci,
            rec_depth,
            called,
            caller,
        } => Event::new(
            seq,
            tags::SUB_CALLERS,
            vec![
                num_u64(*fid),
                num_u64(*line),
                num_u64(*count),
                num_f64(*incl as f64),
                num_f64(*excl as f64),
                num_f64(*reci as f64),
                num_u64(*rec_depth),
                Value::String(bytes_to_string(called)),
                Value::String(bytes_to_string(caller)),
            ],
        ),
        OwnedEventRecord::SrcLine { fid, line, text } => Event::new(
            seq,
            tags::SRC_LINE,
            vec![
                num_u64(*fid),
                num_u64(*line),
                Value::String(bytes_to_string(text)),
            ],
        ),
    };
    Ok(ev)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nytprof_format_v6::chunk::codec;
    use nytprof_format_v6::event_body::EventRecordSpec;
    use nytprof_format_v6::{e3_standin_write_absolute, e3_standin_write_packing};

    #[test]
    fn owned_timeline_maps_to_dump_args() {
        let rec = OwnedEventRecord::TimeLine {
            fid: 1,
            line: 10,
            ticks: 5,
        };
        let ev = owned_record_to_event(0, &rec).unwrap();
        assert_eq!(ev.tag, tags::TIME_LINE);
        assert_eq!(ev.args[0], Value::from(5i64));
        assert_eq!(ev.args[1], Value::from(1u64));
        assert_eq!(ev.args[2], Value::from(10u64));
    }

    #[test]
    fn absolute_and_packing_standin_decode_to_same_tags() {
        let specs = [
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 10,
                ticks: 5,
            },
            EventRecordSpec::TimeLine {
                fid: 1,
                line: 11,
                ticks: 6,
            },
            EventRecordSpec::TimeBlock {
                fid: 1,
                line: 12,
                block_line: 4,
                ticks: 20,
            },
            EventRecordSpec::SubEntry {
                caller_fid: 1,
                caller_line: 10,
            },
            EventRecordSpec::SubReturn {
                depth: 1,
                incl: 100,
                excl: 40,
                string_id: 0,
                string_flags: 0,
                subname: b"main::leaf",
            },
        ];
        let abs = e3_standin_write_absolute(&specs, codec::NONE).unwrap();
        let pack = e3_standin_write_packing(&specs, codec::ZLIB, 2).unwrap();
        let a = decode_v6_events(&abs).unwrap();
        let p = decode_v6_events(&pack).unwrap();
        let tags_a: Vec<_> = a.iter().map(|e| e.tag.as_str()).collect();
        let tags_p: Vec<_> = p.iter().map(|e| e.tag.as_str()).collect();
        assert_eq!(tags_a, tags_p);
        assert!(tags_a.contains(&tags::TIME_LINE));
        assert!(tags_a.contains(&tags::SUB_RETURN));
    }
}
