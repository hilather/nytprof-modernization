//! v5 profile stream reader (text header + optional zlib binary phase).

use std::fs;
use std::io::Read;
use std::path::Path;

use flate2::read::ZlibDecoder;
use nytprof_types::{tags, Event};
use serde_json::{Number, Value};

use crate::error::{Error, Result};
use crate::varint::{read_i32, read_u32};

/// Wire tag bytes (FileHandle.h).
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
    pub const STRING_UTF8: u8 = b'"';
    pub const START_DEFLATE: u8 = b'z';
    pub const SUB_ENTRY: u8 = b'>';
    pub const SUB_RETURN: u8 = b'<';
}

/// Decode a profile from a filesystem path.
pub fn decode_path(path: impl AsRef<Path>) -> Result<Vec<Event>> {
    let data = fs::read(path.as_ref())?;
    decode_all(&data)
}

/// Decode a complete profile from in-memory bytes.
pub fn decode_all(data: &[u8]) -> Result<Vec<Event>> {
    EventIter::new(data)?.collect()
}

/// Iterator over profile events (excludes synthetic `_END`).
///
/// Construction parses the whole stream up front (profiles are modest). The
/// public surface is still event-stream oriented for callers.
pub struct EventIter {
    events: std::vec::IntoIter<Event>,
}

impl EventIter {
    /// Parse `data` into a stream of events.
    ///
    /// Mirrors `load_profile_data_from_stream`: one tag loop for the whole
    /// file. `START_DEFLATE` (`z`) switches subsequent reads to zlib inflate
    /// (windowBits=15). Trailing raw bytes after the inflate stream (oracle
    /// writes post-close comments) are ignored.
    pub fn new(data: &[u8]) -> Result<Self> {
        let mut events = Vec::new();
        let mut seq: u64 = 0;

        // --- header ---
        let header_end = find_newline(data, 0).ok_or_else(|| {
            Error::format("NYTProf data format error while reading header")
        })?;
        let header = std::str::from_utf8(&data[0..header_end])
            .map_err(|_| Error::format("header is not valid UTF-8"))?;
        let (major, minor) = parse_header(header)?;
        events.push(Event::new(
            seq,
            tags::VERSION,
            vec![json_u64(major as u64), json_u64(minor as u64)],
        ));
        seq += 1;

        // Body starts after header line. May later switch to an inflated buffer.
        let mut owned_inflated: Option<Vec<u8>> = None;
        let mut pos: usize = header_end + 1;
        // `cursor` is either the original file or the inflated body.
        let mut cursor: &[u8] = data;

        while pos < cursor.len() {
            let tag = cursor[pos];
            let tag_offset = pos as u64;
            pos += 1;

            match tag {
                wire::COMMENT => {
                    let (line, new_pos) = read_line_including_nl(cursor, pos)?;
                    pos = new_pos;
                    // Oracle COMMENT includes the trailing newline.
                    let text = String::from_utf8_lossy(&line).into_owned();
                    events.push(Event::new(seq, tags::COMMENT, vec![Value::String(text)]));
                    seq += 1;
                }
                wire::ATTRIBUTE => {
                    let (line, new_pos) = read_line_including_nl(cursor, pos)?;
                    pos = new_pos;
                    let (k, v) = parse_key_value(&line, "attribute")?;
                    events.push(Event::new(
                        seq,
                        tags::ATTRIBUTE,
                        vec![Value::String(k), Value::String(v)],
                    ));
                    seq += 1;
                }
                wire::OPTION => {
                    let (line, new_pos) = read_line_including_nl(cursor, pos)?;
                    pos = new_pos;
                    let (k, v) = parse_key_value(&line, "option")?;
                    events.push(Event::new(
                        seq,
                        tags::OPTION,
                        vec![Value::String(k), Value::String(v)],
                    ));
                    seq += 1;
                }
                wire::START_DEFLATE => {
                    events.push(Event::new(seq, tags::START_DEFLATE, vec![]));
                    seq += 1;
                    // Inflate the remainder of the *file* (not current cursor
                    // if we were already inflated — second z is unsupported).
                    if owned_inflated.is_some() {
                        return Err(Error::format(format!(
                            "duplicate START_DEFLATE at offset {tag_offset}"
                        )));
                    }
                    let mut decoder = ZlibDecoder::new(&data[pos..]);
                    let mut buf = Vec::new();
                    decoder.read_to_end(&mut buf).map_err(|e| {
                        Error::Zlib(format!("inflate failed after START_DEFLATE: {e}"))
                    })?;
                    owned_inflated = Some(buf);
                    cursor = owned_inflated.as_ref().unwrap();
                    pos = 0;
                }
                _ => {
                    let event = decode_payload_tag(tag, tag_offset, cursor, &mut pos, seq)?;
                    events.push(event);
                    seq += 1;
                }
            }
        }

        Ok(Self {
            events: events.into_iter(),
        })
    }
}

