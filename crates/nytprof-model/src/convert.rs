//! Strict **v5↔v6 profile conversion** (PR-C01 / TOOL-004 / TOOL-005 / FMT-013).
//!
//! Pipeline:
//! 1. Decode product input via [`decode_events_from_bytes`] (v5 or v6 dual dispatch)
//! 2. Project logical events with **strict representability** checks for the target
//! 3. Encode with format crate writers (v5: `encode_all_as_v5`; v6: absolute EVENT stand-in)
//!
//! # Strict guarantees
//!
//! - Default path **refuses** unrepresentable values (no silent truncation)
//! - Opt-in [`ConvertOptions::allow_lossy`] / CLI `--allow-lossy` truncates
//!   fractional NV toward 0 for v6 u64 fields (PID_*/ticks) and drops
//!   non-zero `NEW_FID` extras / `TIME_BLOCK.sub_line` (absolute body).
//!   Still refuses negative / non-finite / unknown tags.
//! - Successful v5 outputs must decode with the independent v5 decoder (old-tool shape)
//!
//! # Residuals
//!
//! - v6 output is **absolute EVENT** only (NONE codec); not packing / string-dict / multi-kind
//! - Non-zero extended `NEW_FID` fields and non-zero `TIME_BLOCK.sub_line` **refuse** on v5→v6
//!   (absolute body cannot represent them; no silent zeroing)
//! - Fractional NV→u64 refuse on the **strict** path; `--allow-lossy` truncates toward 0
//! - Not full oracle dual equality; packing/string-dict v6 out residual

use std::path::Path;

use nytprof_format_v6::chunk::codec;
use nytprof_format_v6::event_body::EventRecordSpec;
use nytprof_format_v6::{e3_standin_write_absolute, SUPPORTED_MAJOR};
use nytprof_types::{tags, Event};
use serde_json::Value;
use thiserror::Error;

use crate::v6_ingest::decode_events_from_bytes;
use crate::{ModelError, ProfileModel};

/// Target wire format for conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvertTarget {
    /// Oracle-compatible v5 (`NYTProf 5 0` text header + tags).
    V5,
    /// Product v6 absolute EVENT profile (`NYTPROF6` magic).
    V6,
}

impl ConvertTarget {
    /// Parse `v5` / `5` / `v6` / `6` (case-insensitive).
    pub fn parse(s: &str) -> Result<Self, ConvertError> {
        match s.trim().to_ascii_lowercase().as_str() {
            "v5" | "5" => Ok(ConvertTarget::V5),
            "v6" | "6" => Ok(ConvertTarget::V6),
            other => Err(ConvertError::BadTarget {
                got: other.to_string(),
            }),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ConvertTarget::V5 => "v5",
            ConvertTarget::V6 => "v6",
        }
    }
}

/// Convert knobs. Default is **strict** (same as [`ConvertOptions::strict`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ConvertOptions {
    /// When true, project fractional NV ticks/times to v6 `u64` by truncating
    /// toward zero. Negative / non-finite still fail closed.
    pub allow_lossy: bool,
}

impl ConvertOptions {
    pub fn strict() -> Self {
        Self { allow_lossy: false }
    }
}

/// Strict conversion errors (fail closed).
#[derive(Debug, Error)]
pub enum ConvertError {
    #[error("model/decode: {0}")]
    Model(#[from] ModelError),
    #[error("v5 encode: {0}")]
    V5Encode(String),
    #[error("v6 encode: {0}")]
    V6Encode(String),
    #[error("strict convert: {detail}")]
    Strict { detail: String },
    #[error("unknown convert target '{got}' (supported: v5, v6)")]
    BadTarget { got: String },
    #[error("io error {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

pub type ConvertResult<T> = std::result::Result<T, ConvertError>;

/// Convert product profile bytes to `target` under the **strict** path.
pub fn convert_bytes(input: &[u8], target: ConvertTarget) -> ConvertResult<Vec<u8>> {
    convert_bytes_with(input, target, ConvertOptions::strict())
}

/// Convert product profile bytes with explicit options (strict or `--allow-lossy`).
pub fn convert_bytes_with(
    input: &[u8],
    target: ConvertTarget,
    opts: ConvertOptions,
) -> ConvertResult<Vec<u8>> {
    let events = decode_events_from_bytes(input)?;
    encode_events_with(&events, target, opts)
}

/// Convert a profile file and write the result to `output_path` (strict).
pub fn convert_path(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    target: ConvertTarget,
) -> ConvertResult<()> {
    convert_path_with(input_path, output_path, target, ConvertOptions::strict())
}

/// Convert a profile file with explicit options.
pub fn convert_path_with(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    target: ConvertTarget,
    opts: ConvertOptions,
) -> ConvertResult<()> {
    let input_path = input_path.as_ref();
    let output_path = output_path.as_ref();
    let bytes = std::fs::read(input_path).map_err(|e| ConvertError::Io {
        path: input_path.display().to_string(),
        source: e,
    })?;
    let out = convert_bytes_with(&bytes, target, opts)?;
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| ConvertError::Io {
                path: parent.display().to_string(),
                source: e,
            })?;
        }
    }
    std::fs::write(output_path, out).map_err(|e| ConvertError::Io {
        path: output_path.display().to_string(),
        source: e,
    })?;
    Ok(())
}

