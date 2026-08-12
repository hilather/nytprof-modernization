//! Strict **v5 profile encoder** from dump-aligned logical [`Event`]s.
//!
//! Wire layout mirrors oracle 6.15 / COL-006 `nytp_sink_v5` and the independent
//! decoder in [`crate::reader`]:
//! - text header `NYTProf 5 0\n`
//! - text-phase `ATTRIBUTE` / `OPTION` / `COMMENT` / `START_DEFLATE`
//! - optional zlib body after `START_DEFLATE` (`windowBits=15`)
//! - packed integers + native LE `f64` NV + length-prefixed strings
//!
//! **Strict:** values outside v5 ranges (I32 ticks, U32 fields, finite NV) fail
//! closed. No lossy truncation. Used by PR-C01 convert tooling.

use std::io::Write;

use flate2::write::ZlibEncoder;
use flate2::Compression;
use nytprof_types::{tags, Event};
use serde_json::Value;

use crate::error::{Error, Result};
use crate::varint::encode_u32;

/// Wire tag bytes (FileHandle.h) — same as reader.
mod wire {
    pub const ATTRIBUTE: u8 = b':';
    pub const OPTION: u8 = b'!';
    pub const COMMENT: u8 = b'#';
    pub const TIME_BLOCK: u8 = b'*';
    pub const TIME_LINE: u8 = b'+';
    pub const DISCOUNT: u8 = b'-';
    pub const NEW_FID: u8 = b'@';
    pub const SRC_LINE: u8 = b'S';
    pub const SUB_INFO: u8 = b's';
    pub const SUB_CALLERS: u8 = b'c';
    pub const PID_START: u8 = b'P';
    pub const PID_END: u8 = b'p';
    pub const STRING: u8 = b'\'';
    pub const START_DEFLATE: u8 = b'z';
    pub const SUB_ENTRY: u8 = b'>';
    pub const SUB_RETURN: u8 = b'<';
}

/// Encode a complete v5 profile from an ordered logical event stream.
///
/// - Leading `VERSION` (if present) must be major **5** (or is rewritten only
///   when `force_v5_header` path is used via [`encode_all_as_v5`]).
/// - Binary tags may appear before `START_DEFLATE` (uncompressed) or after
///   (zlib-compressed). If the first binary tag appears without a prior
///   `START_DEFLATE`, one is **auto-injected** so common 6.15 tools see a
///   normal compressed body.
pub fn encode_all(events: &[Event]) -> Result<Vec<u8>> {
    // Strict v5-only: VERSION major must be 5 (not projection of 6).
    encode_all_inner(events, /*project_v5_or_v6=*/ false)
}

/// Like [`encode_all`], but accepts source `VERSION` major **5 or 6** and always
/// writes header `NYTProf 5 0\n` (strict v6→v5 projection path).
///
/// Other majors (4, 7, …) are **refused** — no silent re-header of unknown
/// streams. Non-version events use the same representability checks as
/// [`encode_all`], including exact f64 NV mantissa checks.
pub fn encode_all_as_v5(events: &[Event]) -> Result<Vec<u8>> {
    encode_all_inner(events, /*project_v5_or_v6=*/ true)
}