impl Iterator for EventIter {
    type Item = Result<Event>;

    fn next(&mut self) -> Option<Self::Item> {
        self.events.next().map(Ok)
    }
}

/// Binary / structured tags (and unknown → hard error).
fn decode_payload_tag(
    tag: u8,
    tag_offset: u64,
    data: &[u8],
    pos: &mut usize,
    seq: u64,
) -> Result<Event> {
    match tag {
        wire::DISCOUNT => Ok(Event::new(seq, tags::DISCOUNT, vec![])),

        wire::TIME_LINE => {
            let ticks = read_i32(data, pos)?;
            let fid = read_u32(data, pos)?;
            let line = read_u32(data, pos)?;
            Ok(Event::new(
                seq,
                tags::TIME_LINE,
                vec![
                    json_i32_as_oracle(ticks),
                    json_u64(fid as u64),
                    json_u64(line as u64),
                ],
            ))
        }

        wire::TIME_BLOCK => {
            let ticks = read_i32(data, pos)?;
            let fid = read_u32(data, pos)?;
            let line = read_u32(data, pos)?;
            let block_line = read_u32(data, pos)?;
            let sub_line = read_u32(data, pos)?;
            Ok(Event::new(
                seq,
                tags::TIME_BLOCK,
                vec![
                    json_i32_as_oracle(ticks),
                    json_u64(fid as u64),
                    json_u64(line as u64),
                    json_u64(block_line as u64),
                    json_u64(sub_line as u64),
                ],
            ))
        }

        wire::NEW_FID => {
            let id = read_u32(data, pos)?;
            let eval_fid = read_u32(data, pos)?;
            let eval_line = read_u32(data, pos)?;
            let flags = read_u32(data, pos)?;
            let size = read_u32(data, pos)?;
            let mtime = read_u32(data, pos)?;
            let name = read_str(data, pos)?;
            Ok(Event::new(
                seq,
                tags::NEW_FID,
                vec![
                    json_u64(id as u64),
                    json_u64(eval_fid as u64),
                    json_u64(eval_line as u64),
                    json_u64(flags as u64),
                    json_u64(size as u64),
                    json_u64(mtime as u64),
                    Value::String(name),
                ],
            ))
        }

        wire::SRC_LINE => {
            let fid = read_u32(data, pos)?;
            let line = read_u32(data, pos)?;
            let text = read_str(data, pos)?;
            Ok(Event::new(
                seq,
                tags::SRC_LINE,
                vec![
                    json_u64(fid as u64),
                    json_u64(line as u64),
                    Value::String(text),
                ],
            ))
        }

        wire::SUB_ENTRY => {
            let caller_fid = read_u32(data, pos)?;
            let caller_line = read_u32(data, pos)?;
            Ok(Event::new(
                seq,
                tags::SUB_ENTRY,
                vec![
                    json_u64(caller_fid as u64),
                    json_u64(caller_line as u64),
                ],
            ))
        }

        wire::SUB_RETURN => {
            let depth = read_u32(data, pos)?;
            let incl = read_nv(data, pos)?;
            let excl = read_nv(data, pos)?;
            let name = read_str(data, pos)?;
            Ok(Event::new(
                seq,
                tags::SUB_RETURN,
                vec![
                    json_u64(depth as u64),
                    json_nv(incl),
                    json_nv(excl),
                    Value::String(name),
                ],
            ))
        }

        wire::SUB_INFO => {
            // Wire: fid, name, first, last
            // Callback/schema: fid, first_line, last_line, name
            let fid = read_u32(data, pos)?;
            let name = read_str(data, pos)?;
            let first = read_u32(data, pos)?;
            let last = read_u32(data, pos)?;
            Ok(Event::new(
                seq,
                tags::SUB_INFO,
                vec![
                    json_u64(fid as u64),
                    json_u64(first as u64),
                    json_u64(last as u64),
                    Value::String(name),
                ],
            ))
        }

        wire::SUB_CALLERS => {
            // Wire: fid, line, caller, count, incl, excl, reci, rec_depth, called
            // Callback/schema: fid, line, count, incl, excl, reci, rec_depth, called, caller
            let fid = read_u32(data, pos)?;
            let line = read_u32(data, pos)?;
            let caller = read_str(data, pos)?;
            let count = read_u32(data, pos)?;
            let incl = read_nv(data, pos)?;
            let excl = read_nv(data, pos)?;
            let reci = read_nv(data, pos)?;
            let rec_depth = read_u32(data, pos)?;
            let called = read_str(data, pos)?;
            Ok(Event::new(
                seq,
                tags::SUB_CALLERS,
                vec![
                    json_u64(fid as u64),
                    json_u64(line as u64),
                    json_u64(count as u64),
                    json_nv(incl),
                    json_nv(excl),
                    json_nv(reci),
                    json_u64(rec_depth as u64),
                    Value::String(called),
                    Value::String(caller),
                ],
            ))
        }

        wire::PID_START => {
            let pid = read_u32(data, pos)?;
            let ppid = read_u32(data, pos)?;
            let time = read_nv(data, pos)?;
            Ok(Event::new(
                seq,
                tags::PID_START,
                vec![
                    json_u64(pid as u64),
                    json_u64(ppid as u64),
                    json_nv(time),
                ],
            ))
        }

        wire::PID_END => {
            let pid = read_u32(data, pos)?;
            let time = read_nv(data, pos)?;
            Ok(Event::new(
                seq,
                tags::PID_END,
                vec![json_u64(pid as u64), json_nv(time)],
            ))
        }

        other => Err(Error::UnsupportedTag {
            tag: other,
            ch: display_char(other),
            offset: tag_offset,
        }),
    }
}