/// Encode an already-decoded logical event stream to `target` (strict).
pub fn encode_events(events: &[Event], target: ConvertTarget) -> ConvertResult<Vec<u8>> {
    encode_events_with(events, target, ConvertOptions::strict())
}

/// Encode with explicit options.
pub fn encode_events_with(
    events: &[Event],
    target: ConvertTarget,
    opts: ConvertOptions,
) -> ConvertResult<Vec<u8>> {
    match target {
        ConvertTarget::V5 => encode_to_v5(events),
        ConvertTarget::V6 => encode_to_v6(events, opts),
    }
}

fn encode_to_v5(events: &[Event]) -> ConvertResult<Vec<u8>> {
    // Representability is enforced inside the v5 encoder (I32 ticks, U32, finite NV).
    nytprof_format_v5::encode_all_as_v5(events).map_err(|e| ConvertError::V5Encode(e.to_string()))
}

/// Build owned string storage + EventRecordSpec list, then absolute v6 encode.
fn encode_to_v6(events: &[Event], opts: ConvertOptions) -> ConvertResult<Vec<u8>> {
    // Hold string bytes for the lifetime of specs.
    let mut strings: Vec<Vec<u8>> = Vec::new();
    // Index into `strings` for each string field we push; we build specs in a second pass
    // using indices — actually EventRecordSpec needs & [u8], so we build strings first then specs.
    // Simpler: two-phase with indices recorded alongside event ops.
    #[derive(Clone)]
    enum Op {
        Version {
            major: u64,
            minor: u64,
        },
        Comment {
            si: usize,
        },
        Attribute {
            ki: usize,
            vi: usize,
        },
        Option {
            ki: usize,
            vi: usize,
        },
        StartDeflate,
        PidStart {
            pid: u64,
            ppid: u64,
            start_time: u64,
        },
        PidEnd {
            pid: u64,
            end_time: u64,
        },
        NewFid {
            fid: u64,
            si: usize,
        },
        TimeLine {
            fid: u64,
            line: u64,
            ticks: u64,
        },
        TimeBlock {
            fid: u64,
            line: u64,
            block_line: u64,
            ticks: u64,
        },
        Discount,
        SubEntry {
            caller_fid: u64,
            caller_line: u64,
        },
        SubReturn {
            depth: u64,
            incl: u64,
            excl: u64,
            si: usize,
        },
        SubInfo {
            fid: u64,
            first_line: u64,
            last_line: u64,
            si: usize,
        },
        SubCallers {
            fid: u64,
            line: u64,
            count: u64,
            incl: u64,
            excl: u64,
            reci: u64,
            rec_depth: u64,
            called_i: usize,
            caller_i: usize,
        },
        SrcLine {
            fid: u64,
            line: u64,
            si: usize,
        },
    }

    let mut ops: Vec<Op> = Vec::with_capacity(events.len());
    let mut pushed_version = false;

    for ev in events {
        match ev.tag.as_str() {
            tags::END => continue,
            tags::VERSION => {
                // Target v6 always emits SUPPORTED_MAJOR / minor 0 once.
                if !pushed_version {
                    ops.push(Op::Version {
                        major: SUPPORTED_MAJOR as u64,
                        minor: 0,
                    });
                    pushed_version = true;
                }
                // Allow source major 5 (v5 input) or 6 (v6 input); reject others.
                let maj = arg_u64(&ev.args, 0, "VERSION", "major", ev.seq)?;
                let min = arg_u64(&ev.args, 1, "VERSION", "minor", ev.seq)?;
                if maj != 5 && maj != SUPPORTED_MAJOR as u64 {
                    return Err(ConvertError::Strict {
                        detail: format!(
                            "seq {}: VERSION {maj}.{min} not projectable to v6 major {}",
                            ev.seq, SUPPORTED_MAJOR
                        ),
                    });
                }
            }
            tags::COMMENT => {
                let t = arg_str(&ev.args, 0, "COMMENT", "text", ev.seq)?;
                let si = push_str(&mut strings, t);
                ops.push(Op::Comment { si });
            }
            tags::ATTRIBUTE => {
                let k = arg_str(&ev.args, 0, "ATTRIBUTE", "key", ev.seq)?;
                let v = arg_str(&ev.args, 1, "ATTRIBUTE", "value", ev.seq)?;
                let ki = push_str(&mut strings, k);
                let vi = push_str(&mut strings, v);
                ops.push(Op::Attribute { ki, vi });
            }
            tags::OPTION => {
                let k = arg_str(&ev.args, 0, "OPTION", "key", ev.seq)?;
                let v = arg_str(&ev.args, 1, "OPTION", "value", ev.seq)?;
                let ki = push_str(&mut strings, k);
                let vi = push_str(&mut strings, v);
                ops.push(Op::Option { ki, vi });
            }
            tags::START_DEFLATE => ops.push(Op::StartDeflate),
            tags::PID_START => {
                let pid = arg_u64(&ev.args, 0, "PID_START", "pid", ev.seq)?;
                let ppid = arg_u64(&ev.args, 1, "PID_START", "ppid", ev.seq)?;
                let start_time = arg_exact_u64_ticks(
                    &ev.args,
                    2,
                    "PID_START",
                    "start_time",
                    ev.seq,
                    opts.allow_lossy,
                )?;
                ops.push(Op::PidStart {
                    pid,
                    ppid,
                    start_time,
                });
            }
            tags::PID_END => {
                let pid = arg_u64(&ev.args, 0, "PID_END", "pid", ev.seq)?;
                let end_time = arg_exact_u64_ticks(
                    &ev.args,
                    1,
                    "PID_END",
                    "end_time",
                    ev.seq,
                    opts.allow_lossy,
                )?;
                ops.push(Op::PidEnd { pid, end_time });
            }
            tags::NEW_FID => {
                let fid = arg_u64(&ev.args, 0, "NEW_FID", "fid", ev.seq)?;
                // Strict: extended v5 fields must be zero (v6 absolute body has fid+name only).
                if ev.args.len() >= 7 {
                    for (idx, field) in [
                        (1, "eval_fid"),
                        (2, "eval_line"),
                        (3, "flags"),
                        (4, "size"),
                        (5, "mtime"),
                    ] {
                        let v = arg_u64(&ev.args, idx, "NEW_FID", field, ev.seq)?;
                        if v != 0 {
                            if opts.allow_lossy {
                                // Absolute v6 body is fid+name only; drop extras.
                                continue;
                            }
                            return Err(ConvertError::Strict {
                                detail: format!(
                                    "seq {}: NEW_FID.{field}={v} not representable on v6 absolute body (strict refuse non-zero)",
                                    ev.seq
                                ),
                            });
                        }
                    }
                    let name = arg_str(&ev.args, 6, "NEW_FID", "name", ev.seq)?;
                    let si = push_str(&mut strings, name);
                    ops.push(Op::NewFid { fid, si });
                } else if ev.args.len() >= 2 {
                    // Already short form (fid, name) — unusual but accept.
                    let name = arg_str(&ev.args, 1, "NEW_FID", "name", ev.seq)?;
                    let si = push_str(&mut strings, name);
                    ops.push(Op::NewFid { fid, si });
                } else {
                    return Err(ConvertError::Strict {
                        detail: format!("seq {}: NEW_FID needs fid+name args", ev.seq),
                    });
                }
            }
            tags::TIME_LINE => {
                let ticks = arg_exact_u64_ticks(
                    &ev.args,
                    0,
                    "TIME_LINE",
                    "ticks",
                    ev.seq,
                    opts.allow_lossy,
                )?;
                let fid = arg_u64(&ev.args, 1, "TIME_LINE", "fid", ev.seq)?;
                let line = arg_u64(&ev.args, 2, "TIME_LINE", "line", ev.seq)?;
                ops.push(Op::TimeLine { fid, line, ticks });
            }
            tags::TIME_BLOCK => {
                let ticks = arg_exact_u64_ticks(
                    &ev.args,
                    0,
                    "TIME_BLOCK",
                    "ticks",
                    ev.seq,
                    opts.allow_lossy,
                )?;
                let fid = arg_u64(&ev.args, 1, "TIME_BLOCK", "fid", ev.seq)?;
                let line = arg_u64(&ev.args, 2, "TIME_BLOCK", "line", ev.seq)?;
                let block_line = arg_u64(&ev.args, 3, "TIME_BLOCK", "block_line", ev.seq)?;
                // v6 absolute body has no sub_line. Strict: refuse non-zero (no silent zero).
                // Zero sub_line (or absent arg) is representable and projects cleanly.
                if ev.args.len() > 4 {
                    let sub_line = arg_u64(&ev.args, 4, "TIME_BLOCK", "sub_line", ev.seq)?;
                    if sub_line != 0 && !opts.allow_lossy {
                        return Err(ConvertError::Strict {
                            detail: format!(
                                "seq {}: TIME_BLOCK.sub_line={sub_line} not representable on v6 absolute body (strict refuse non-zero; no silent zeroing)",
                                ev.seq
                            ),
                        });
                    }
                    // --allow-lossy: drop non-zero sub_line (absolute body has no field).
                }
                ops.push(Op::TimeBlock {
                    fid,
                    line,
                    block_line,
                    ticks,
                });
            }
            tags::DISCOUNT => ops.push(Op::Discount),
            tags::SUB_ENTRY => {
                let caller_fid = arg_u64(&ev.args, 0, "SUB_ENTRY", "caller_fid", ev.seq)?;
                let caller_line = arg_u64(&ev.args, 1, "SUB_ENTRY", "caller_line", ev.seq)?;
                ops.push(Op::SubEntry {
                    caller_fid,
                    caller_line,
                });
            }
            tags::SUB_RETURN => {
                let depth = arg_u64(&ev.args, 0, "SUB_RETURN", "depth", ev.seq)?;
                let incl = arg_exact_u64_ticks(
                    &ev.args,
                    1,
                    "SUB_RETURN",
                    "incl",
                    ev.seq,
                    opts.allow_lossy,
                )?;
                let excl = arg_exact_u64_ticks(
                    &ev.args,
                    2,
                    "SUB_RETURN",
                    "excl",
                    ev.seq,
                    opts.allow_lossy,
                )?;
                let name = arg_str(&ev.args, 3, "SUB_RETURN", "subname", ev.seq)?;
                let si = push_str(&mut strings, name);
                ops.push(Op::SubReturn {
                    depth,
                    incl,
                    excl,
                    si,
                });
            }
            tags::SUB_INFO => {
                let fid = arg_u64(&ev.args, 0, "SUB_INFO", "fid", ev.seq)?;
                let first_line = arg_u64(&ev.args, 1, "SUB_INFO", "first_line", ev.seq)?;
                let last_line = arg_u64(&ev.args, 2, "SUB_INFO", "last_line", ev.seq)?;
                let name = arg_str(&ev.args, 3, "SUB_INFO", "name", ev.seq)?;
                let si = push_str(&mut strings, name);
                ops.push(Op::SubInfo {
                    fid,
                    first_line,
                    last_line,
                    si,
                });
            }
            tags::SUB_CALLERS => {
                let fid = arg_u64(&ev.args, 0, "SUB_CALLERS", "fid", ev.seq)?;
                let line = arg_u64(&ev.args, 1, "SUB_CALLERS", "line", ev.seq)?;
                let count = arg_u64(&ev.args, 2, "SUB_CALLERS", "count", ev.seq)?;
                let incl = arg_exact_u64_ticks(
                    &ev.args,
                    3,
                    "SUB_CALLERS",
                    "incl",
                    ev.seq,
                    opts.allow_lossy,
                )?;
                let excl = arg_exact_u64_ticks(
                    &ev.args,
                    4,
                    "SUB_CALLERS",
                    "excl",
                    ev.seq,
                    opts.allow_lossy,
                )?;
                let reci = arg_exact_u64_ticks(
                    &ev.args,
                    5,
                    "SUB_CALLERS",
                    "reci",
                    ev.seq,
                    opts.allow_lossy,
                )?;
                let rec_depth = arg_u64(&ev.args, 6, "SUB_CALLERS", "rec_depth", ev.seq)?;
                let called = arg_str(&ev.args, 7, "SUB_CALLERS", "called", ev.seq)?;
                let caller = arg_str(&ev.args, 8, "SUB_CALLERS", "caller", ev.seq)?;
                let called_i = push_str(&mut strings, called);
                let caller_i = push_str(&mut strings, caller);
                ops.push(Op::SubCallers {
                    fid,
                    line,
                    count,
                    incl,
                    excl,
                    reci,
                    rec_depth,
                    called_i,
                    caller_i,
                });
            }
            tags::SRC_LINE => {
                let fid = arg_u64(&ev.args, 0, "SRC_LINE", "fid", ev.seq)?;
                let line = arg_u64(&ev.args, 1, "SRC_LINE", "line", ev.seq)?;
                let text = arg_str(&ev.args, 2, "SRC_LINE", "text", ev.seq)?;
                let si = push_str(&mut strings, text);
                ops.push(Op::SrcLine { fid, line, si });
            }
            other => {
                return Err(ConvertError::Strict {
                    detail: format!("seq {}: tag '{other}' has no strict v6 mapping", ev.seq),
                });
            }
        }
    }

    if !pushed_version {
        ops.insert(
            0,
            Op::Version {
                major: SUPPORTED_MAJOR as u64,
                minor: 0,
            },
        );
    }

    let specs: Vec<EventRecordSpec<'_>> = ops
        .iter()
        .map(|op| match op {
            Op::Version { major, minor } => EventRecordSpec::Version {
                major: *major,
                minor: *minor,
            },
            Op::Comment { si } => EventRecordSpec::Comment {
                string_id: 0,
                string_flags: 0,
                text: strings[*si].as_slice(),
            },
            Op::Attribute { ki, vi } => EventRecordSpec::Attribute {
                key_string_id: 0,
                key_string_flags: 0,
                key: strings[*ki].as_slice(),
                value_string_id: 0,
                value_string_flags: 0,
                value: strings[*vi].as_slice(),
            },
            Op::Option { ki, vi } => EventRecordSpec::Option {
                key_string_id: 0,
                key_string_flags: 0,
                key: strings[*ki].as_slice(),
                value_string_id: 0,
                value_string_flags: 0,
                value: strings[*vi].as_slice(),
            },
            Op::StartDeflate => EventRecordSpec::StartDeflate,
            Op::PidStart {
                pid,
                ppid,
                start_time,
            } => EventRecordSpec::PidStart {
                pid: *pid,
                ppid: *ppid,
                start_time: *start_time,
            },
            Op::PidEnd { pid, end_time } => EventRecordSpec::PidEnd {
                pid: *pid,
                end_time: *end_time,
            },
            Op::NewFid { fid, si } => EventRecordSpec::NewFid {
                fid: *fid,
                string_id: 0,
                string_flags: 0,
                filename: strings[*si].as_slice(),
            },
            Op::TimeLine { fid, line, ticks } => EventRecordSpec::TimeLine {
                fid: *fid,
                line: *line,
                ticks: *ticks,
            },
            Op::TimeBlock {
                fid,
                line,
                block_line,
                ticks,
            } => EventRecordSpec::TimeBlock {
                fid: *fid,
                line: *line,
                block_line: *block_line,
                ticks: *ticks,
            },
            Op::Discount => EventRecordSpec::Discount,
            Op::SubEntry {
                caller_fid,
                caller_line,
            } => EventRecordSpec::SubEntry {
                caller_fid: *caller_fid,
                caller_line: *caller_line,
            },
            Op::SubReturn {
                depth,
                incl,
                excl,
                si,
            } => EventRecordSpec::SubReturn {
                depth: *depth,
                incl: *incl,
                excl: *excl,
                string_id: 0,
                string_flags: 0,
                subname: strings[*si].as_slice(),
            },
            Op::SubInfo {
                fid,
                first_line,
                last_line,
                si,
            } => EventRecordSpec::SubInfo {
                fid: *fid,
                first_line: *first_line,
                last_line: *last_line,
                string_id: 0,
                string_flags: 0,
                name: strings[*si].as_slice(),
            },
            Op::SubCallers {
                fid,
                line,
                count,
                incl,
                excl,
                reci,
                rec_depth,
                called_i,
                caller_i,
            } => EventRecordSpec::SubCallers {
                fid: *fid,
                line: *line,
                count: *count,
                incl: *incl,
                excl: *excl,
                reci: *reci,
                rec_depth: *rec_depth,
                called_string_id: 0,
                called_string_flags: 0,
                called: strings[*called_i].as_slice(),
                caller_string_id: 0,
                caller_string_flags: 0,
                caller: strings[*caller_i].as_slice(),
            },
            Op::SrcLine { fid, line, si } => EventRecordSpec::SrcLine {
                fid: *fid,
                line: *line,
                string_id: 0,
                string_flags: 0,
                text: strings[*si].as_slice(),
            },
        })
        .collect();

    e3_standin_write_absolute(&specs, codec::NONE)
        .map_err(|e| ConvertError::V6Encode(e.to_string()))
}