fn encode_all_inner(events: &[Event], project_v5_or_v6: bool) -> Result<Vec<u8>> {
    let mut out: Vec<u8> = Vec::with_capacity(4096);
    out.extend_from_slice(b"NYTProf 5 0\n");

    let mut i = 0usize;
    // Skip/validate VERSION — header already written.
    if let Some(ev) = events.first() {
        if ev.tag == tags::VERSION {
            let major = arg_u64(&ev.args, 0, tags::VERSION, "major")?;
            let _minor = arg_u64(&ev.args, 1, tags::VERSION, "minor")?;
            if project_v5_or_v6 {
                // Projection path: only majors 5 and 6 are known product families.
                if major != 5 && major != 6 {
                    return Err(Error::format(format!(
                        "strict v5 encode: VERSION major {major} not projectable (only 5 or 6)"
                    )));
                }
            } else if major != 5 {
                return Err(Error::format(format!(
                    "strict v5 encode: VERSION major {major} is not 5 (use encode_all_as_v5 for projection)"
                )));
            }
            i = 1;
        }
    }

    let mut binary = Vec::new();
    let mut deflating = false;
    let mut saw_binary = false;

    while i < events.len() {
        let ev = &events[i];
        i += 1;

        match ev.tag.as_str() {
            tags::VERSION => {
                // Only the first VERSION is header material; extra VERSION is fail-closed
                // (v5 wire has no mid-stream version tag).
                return Err(Error::format(format!(
                    "strict v5 encode: duplicate VERSION at seq {}",
                    ev.seq
                )));
            }
            tags::END => {
                // Synthetic dump trailer — never on wire.
                continue;
            }
            tags::START_DEFLATE if !deflating => {
                // Text-phase START_DEFLATE: tag on outer stream, then switch.
                out.push(wire::START_DEFLATE);
                deflating = true;
            }
            tags::START_DEFLATE => {
                return Err(Error::format(format!(
                    "strict v5 encode: duplicate START_DEFLATE at seq {}",
                    ev.seq
                )));
            }
            tags::COMMENT | tags::ATTRIBUTE | tags::OPTION => {
                // Text-form tags may appear before *and after* START_DEFLATE
                // (oracle writes ATTRIBUTE/OPTION/COMMENT inside the inflated body).
                if deflating {
                    encode_text_tag(&mut binary, ev)?;
                    saw_binary = true;
                } else if saw_binary {
                    // Uncompressed binary phase (rare): still text-form on the same stream.
                    encode_text_tag(&mut out, ev)?;
                } else {
                    encode_text_tag(&mut out, ev)?;
                }
            }
            _ => {
                // Binary / structured tag.
                if !deflating && !saw_binary {
                    // Auto-inject START_DEFLATE for old-tool friendliness.
                    out.push(wire::START_DEFLATE);
                    deflating = true;
                }
                saw_binary = true;
                if deflating {
                    encode_binary_tag(&mut binary, ev)?;
                } else {
                    encode_binary_tag(&mut out, ev)?;
                }
            }
        }
    }

    if deflating {
        // Compress binary body (oracle windowBits=15 via flate2 default zlib).
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(&binary).map_err(|e| {
            Error::Zlib(format!("deflate failed while encoding v5 body: {e}"))
        })?;
        let compressed = enc.finish().map_err(|e| {
            Error::Zlib(format!("deflate finish failed while encoding v5 body: {e}"))
        })?;
        out.extend_from_slice(&compressed);
    }

    Ok(out)
}

fn encode_text_tag(out: &mut Vec<u8>, ev: &Event) -> Result<()> {
    match ev.tag.as_str() {
        tags::COMMENT => {
            let text = arg_str(&ev.args, 0, tags::COMMENT, "text")?;
            out.push(wire::COMMENT);
            out.extend_from_slice(text.as_bytes());
            if !text.ends_with('\n') {
                out.push(b'\n');
            }
            Ok(())
        }
        tags::ATTRIBUTE => {
            let key = arg_str(&ev.args, 0, tags::ATTRIBUTE, "key")?;
            let value = arg_str(&ev.args, 1, tags::ATTRIBUTE, "value")?;
            // Fail closed on embedded newlines / '=' in key (corrupt wire).
            if key.contains('=') || key.contains('\n') || value.contains('\n') {
                return Err(Error::format(format!(
                    "strict v5 encode: ATTRIBUTE key/value must not embed '=' (key) or newlines (seq {})",
                    ev.seq
                )));
            }
            out.push(wire::ATTRIBUTE);
            out.extend_from_slice(key.as_bytes());
            out.push(b'=');
            out.extend_from_slice(value.as_bytes());
            out.push(b'\n');
            Ok(())
        }
        tags::OPTION => {
            let key = arg_str(&ev.args, 0, tags::OPTION, "key")?;
            let value = arg_str(&ev.args, 1, tags::OPTION, "value")?;
            if key.contains('=') || key.contains('\n') || value.contains('\n') {
                return Err(Error::format(format!(
                    "strict v5 encode: OPTION key/value must not embed '=' (key) or newlines (seq {})",
                    ev.seq
                )));
            }
            out.push(wire::OPTION);
            out.extend_from_slice(key.as_bytes());
            out.push(b'=');
            out.extend_from_slice(value.as_bytes());
            out.push(b'\n');
            Ok(())
        }
        other => Err(Error::format(format!(
            "strict v5 encode: internal text-phase tag {other}"
        ))),
    }
}