/// String encoding: tag `'` or `"` then packed len then raw bytes.
fn read_str(data: &[u8], pos: &mut usize) -> Result<String> {
    if *pos >= data.len() {
        return Err(Error::UnexpectedEof {
            what: "string prefix",
            offset: *pos as u64,
        });
    }
    let tag = data[*pos];
    let tag_off = *pos as u64;
    *pos += 1;
    if tag != wire::STRING && tag != wire::STRING_UTF8 {
        return Err(Error::format(format!(
            "expected string tag at offset {tag_off}, found 0x{tag:02x} ('{}')",
            display_char(tag)
        )));
    }
    let len = read_u32(data, pos)? as usize;
    if *pos + len > data.len() {
        return Err(Error::UnexpectedEof {
            what: "string",
            offset: *pos as u64,
        });
    }
    let bytes = &data[*pos..*pos + len];
    *pos += len;
    Ok(String::from_utf8_lossy(bytes).into_owned())
}

/// NV: 8-byte native double. Platform is x86_64 little-endian (fixture nv_size=8).
fn read_nv(data: &[u8], pos: &mut usize) -> Result<f64> {
    if *pos + 8 > data.len() {
        return Err(Error::UnexpectedEof {
            what: "float",
            offset: *pos as u64,
        });
    }
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&data[*pos..*pos + 8]);
    *pos += 8;
    Ok(f64::from_le_bytes(buf))
}

fn parse_header(line: &str) -> Result<(u32, u32)> {
    let line = line.trim_end_matches(['\r', '\n']);
    let rest = line
        .strip_prefix("NYTProf ")
        .ok_or_else(|| Error::format(format!("bad header magic: {line:?}")))?;
    let mut parts = rest.split_whitespace();
    let major: u32 = parts
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| Error::format(format!("bad header major: {line:?}")))?;
    let minor: u32 = parts
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| Error::format(format!("bad header minor: {line:?}")))?;
    if major != 5 {
        return Err(Error::format(format!(
            "unsupported NYTProf major version {major}.{minor} (only v5 supported)"
        )));
    }
    Ok((major, minor))
}