fn push_str(strings: &mut Vec<Vec<u8>>, s: &str) -> usize {
    let i = strings.len();
    strings.push(s.as_bytes().to_vec());
    i
}

fn arg_at<'a>(
    args: &'a [Value],
    i: usize,
    tag: &str,
    field: &str,
    seq: u64,
) -> ConvertResult<&'a Value> {
    args.get(i).ok_or_else(|| ConvertError::Strict {
        detail: format!("seq {seq}: {tag} missing arg[{i}] ({field})"),
    })
}

fn arg_u64(args: &[Value], i: usize, tag: &str, field: &str, seq: u64) -> ConvertResult<u64> {
    let v = arg_at(args, i, tag, field, seq)?;
    match v {
        Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                Ok(u)
            } else if let Some(i64v) = n.as_i64() {
                if i64v < 0 {
                    Err(ConvertError::Strict {
                        detail: format!("seq {seq}: {tag}.{field} negative ({i64v})"),
                    })
                } else {
                    Ok(i64v as u64)
                }
            } else {
                Err(ConvertError::Strict {
                    detail: format!("seq {seq}: {tag}.{field} not an integer ({v})"),
                })
            }
        }
        _ => Err(ConvertError::Strict {
            detail: format!("seq {seq}: {tag}.{field} not a number ({v})"),
        }),
    }
}