fn encode_binary_tag(out: &mut Vec<u8>, ev: &Event) -> Result<()> {
    match ev.tag.as_str() {
        tags::DISCOUNT => {
            out.push(wire::DISCOUNT);
            Ok(())
        }
        tags::TIME_LINE => {
            let ticks = arg_i32_ticks(&ev.args, 0, tags::TIME_LINE, "ticks")?;
            let fid = arg_u32(&ev.args, 1, tags::TIME_LINE, "fid")?;
            let line = arg_u32(&ev.args, 2, tags::TIME_LINE, "line")?;
            out.push(wire::TIME_LINE);
            out.extend_from_slice(&encode_u32(ticks as u32));
            out.extend_from_slice(&encode_u32(fid));
            out.extend_from_slice(&encode_u32(line));
            Ok(())
        }
        tags::TIME_BLOCK => {
            let ticks = arg_i32_ticks(&ev.args, 0, tags::TIME_BLOCK, "ticks")?;
            let fid = arg_u32(&ev.args, 1, tags::TIME_BLOCK, "fid")?;
            let line = arg_u32(&ev.args, 2, tags::TIME_BLOCK, "line")?;
            let block_line = arg_u32(&ev.args, 3, tags::TIME_BLOCK, "block_line")?;
            let sub_line = arg_u32(&ev.args, 4, tags::TIME_BLOCK, "sub_line")?;
            out.push(wire::TIME_BLOCK);
            out.extend_from_slice(&encode_u32(ticks as u32));
            out.extend_from_slice(&encode_u32(fid));
            out.extend_from_slice(&encode_u32(line));
            out.extend_from_slice(&encode_u32(block_line));
            out.extend_from_slice(&encode_u32(sub_line));
            Ok(())
        }
        tags::NEW_FID => {
            let fid = arg_u32(&ev.args, 0, tags::NEW_FID, "fid")?;
            let eval_fid = arg_u32(&ev.args, 1, tags::NEW_FID, "eval_fid")?;
            let eval_line = arg_u32(&ev.args, 2, tags::NEW_FID, "eval_line")?;
            let flags = arg_u32(&ev.args, 3, tags::NEW_FID, "flags")?;
            let size = arg_u32(&ev.args, 4, tags::NEW_FID, "size")?;
            let mtime = arg_u32(&ev.args, 5, tags::NEW_FID, "mtime")?;
            let name = arg_str(&ev.args, 6, tags::NEW_FID, "name")?;
            out.push(wire::NEW_FID);
            out.extend_from_slice(&encode_u32(fid));
            out.extend_from_slice(&encode_u32(eval_fid));
            out.extend_from_slice(&encode_u32(eval_line));
            out.extend_from_slice(&encode_u32(flags));
            out.extend_from_slice(&encode_u32(size));
            out.extend_from_slice(&encode_u32(mtime));
            encode_str(out, name)?;
            Ok(())
        }
        tags::SRC_LINE => {
            let fid = arg_u32(&ev.args, 0, tags::SRC_LINE, "fid")?;
            let line = arg_u32(&ev.args, 1, tags::SRC_LINE, "line")?;
            let text = arg_str(&ev.args, 2, tags::SRC_LINE, "text")?;
            out.push(wire::SRC_LINE);
            out.extend_from_slice(&encode_u32(fid));
            out.extend_from_slice(&encode_u32(line));
            encode_str(out, text)?;
            Ok(())
        }
        tags::SUB_ENTRY => {
            let caller_fid = arg_u32(&ev.args, 0, tags::SUB_ENTRY, "caller_fid")?;
            let caller_line = arg_u32(&ev.args, 1, tags::SUB_ENTRY, "caller_line")?;
            out.push(wire::SUB_ENTRY);
            out.extend_from_slice(&encode_u32(caller_fid));
            out.extend_from_slice(&encode_u32(caller_line));
            Ok(())
        }
        tags::SUB_RETURN => {
            let depth = arg_u32(&ev.args, 0, tags::SUB_RETURN, "depth")?;
            let incl = arg_nv(&ev.args, 1, tags::SUB_RETURN, "incl")?;
            let excl = arg_nv(&ev.args, 2, tags::SUB_RETURN, "excl")?;
            let name = arg_str(&ev.args, 3, tags::SUB_RETURN, "subname")?;
            out.push(wire::SUB_RETURN);
            out.extend_from_slice(&encode_u32(depth));
            encode_nv(out, incl);
            encode_nv(out, excl);
            encode_str(out, name)?;
            Ok(())
        }
        tags::SUB_INFO => {
            // Callback order: fid, first, last, name → wire: fid, name, first, last
            let fid = arg_u32(&ev.args, 0, tags::SUB_INFO, "fid")?;
            let first = arg_u32(&ev.args, 1, tags::SUB_INFO, "first_line")?;
            let last = arg_u32(&ev.args, 2, tags::SUB_INFO, "last_line")?;
            let name = arg_str(&ev.args, 3, tags::SUB_INFO, "name")?;
            out.push(wire::SUB_INFO);
            out.extend_from_slice(&encode_u32(fid));
            encode_str(out, name)?;
            out.extend_from_slice(&encode_u32(first));
            out.extend_from_slice(&encode_u32(last));
            Ok(())
        }
        tags::SUB_CALLERS => {
            // Callback: fid, line, count, incl, excl, reci, rec_depth, called, caller
            // Wire:     fid, line, caller, count, incl, excl, reci, rec_depth, called
            let fid = arg_u32(&ev.args, 0, tags::SUB_CALLERS, "fid")?;
            let line = arg_u32(&ev.args, 1, tags::SUB_CALLERS, "line")?;
            let count = arg_u32(&ev.args, 2, tags::SUB_CALLERS, "count")?;
            let incl = arg_nv(&ev.args, 3, tags::SUB_CALLERS, "incl")?;
            let excl = arg_nv(&ev.args, 4, tags::SUB_CALLERS, "excl")?;
            let reci = arg_nv(&ev.args, 5, tags::SUB_CALLERS, "reci")?;
            let rec_depth = arg_u32(&ev.args, 6, tags::SUB_CALLERS, "rec_depth")?;
            let called = arg_str(&ev.args, 7, tags::SUB_CALLERS, "called")?;
            let caller = arg_str(&ev.args, 8, tags::SUB_CALLERS, "caller")?;
            out.push(wire::SUB_CALLERS);
            out.extend_from_slice(&encode_u32(fid));
            out.extend_from_slice(&encode_u32(line));
            encode_str(out, caller)?;
            out.extend_from_slice(&encode_u32(count));
            encode_nv(out, incl);
            encode_nv(out, excl);
            encode_nv(out, reci);
            out.extend_from_slice(&encode_u32(rec_depth));
            encode_str(out, called)?;
            Ok(())
        }
        tags::PID_START => {
            let pid = arg_u32(&ev.args, 0, tags::PID_START, "pid")?;
            let ppid = arg_u32(&ev.args, 1, tags::PID_START, "ppid")?;
            let time = arg_nv(&ev.args, 2, tags::PID_START, "start_time")?;
            out.push(wire::PID_START);
            out.extend_from_slice(&encode_u32(pid));
            out.extend_from_slice(&encode_u32(ppid));
            encode_nv(out, time);
            Ok(())
        }
        tags::PID_END => {
            let pid = arg_u32(&ev.args, 0, tags::PID_END, "pid")?;
            let time = arg_nv(&ev.args, 1, tags::PID_END, "end_time")?;
            out.push(wire::PID_END);
            out.extend_from_slice(&encode_u32(pid));
            encode_nv(out, time);
            Ok(())
        }
        other => Err(Error::format(format!(
            "strict v5 encode: unsupported / unrepresentable tag '{other}' at seq {}",
            ev.seq
        ))),
    }
}