/// Read bytes from `pos` through and including the next `\n`.
fn read_line_including_nl(data: &[u8], pos: usize) -> Result<(Vec<u8>, usize)> {
    match find_newline(data, pos) {
        Some(nl) => Ok((data[pos..=nl].to_vec(), nl + 1)),
        None => Err(Error::UnexpectedEof {
            what: "line",
            offset: pos as u64,
        }),
    }
}

fn find_newline(data: &[u8], pos: usize) -> Option<usize> {
    data[pos..]
        .iter()
        .position(|&b| b == b'\n')
        .map(|i| pos + i)
}

/// Parse `key=value\n` (line may include trailing newline; value excludes it).
fn parse_key_value(line: &[u8], what: &str) -> Result<(String, String)> {
    let mut end = line.len();
    while end > 0 && (line[end - 1] == b'\n' || line[end - 1] == b'\r') {
        end -= 1;
    }
    let body = &line[..end];
    let eq = body.iter().position(|&b| b == b'=').ok_or_else(|| {
        Error::format(format!(
            "{what} malformed '{}'",
            String::from_utf8_lossy(body)
        ))
    })?;
    let key = String::from_utf8_lossy(&body[..eq]).into_owned();
    let value = String::from_utf8_lossy(&body[eq + 1..]).into_owned();
    Ok((key, value))
}

fn display_char(b: u8) -> char {
    if (0x20..0x7F).contains(&b) {
        b as char
    } else {
        '.'
    }
}

fn json_u64(v: u64) -> Value {
    Value::Number(Number::from(v))
}

/// Match oracle `sv_setuv` for TIME_* ticks: non-negative I32 as unsigned magnitude.
fn json_i32_as_oracle(v: i32) -> Value {
    if v >= 0 {
        json_u64(v as u64)
    } else {
        // Two's-complement bit pattern as UV (load_perl_callback 'i' → sv_setuv).
        json_u64(v as u32 as u64)
    }
}