/// Strict integer-tick projection for v6 u64 fields (and v5 NV that must be exact integers).
fn arg_exact_u64_ticks(
    args: &[Value],
    i: usize,
    tag: &str,
    field: &str,
    seq: u64,
    allow_lossy: bool,
) -> ConvertResult<u64> {
    let v = arg_at(args, i, tag, field, seq)?;
    match v {
        Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                return Ok(u);
            }
            if let Some(i64v) = n.as_i64() {
                if i64v < 0 {
                    return Err(ConvertError::Strict {
                        detail: format!("seq {seq}: {tag}.{field}={i64v} negative (strict refuse)"),
                    });
                }
                return Ok(i64v as u64);
            }
            if let Some(f) = n.as_f64() {
                if !f.is_finite() || f < 0.0 {
                    return Err(ConvertError::Strict {
                        detail: format!(
                            "seq {seq}: {tag}.{field}={f} not a non-negative finite integer"
                        ),
                    });
                }
                if f.fract() != 0.0 {
                    if allow_lossy {
                        // Explicit lossy: truncate toward zero (oracle wall NV → u64).
                        return Ok(f.trunc() as u64);
                    }
                    return Err(ConvertError::Strict {
                        detail: format!(
                            "seq {seq}: {tag}.{field}={f} is fractional (strict refuse; no lossy NV→u64)"
                        ),
                    });
                }
                if f > u64::MAX as f64 {
                    return Err(ConvertError::Strict {
                        detail: format!("seq {seq}: {tag}.{field}={f} exceeds u64"),
                    });
                }
                return Ok(f as u64);
            }
            Err(ConvertError::Strict {
                detail: format!("seq {seq}: {tag}.{field} not representable ({v})"),
            })
        }
        _ => Err(ConvertError::Strict {
            detail: format!("seq {seq}: {tag}.{field} not a number ({v})"),
        }),
    }
}