fn encode_str(out: &mut Vec<u8>, s: &str) -> Result<()> {
    let bytes = s.as_bytes();
    if bytes.len() > u32::MAX as usize {
        return Err(Error::format(format!(
            "strict v5 encode: string length {} exceeds u32",
            bytes.len()
        )));
    }
    out.push(wire::STRING);
    out.extend_from_slice(&encode_u32(bytes.len() as u32));
    out.extend_from_slice(bytes);
    Ok(())
}

fn encode_nv(out: &mut Vec<u8>, v: f64) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn arg_at<'a>(args: &'a [Value], i: usize, tag: &str, field: &str) -> Result<&'a Value> {
    args.get(i).ok_or_else(|| {
        Error::format(format!(
            "strict v5 encode: {tag} missing arg[{i}] ({field})"
        ))
    })
}

fn arg_u64(args: &[Value], i: usize, tag: &str, field: &str) -> Result<u64> {
    let v = arg_at(args, i, tag, field)?;
    match v {
        Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                Ok(u)
            } else if let Some(i64v) = n.as_i64() {
                if i64v < 0 {
                    Err(Error::format(format!(
                        "strict v5 encode: {tag}.{field} negative ({i64v})"
                    )))
                } else {
                    Ok(i64v as u64)
                }
            } else {
                Err(Error::format(format!(
                    "strict v5 encode: {tag}.{field} not an integer ({v})"
                )))
            }
        }
        _ => Err(Error::format(format!(
            "strict v5 encode: {tag}.{field} not a number ({v})"
        ))),
    }
}