/// Emit NV as JSON number; whole values in i64 range as integers (oracle JSON::PP).
fn json_nv(v: f64) -> Value {
    if v.is_finite() && v.fract() == 0.0 && v >= (i64::MIN as f64) && v <= (i64::MAX as f64) {
        return Value::Number(Number::from(v as i64));
    }
    Number::from_f64(v)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/v5")
            .join(name)
            .join("nytprof.out")
    }

    fn assert_basic_decode(path: &Path) {
        let events = decode_path(path).expect("decode");
        assert!(!events.is_empty(), "expected events");

        assert_eq!(events[0].tag, tags::VERSION);
        assert_eq!(events[0].args, vec![json_u64(5), json_u64(0)]);

        let tags_seen: HashSet<&str> = events.iter().map(|e| e.tag.as_str()).collect();
        assert!(tags_seen.contains(tags::START_DEFLATE), "START_DEFLATE");
        assert!(tags_seen.contains(tags::TIME_LINE), "TIME_LINE");
        assert!(tags_seen.contains(tags::PID_START), "PID_START");
        assert!(tags_seen.contains(tags::PID_END), "PID_END");
        assert!(
            events.iter().any(|e| e.tag == tags::PID_END),
            "must parse through PID_END"
        );
    }

    #[test]
    fn decode_default_calls1() {
        let path = fixture("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        assert_basic_decode(&path);

        let events = decode_path(&path).unwrap();
        // Oracle dump has 2473 real events + _END.
        assert_eq!(events.len(), 2473, "event count vs oracle (excl _END)");
    }

    /// NATIVE-DUMP-PARITY / DUMP-PARITY-EXPAND (decode side): tag multiplicities
    /// from `decode_path` must match the committed golden `readstream.jsonl`.
    /// Counts are loaded from both sides — not hard-coded alone.
    /// Full JSONL structural compare remains `tools/oracle/selftest_native_dump_parity.sh`.
    fn assert_native_dump_tag_counts_match_golden(fixture_name: &str) {
        use std::collections::HashMap;
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        let out = fixture(fixture_name);
        let jsonl = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/v5")
            .join(fixture_name)
            .join("readstream.jsonl");
        assert!(out.is_file(), "missing {}", out.display());
        assert!(jsonl.is_file(), "missing {}", jsonl.display());

        let binary = decode_path(&out).expect("decode nytprof.out");

        let mut golden_counts: HashMap<String, usize> = HashMap::new();
        let mut golden_real = 0usize;
        let f = File::open(&jsonl).expect("open golden jsonl");
        for (lineno, line) in BufReader::new(f).lines().enumerate() {
            let line = line.expect("read jsonl line");
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let v: serde_json::Value =
                serde_json::from_str(line).unwrap_or_else(|e| {
                    panic!("parse {} line {}: {e}", jsonl.display(), lineno + 1)
                });
            let tag = v
                .get("tag")
                .and_then(|t| t.as_str())
                .unwrap_or_else(|| panic!("missing tag at line {}", lineno + 1));
            if tag == tags::END {
                continue; // synthetic _END; binary decode omits it
            }
            golden_real += 1;
            *golden_counts.entry(tag.to_string()).or_insert(0) += 1;
        }

        assert_eq!(
            binary.len(),
            golden_real,
            "{fixture_name}: event count (excl _END) binary vs golden jsonl"
        );

        let mut binary_counts: HashMap<String, usize> = HashMap::new();
        for ev in &binary {
            *binary_counts.entry(ev.tag.clone()).or_insert(0) += 1;
        }

        // Critical multiplicity tags for dump parity (see native-dump-parity-mvp-v0).
        for tag in [tags::TIME_LINE, tags::TIME_BLOCK, tags::SUB_RETURN] {
            let g = *golden_counts.get(tag).unwrap_or(&0);
            let b = *binary_counts.get(tag).unwrap_or(&0);
            assert_eq!(
                b, g,
                "{fixture_name}: {tag} count: binary={b} golden={g} (from {})",
                jsonl.display()
            );
        }

        // Fixture-class sanity derived from *this* golden (not hard-coded counts).
        let tl = *golden_counts.get(tags::TIME_LINE).unwrap_or(&0);
        let tb = *golden_counts.get(tags::TIME_BLOCK).unwrap_or(&0);
        let sr = *golden_counts.get(tags::SUB_RETURN).unwrap_or(&0);
        assert!(
            tl + tb > 0,
            "{fixture_name}: expected TIME_LINE+TIME_BLOCK > 0 on golden"
        );
        assert!(
            sr > 0,
            "{fixture_name}: expected SUB_RETURN > 0 on golden"
        );

        // Known shapes for the DUMP-PARITY-EXPAND set (still derived from golden).
        match fixture_name {
            "default-calls1" | "calls2-default" => {
                assert!(
                    tl > 0,
                    "{fixture_name}: expected TIME_LINE > 0 on golden"
                );
                assert_eq!(
                    tb, 0,
                    "{fixture_name}: expected TIME_BLOCK == 0 on golden, got {tb}"
                );
            }
            "blocks-calls1" => {
                assert!(
                    tb > 0,
                    "{fixture_name}: expected TIME_BLOCK > 0 on golden"
                );
                assert_eq!(
                    tl, 0,
                    "{fixture_name}: expected TIME_LINE == 0 on golden, got {tl}"
                );
            }
            _ => {}
        }
    }

    #[test]
    fn native_dump_tag_counts_match_golden_default_calls1() {
        assert_native_dump_tag_counts_match_golden("default-calls1");
    }

    #[test]
    fn native_dump_tag_counts_match_golden_calls2_default() {
        assert_native_dump_tag_counts_match_golden("calls2-default");
    }

    #[test]
    fn native_dump_tag_counts_match_golden_blocks_calls1() {
        assert_native_dump_tag_counts_match_golden("blocks-calls1");
    }

    #[test]
    fn decode_default_calls2() {
        let path = fixture("default-calls2");
        assert!(path.is_file(), "missing fixture {}", path.display());
        assert_basic_decode(&path);

        let events = decode_path(&path).unwrap();
        assert_eq!(events.len(), 2500, "event count vs oracle (excl _END)");
        assert!(
            events.iter().any(|e| e.tag == tags::SUB_ENTRY),
            "calls2 should include SUB_ENTRY"
        );
    }

    #[test]
    fn unknown_binary_tag_errors() {
        // Display path (error formatting).
        let err = Error::UnsupportedTag {
            tag: 0x51,
            ch: 'Q',
            offset: 99,
        };
        let s = err.to_string();
        assert!(s.contains("0x51"), "{s}");
        assert!(s.contains("99"), "{s}");

        // Real decoder entry point: unsupported payload tag after valid header.
        let mut buf = b"NYTProf 5 0\n".to_vec();
        buf.push(b'Q'); // 0x51 — not a known wire tag
        let err = decode_all(&buf).expect_err("unsupported tag must Err");
        match err {
            Error::UnsupportedTag { tag, ch, offset } => {
                assert_eq!(tag, 0x51);
                assert_eq!(ch, 'Q');
                assert_eq!(offset, 12); // after "NYTProf 5 0\n"
            }
            other => panic!("expected UnsupportedTag, got {other:?}"),
        }
    }

    #[test]
    fn decode_empty_input_errors() {
        let err = decode_all(b"").expect_err("empty must Err");
        assert!(
            matches!(err, Error::Format(_)),
            "empty → Format, got {err:?}"
        );
    }

    #[test]
    fn decode_bad_header_errors() {
        let err = decode_all(b"NOTPROF 5 0\n").expect_err("bad magic must Err");
        match &err {
            Error::Format(msg) => assert!(
                msg.contains("bad header magic") || msg.contains("header"),
                "unexpected Format msg: {msg}"
            ),
            other => panic!("expected Format, got {other:?}"),
        }

        let err = decode_all(b"hello").expect_err("no newline header must Err");
        assert!(
            matches!(err, Error::Format(_)),
            "hello → Format, got {err:?}"
        );
    }

    #[test]
    fn decode_truncated_after_header_errors() {
        let path = fixture("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let bytes = std::fs::read(&path).expect("read fixture");
        assert!(bytes.len() > 20, "fixture too small");

        let err = decode_all(&bytes[..20]).expect_err("truncated header body must Err");
        // Header line is 12 bytes ("NYTProf 5 0\n"); remaining is a partial comment.
        assert!(
            matches!(
                err,
                Error::UnexpectedEof { .. } | Error::Format(_) | Error::Zlib(_)
            ),
            "truncated-after-header → Err kind, got {err:?}"
        );
    }

    #[test]
    fn decode_truncated_mid_file_errors() {
        let path = fixture("default-calls1");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let bytes = std::fs::read(&path).expect("read fixture");
        let half = bytes.len() / 2;
        assert!(half > 0, "fixture empty");

        let err = decode_all(&bytes[..half]).expect_err("mid-file truncate must Err");
        // Any Err is success (UnexpectedEof, Zlib, Format, …); must not Ok.
        let _ = err;
        // Also exercise decode_path on a truncated tempfile.
        let tmp = std::env::temp_dir().join(format!(
            "nytprof-format-v5-trunc-mid-{}.out",
            std::process::id()
        ));
        std::fs::write(&tmp, &bytes[..half]).expect("write temp");
        let path_err = decode_path(&tmp).expect_err("decode_path truncated must Err");
        let _ = path_err;
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn decode_garbage_tag_after_header_errors() {
        // Valid header + garbage binary tag byte (not a known wire tag).
        let mut buf = b"NYTProf 5 0\n".to_vec();
        buf.push(0xFF);
        let err = decode_all(&buf).expect_err("garbage tag must Err");
        match err {
            Error::UnsupportedTag { tag, offset, .. } => {
                assert_eq!(tag, 0xFF);
                assert_eq!(offset, 12);
            }
            other => panic!("expected UnsupportedTag, got {other:?}"),
        }
    }

    #[test]
    fn uncompressed_profile_roundtrip_tags() {
        // Synthetic: header + attr + PID_START/END without zlib.
        let mut buf = b"NYTProf 5 0\n:nv_size=8\n".to_vec();
        buf.push(wire::PID_START);
        buf.extend(crate::varint::encode_u32(123)); // pid
        buf.extend(crate::varint::encode_u32(1)); // ppid
        buf.extend(1.5f64.to_le_bytes());
        buf.push(wire::PID_END);
        buf.extend(crate::varint::encode_u32(123));
        buf.extend(2.5f64.to_le_bytes());

        let events = decode_all(&buf).expect("decode uncompressed");
        assert_eq!(events[0].tag, tags::VERSION);
        assert_eq!(events[1].tag, tags::ATTRIBUTE);
        assert_eq!(events[2].tag, tags::PID_START);
        assert_eq!(events[2].args[0], json_u64(123));
        assert_eq!(events[3].tag, tags::PID_END);
        assert_eq!(events.len(), 4);
    }
}