fn arg_str<'a>(
    args: &'a [Value],
    i: usize,
    tag: &str,
    field: &str,
    seq: u64,
) -> ConvertResult<&'a str> {
    let v = arg_at(args, i, tag, field, seq)?;
    match v {
        Value::String(s) => Ok(s.as_str()),
        _ => Err(ConvertError::Strict {
            detail: format!("seq {seq}: {tag}.{field} not a string ({v})"),
        }),
    }
}

/// Convenience: convert then load both sides as models for aggregate checks.
pub fn convert_and_models(
    input: &[u8],
    target: ConvertTarget,
) -> ConvertResult<(Vec<u8>, ProfileModel, ProfileModel)> {
    let src_model = ProfileModel::from_bytes(input)?;
    let out = convert_bytes(input, target)?;
    let dst_model = ProfileModel::from_bytes(&out)?;
    Ok((out, src_model, dst_model))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::e4_v0_aggregates_equal;
    use std::path::PathBuf;

    fn dual(stem: &str, side: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/e4/dual-sink")
            .join(format!("{stem}_{side}.nytprof"))
    }

    fn fixture_v5(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/v5")
            .join(name)
            .join("nytprof.out")
    }

    #[test]
    fn m4_v5_to_v6_aggregates_equal() {
        let path = dual("m4", "v5");
        assert!(path.is_file(), "missing {}", path.display());
        let bytes = std::fs::read(&path).unwrap();
        let (out, src, dst) = convert_and_models(&bytes, ConvertTarget::V6).expect("convert");
        assert!(out.starts_with(b"NYTPROF6"));
        // total_events may differ (auto-VERSION / START_DEFLATE inject) — skip total.
        e4_v0_aggregates_equal(&src, &dst, false).expect("E4 after v5→v6");
    }

    #[test]
    fn m4_v6_to_v5_aggregates_equal() {
        let path = dual("m4", "v6");
        assert!(path.is_file(), "missing {}", path.display());
        let bytes = std::fs::read(&path).unwrap();
        let (out, src, dst) = convert_and_models(&bytes, ConvertTarget::V5).expect("convert");
        assert!(out.starts_with(b"NYTProf 5 0\n"));
        // Independent v5 decoder must accept (old-tool shape).
        let _ = nytprof_format_v5::decode_all(&out).expect("v5 decoder on converted");
        e4_v0_aggregates_equal(&src, &dst, false).expect("E4 after v6→v5");
    }

    #[test]
    fn default_calls1_dual_round_trip_v5_v6_v5() {
        let path = dual("default_calls1", "v5");
        let bytes = std::fs::read(&path).unwrap();
        let v6 = convert_bytes(&bytes, ConvertTarget::V6).expect("to v6");
        let v5b = convert_bytes(&v6, ConvertTarget::V5).expect("back to v5");
        let a = ProfileModel::from_bytes(&bytes).unwrap();
        let b = ProfileModel::from_bytes(&v5b).unwrap();
        e4_v0_aggregates_equal(&a, &b, false).expect("round-trip aggregates");
        nytprof_format_v5::decode_all(&v5b).expect("old-tool shape decode");
    }

    #[test]
    fn calls2_dual_both_directions() {
        // calls2 has zero sub_line; both directions are representable on strict path.
        for (side, target) in [("v5", ConvertTarget::V6), ("v6", ConvertTarget::V5)] {
            let path = dual("calls2_default", side);
            let bytes = std::fs::read(&path).unwrap();
            let (out, src, dst) = convert_and_models(&bytes, target).unwrap_or_else(|e| {
                panic!("calls2_default/{side}→{target:?}: {e}");
            });
            assert!(!out.is_empty());
            e4_v0_aggregates_equal(&src, &dst, false)
                .unwrap_or_else(|e| panic!("calls2_default/{side}: {e}"));
        }
    }

    #[test]
    fn blocks_v6_to_v5_ok_v5_to_v6_refuses_nonzero_sub_line() {
        // v6 side has no sub_line → v5 is fine.
        let v6_path = dual("blocks_calls1", "v6");
        let bytes = std::fs::read(&v6_path).unwrap();
        let (out, src, dst) = convert_and_models(&bytes, ConvertTarget::V5).expect("blocks v6→v5");
        assert!(out.starts_with(b"NYTProf 5 0\n"));
        e4_v0_aggregates_equal(&src, &dst, false).expect("blocks v6→v5 aggregates");

        // v5 dual-sink blocks has non-zero sub_line → strict refuse (no silent zero).
        let v5_path = dual("blocks_calls1", "v5");
        let v5_bytes = std::fs::read(&v5_path).unwrap();
        let err = convert_bytes(&v5_bytes, ConvertTarget::V6)
            .expect_err("blocks v5→v6 must refuse non-zero sub_line");
        let msg = err.to_string();
        assert!(
            msg.contains("sub_line") && (msg.contains("strict") || msg.contains("refuse")),
            "expected sub_line refuse, got: {msg}"
        );
    }

    #[test]
    fn strict_refuse_nonzero_sub_line_to_v6() {
        use serde_json::json;
        let events = vec![
            Event::new(0, tags::VERSION, vec![json!(5), json!(0)]),
            Event::new(
                1,
                tags::TIME_BLOCK,
                // ticks, fid, line, block_line, sub_line=3
                vec![json!(5), json!(1), json!(5), json!(4), json!(3)],
            ),
        ];
        let err = encode_events(&events, ConvertTarget::V6).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("sub_line"),
            "expected sub_line refuse, got: {msg}"
        );
    }

    #[test]
    fn strict_refuse_nonzero_new_fid_eval_to_v6() {
        use serde_json::json;
        let events = vec![
            Event::new(0, tags::VERSION, vec![json!(5), json!(0)]),
            Event::new(
                1,
                tags::NEW_FID,
                // fid, eval_fid=1 (non-zero), eval_line, flags, size, mtime, name
                vec![
                    json!(1),
                    json!(1),
                    json!(0),
                    json!(0),
                    json!(0),
                    json!(0),
                    json!("workload.pl"),
                ],
            ),
        ];
        let err = encode_events(&events, ConvertTarget::V6).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("eval_fid") || msg.contains("NEW_FID"),
            "expected NEW_FID refuse, got: {msg}"
        );
    }

    #[test]
    fn strict_refuse_unknown_tag() {
        use serde_json::json;
        let events = vec![
            Event::new(0, tags::VERSION, vec![json!(5), json!(0)]),
            Event::new(1, "NOT_A_TAG", vec![json!(1)]),
        ];
        let err = encode_events(&events, ConvertTarget::V6).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("NOT_A_TAG") || msg.contains("no strict"),
            "expected unknown tag refuse, got: {msg}"
        );
        let err5 = encode_events(&events, ConvertTarget::V5).unwrap_err();
        let msg5 = err5.to_string();
        assert!(
            msg5.contains("NOT_A_TAG") || msg5.contains("unsupported"),
            "expected v5 unknown tag refuse, got: {msg5}"
        );
    }

    #[test]
    fn strict_refuse_nv_mantissa_to_v5() {
        use serde_json::json;
        let bad = (1u64 << 53) + 1;
        let events = vec![
            Event::new(0, tags::VERSION, vec![json!(6), json!(0)]),
            Event::new(
                1,
                tags::SUB_RETURN,
                vec![json!(1), json!(bad), json!(0u64), json!("main::x")],
            ),
        ];
        let err = encode_events(&events, ConvertTarget::V5).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("exactly representable")
                || msg.contains("mantissa")
                || msg.contains("v5 encode"),
            "expected mantissa refuse, got: {msg}"
        );
    }

    #[test]
    fn zero_sub_line_time_block_to_v6_ok() {
        use serde_json::json;
        let events = vec![
            Event::new(0, tags::VERSION, vec![json!(5), json!(0)]),
            Event::new(
                1,
                tags::TIME_BLOCK,
                vec![json!(5), json!(1), json!(5), json!(4), json!(0)],
            ),
            Event::new(2, tags::PID_END, vec![json!(1), json!(0)]),
        ];
        let wire = encode_events(&events, ConvertTarget::V6).expect("zero sub_line ok");
        assert!(wire.starts_with(b"NYTPROF6"));
    }

    #[test]
    fn strict_refuse_fractional_sub_return_to_v6() {
        use serde_json::json;
        let events = vec![
            Event::new(0, tags::VERSION, vec![json!(5), json!(0)]),
            Event::new(
                1,
                tags::SUB_RETURN,
                vec![json!(1), json!(1.5), json!(0.5), json!("main::x")],
            ),
        ];
        let err = encode_events(&events, ConvertTarget::V6).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("fractional") || msg.contains("strict"),
            "unexpected: {msg}"
        );
    }

    #[test]
    fn strict_refuse_ticks_overflow_to_v5() {
        use serde_json::json;
        let events = vec![
            Event::new(0, tags::VERSION, vec![json!(6), json!(0)]),
            Event::new(
                1,
                tags::TIME_LINE,
                vec![json!((i32::MAX as i64) + 1), json!(1), json!(1)],
            ),
        ];
        let err = encode_events(&events, ConvertTarget::V5).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("i32") || msg.contains("strict") || msg.contains("v5 encode"),
            "unexpected: {msg}"
        );
    }

    #[test]
    fn oracle_default_calls1_strict_v5_to_v6_refuses_fractional_pid_time() {
        // Residual honesty: oracle wall-clock PID_* NV is fractional seconds.
        // Strict path refuses non-integer projection to v6 u64 (no silent lossy).
        let path = fixture_v5("default-calls1");
        if !path.is_file() {
            return;
        }
        let bytes = std::fs::read(&path).unwrap();
        let err = convert_bytes(&bytes, ConvertTarget::V6).expect_err("strict refuse wall NV");
        let msg = err.to_string();
        assert!(
            msg.contains("fractional") || msg.contains("PID_"),
            "expected fractional PID time refuse, got: {msg}"
        );
    }

    #[test]
    fn oracle_default_calls1_v5_identity_encode_round_trip() {
        // v5→v5 re-encode (same target) stays on integer/NV wire and must load.
        let path = fixture_v5("default-calls1");
        if !path.is_file() {
            return;
        }
        let bytes = std::fs::read(&path).unwrap();
        let out = convert_bytes(&bytes, ConvertTarget::V5).expect("v5→v5");
        assert!(out.starts_with(b"NYTProf 5 0\n"));
        nytprof_format_v5::decode_all(&out).expect("old-tool shape decode");
        let a = ProfileModel::from_bytes(&bytes).unwrap();
        let b = ProfileModel::from_bytes(&out).unwrap();
        e4_v0_aggregates_equal(&a, &b, false).expect("v5 identity aggregates");
        assert_eq!(a.sub_total("main::leaf").map(|t| t.returns), Some(15));
        assert_eq!(b.sub_total("main::leaf").map(|t| t.returns), Some(15));
    }

    #[test]
    fn oracle_default_calls1_allow_lossy_v5_to_v6() {
        let path = fixture_v5("default-calls1");
        assert!(path.is_file(), "missing {}", path.display());
        let bytes = std::fs::read(&path).unwrap();
        convert_bytes(&bytes, ConvertTarget::V6).expect_err("strict default must still refuse");
        let opts = ConvertOptions { allow_lossy: true };
        let out = convert_bytes_with(&bytes, ConvertTarget::V6, opts)
            .expect("allow_lossy must convert oracle fractional NV");
        assert!(
            out.starts_with(b"NYTPROF6"),
            "lossy convert must write NYTPROF6"
        );
        let dst = ProfileModel::from_bytes(&out).expect("load lossy v6");
        assert_eq!(dst.sub_total("main::leaf").map(|t| t.returns), Some(15));
        assert_eq!(dst.sub_total("main::mid").map(|t| t.returns), Some(3));
    }
}