fn arg_u32(args: &[Value], i: usize, tag: &str, field: &str) -> Result<u32> {
    let u = arg_u64(args, i, tag, field)?;
    if u > u32::MAX as u64 {
        return Err(Error::format(format!(
            "strict v5 encode: {tag}.{field}={u} exceeds u32"
        )));
    }
    Ok(u as u32)
}

/// TIME_* ticks as I32 (oracle packed i32). Non-negative values must fit I32_MAX.
fn arg_i32_ticks(args: &[Value], i: usize, tag: &str, field: &str) -> Result<i32> {
    let v = arg_at(args, i, tag, field)?;
    let n = match v {
        Value::Number(n) => n,
        _ => {
            return Err(Error::format(format!(
                "strict v5 encode: {tag}.{field} not a number ({v})"
            )))
        }
    };
    if let Some(i64v) = n.as_i64() {
        if i64v < i32::MIN as i64 || i64v > i32::MAX as i64 {
            return Err(Error::format(format!(
                "strict v5 encode: {tag}.{field}={i64v} outside i32 range"
            )));
        }
        return Ok(i64v as i32);
    }
    if let Some(u) = n.as_u64() {
        if u > i32::MAX as u64 {
            return Err(Error::format(format!(
                "strict v5 encode: {tag}.{field}={u} outside i32 range"
            )));
        }
        return Ok(u as i32);
    }
    Err(Error::format(format!(
        "strict v5 encode: {tag}.{field} not an integer ({v})"
    )))
}

/// Parse NV for v5 wire as finite LE `f64`, with **exact** integer mantissa checks.
///
/// - Non-finite → refuse
/// - Values that arrived as integer JSON (`u64` / `i64`) must survive
///   `as f64` → back without rounding (refuses `|n| > 2^53` non-exact integers)
/// - Fractional numbers already representable as f64 (oracle wall-clock PID) pass
fn arg_nv(args: &[Value], i: usize, tag: &str, field: &str) -> Result<f64> {
    let v = arg_at(args, i, tag, field)?;
    match v {
        Value::Number(n) => {
            // Prefer exact integer paths first (serde may expose both).
            if let Some(u) = n.as_u64() {
                return exact_u64_as_f64_nv(u, tag, field);
            }
            if let Some(i64v) = n.as_i64() {
                return exact_i64_as_f64_nv(i64v, tag, field);
            }
            let f = n.as_f64().ok_or_else(|| {
                Error::format(format!(
                    "strict v5 encode: {tag}.{field} not representable as f64 ({v})"
                ))
            })?;
            if !f.is_finite() {
                return Err(Error::format(format!(
                    "strict v5 encode: {tag}.{field} is non-finite"
                )));
            }
            // Fractional (or non-integer JSON number): require the f64 image is exact
            // for any whole-number magnitude that would have been an integer path.
            if f.fract() == 0.0 && f.is_sign_positive() && f <= u64::MAX as f64 {
                let as_u = f as u64;
                if (as_u as f64) != f {
                    return Err(Error::format(format!(
                        "strict v5 encode: {tag}.{field}={f} not exactly representable as f64 NV"
                    )));
                }
            } else if f.fract() == 0.0
                && f >= i64::MIN as f64
                && f <= i64::MAX as f64
            {
                let as_i = f as i64;
                if (as_i as f64) != f {
                    return Err(Error::format(format!(
                        "strict v5 encode: {tag}.{field}={f} not exactly representable as f64 NV"
                    )));
                }
            }
            Ok(f)
        }
        _ => Err(Error::format(format!(
            "strict v5 encode: {tag}.{field} not a number ({v})"
        ))),
    }
}

fn exact_u64_as_f64_nv(u: u64, tag: &str, field: &str) -> Result<f64> {
    let f = u as f64;
    if f as u64 != u {
        return Err(Error::format(format!(
            "strict v5 encode: {tag}.{field}={u} not exactly representable as f64 NV (mantissa)"
        )));
    }
    if !f.is_finite() {
        return Err(Error::format(format!(
            "strict v5 encode: {tag}.{field} is non-finite"
        )));
    }
    Ok(f)
}

fn exact_i64_as_f64_nv(i: i64, tag: &str, field: &str) -> Result<f64> {
    let f = i as f64;
    if f as i64 != i {
        return Err(Error::format(format!(
            "strict v5 encode: {tag}.{field}={i} not exactly representable as f64 NV (mantissa)"
        )));
    }
    if !f.is_finite() {
        return Err(Error::format(format!(
            "strict v5 encode: {tag}.{field} is non-finite"
        )));
    }
    Ok(f)
}

fn arg_str<'a>(args: &'a [Value], i: usize, tag: &str, field: &str) -> Result<&'a str> {
    let v = arg_at(args, i, tag, field)?;
    match v {
        Value::String(s) => Ok(s.as_str()),
        _ => Err(Error::format(format!(
            "strict v5 encode: {tag}.{field} not a string ({v})"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::decode_all;
    use serde_json::json;

    fn ev(seq: u64, tag: &str, args: Vec<Value>) -> Event {
        Event::new(seq, tag, args)
    }

    #[test]
    fn round_trip_mini_stream() {
        let events = vec![
            ev(0, tags::VERSION, vec![json!(5), json!(0)]),
            ev(
                1,
                tags::ATTRIBUTE,
                vec![json!("ticks_per_sec"), json!("1000000")],
            ),
            ev(2, tags::OPTION, vec![json!("calls"), json!("1")]),
            ev(3, tags::START_DEFLATE, vec![]),
            ev(4, tags::PID_START, vec![json!(1), json!(0), json!(0)]),
            ev(5, tags::TIME_LINE, vec![json!(10), json!(1), json!(5)]),
            ev(6, tags::DISCOUNT, vec![]),
            ev(
                7,
                tags::SUB_RETURN,
                vec![json!(1), json!(100), json!(40), json!("main::leaf")],
            ),
            ev(8, tags::PID_END, vec![json!(1), json!(1)]),
        ];
        let wire = encode_all(&events).expect("encode");
        assert!(wire.starts_with(b"NYTProf 5 0\n"));
        let back = decode_all(&wire).expect("decode");
        // VERSION re-emitted from header; START_DEFLATE present; counts match.
        assert_eq!(back[0].tag, tags::VERSION);
        assert!(back.iter().any(|e| e.tag == tags::TIME_LINE));
        assert!(back.iter().any(|e| e.tag == tags::SUB_RETURN));
        assert!(back.iter().any(|e| e.tag == tags::PID_END));
        let tl = back.iter().find(|e| e.tag == tags::TIME_LINE).unwrap();
        assert_eq!(tl.args[0], json!(10));
        assert_eq!(tl.args[1], json!(1));
        assert_eq!(tl.args[2], json!(5));
    }

    #[test]
    fn auto_inject_start_deflate() {
        let events = vec![
            ev(0, tags::VERSION, vec![json!(5), json!(0)]),
            ev(1, tags::TIME_LINE, vec![json!(1), json!(1), json!(1)]),
            ev(2, tags::PID_END, vec![json!(1), json!(0)]),
        ];
        let wire = encode_all(&events).expect("encode");
        let back = decode_all(&wire).expect("decode");
        assert!(back.iter().any(|e| e.tag == tags::START_DEFLATE));
        assert!(back.iter().any(|e| e.tag == tags::TIME_LINE));
    }

    #[test]
    fn strict_ticks_overflow_fails() {
        let events = vec![
            ev(0, tags::VERSION, vec![json!(5), json!(0)]),
            // i32::MAX + 1
            ev(
                1,
                tags::TIME_LINE,
                vec![json!((i32::MAX as i64) + 1), json!(1), json!(1)],
            ),
        ];
        let err = encode_all(&events).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("outside i32") || msg.contains("strict"),
            "unexpected: {msg}"
        );
    }

    #[test]
    fn encode_as_v5_accepts_major_6_version() {
        let events = vec![
            ev(0, tags::VERSION, vec![json!(6), json!(0)]),
            ev(1, tags::DISCOUNT, vec![]),
        ];
        let wire = encode_all_as_v5(&events).expect("encode_as_v5");
        assert!(wire.starts_with(b"NYTProf 5 0\n"));
        let back = decode_all(&wire).expect("decode");
        assert_eq!(back[0].args[0], json!(5));
    }

    #[test]
    fn encode_all_rejects_major_6() {
        let events = vec![
            ev(0, tags::VERSION, vec![json!(6), json!(0)]),
            ev(1, tags::DISCOUNT, vec![]),
        ];
        assert!(encode_all(&events).is_err());
    }

    #[test]
    fn encode_as_v5_refuses_unknown_major() {
        let events = vec![
            ev(0, tags::VERSION, vec![json!(7), json!(0)]),
            ev(1, tags::DISCOUNT, vec![]),
        ];
        let err = encode_all_as_v5(&events).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("not projectable") || msg.contains("major"),
            "unexpected: {msg}"
        );
    }

    #[test]
    fn strict_nv_mantissa_overflow_refuses() {
        // 2^53 + 1 is not exactly representable as f64.
        let bad = (1u64 << 53) + 1;
        let events = vec![
            ev(0, tags::VERSION, vec![json!(5), json!(0)]),
            ev(
                1,
                tags::SUB_RETURN,
                vec![json!(1), json!(bad), json!(0u64), json!("main::x")],
            ),
        ];
        let err = encode_all(&events).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("exactly representable") || msg.contains("mantissa"),
            "unexpected: {msg}"
        );
    }

    #[test]
    fn strict_nv_exact_small_integer_ok() {
        let events = vec![
            ev(0, tags::VERSION, vec![json!(5), json!(0)]),
            ev(
                1,
                tags::SUB_RETURN,
                vec![json!(1), json!(100u64), json!(40u64), json!("main::leaf")],
            ),
        ];
        let wire = encode_all(&events).expect("small integer NV must encode");
        let back = decode_all(&wire).expect("decode");
        let sr = back.iter().find(|e| e.tag == tags::SUB_RETURN).unwrap();
        assert_eq!(sr.args[1], json!(100));
        assert_eq!(sr.args[2], json!(40));
    }
}
