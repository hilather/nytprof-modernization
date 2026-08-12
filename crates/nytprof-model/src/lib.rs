//! Compact profile model and exact aggregation (first-slice MVP).
//!
//! Aggregation rules: `docs/schemas/aggregate-comparison-v0.md` (A1–A9).
//! Built by replaying the ordered logical event stream from the v5 decoder,
//! product v6 always-inflate path, or oracle `readstream.jsonl` dumps.

use std::collections::HashMap;
use std::path::Path;

use nytprof_format_v5::{self, Error as DecodeError};
use nytprof_types::{tags, Event};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

mod v6_ingest;

pub use v6_ingest::{decode_events_from_bytes, decode_events_from_path, owned_records_to_events};

/// Errors while building a [`ProfileModel`].
#[derive(Debug, Error)]
pub enum ModelError {
    #[error(transparent)]
    Decode(#[from] DecodeError),
    #[error("v6 decode: {0}")]
    DecodeV6(String),
    #[error("io error reading {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("unsupported profile format: {detail}")]
    UnsupportedProfile { detail: String },
    #[error("invalid {tag} args at seq {seq}: {detail}")]
    InvalidArgs {
        tag: String,
        seq: u64,
        detail: String,
    },
}

pub type Result<T> = std::result::Result<T, ModelError>;

/// Per-location totals from `TIME_LINE` / `TIME_BLOCK` events (A4 / A4b).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineTotal {
    pub calls: u64,
    /// Sum of tick values from each contributing timing event at this location.
    pub ticks: i64,
}

/// Alias used by some call sites / docs.
pub type LineTotals = LineTotal;

/// Per-subroutine totals from `SUB_RETURN` events (A5).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct SubTotal {
    pub returns: u64,
    /// Inclusive time sum (`incl` NV args).
    pub incl: f64,
    /// Exclusive time sum (`excl` NV args).
    pub excl: f64,
}

/// Alias matching schema wording (`incl_ticks` / `excl_ticks` as fields via accessors).
pub type SubReturnTotals = SubTotal;

impl SubTotal {
    /// Inclusive tick sum (schema name).
    #[inline]
    pub fn incl_ticks(self) -> f64 {
        self.incl
    }

    /// Exclusive tick sum (schema name).
    #[inline]
    pub fn excl_ticks(self) -> f64 {
        self.excl
    }
}

/// Aggregated call-edge totals from `SUB_CALLERS` events (A7).
///
/// Keyed on `ProfileModel` by `(caller, called)`. Multiple sites for the same
/// edge merge by summing counts/times and taking the max recursion depth.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct CallEdgeTotal {
    pub count: u64,
    pub incl: f64,
    pub excl: f64,
    pub reci: f64,
    pub max_rec_depth: u32,
    /// Number of `SUB_CALLERS` records merged into this edge.
    pub sites: u64,
}

/// Subroutine definition range from `SUB_INFO` events (A9).
///
/// ReadStream args order: `[fid, first_line, last_line, name]`. Last write
/// wins when the same name appears more than once.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubDef {
    pub fid: u32,
    pub first_line: u32,
    pub last_line: u32,
}

/// Compact aggregated profile model.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProfileModel {
    /// Profile attributes (`ATTRIBUTE` key → value).
    pub attributes: HashMap<String, String>,
    /// Profiler options (`OPTION` key → value).
    pub options: HashMap<String, String>,
    /// File id → source path from `NEW_FID`.
    pub files: HashMap<u32, String>,

    /// A1 — count of `TIME_LINE` events.
    pub time_line_events: u64,
    /// A2 — count of `TIME_BLOCK` events.
    pub time_block_events: u64,
    /// A3 — count of `DISCOUNT` events.
    pub discount_events: u64,
    /// Count of `SUB_ENTRY` events (present in calls=1 profiles).
    pub sub_entry_events: u64,
    /// Count of `SUB_RETURN` events.
    pub sub_return_events: u64,
    /// Count of `NEW_FID` events.
    pub new_fid_events: u64,
    /// Count of `SUB_CALLERS` events (A7 stream count).
    pub sub_callers_events: u64,
    /// Count of `SRC_LINE` events (A8 stream count).
    pub src_line_events: u64,
    /// Count of `SUB_INFO` events (A9 stream count).
    pub sub_info_events: u64,
    /// Count of `PID_START` events (process lifecycle; used for completeness).
    pub pid_start_events: u64,
    /// Count of `PID_END` events (process lifecycle; used for completeness).
    pub pid_end_events: u64,
    /// Total logical events folded into the model.
    pub total_events: u64,

    /// A4 — `(fid, line) → { calls, ticks }` from `TIME_LINE` and `TIME_BLOCK`
    /// (both contribute via the statement `line` field).
    pub line_totals: HashMap<(u32, u32), LineTotal>,
    /// A4b — `(fid, block_line) → { calls, ticks }` from `TIME_BLOCK` only
    /// (event args: ticks, fid, line, block_line, sub_line).
    pub block_line_totals: HashMap<(u32, u32), LineTotal>,
    /// A5 — `subname → { returns, incl, excl }` from `SUB_RETURN`.
    pub sub_return_totals: HashMap<String, SubTotal>,
    /// A7 — `(caller, called) → CallEdgeTotal` from `SUB_CALLERS`.
    pub call_edges: HashMap<(String, String), CallEdgeTotal>,
    /// A8 — `(fid, line) → source text` from `SRC_LINE` (last write wins).
    pub source_lines: HashMap<(u32, u32), String>,
    /// A9 — `subname → { fid, first_line, last_line }` from `SUB_INFO`
    /// (last write wins).
    pub sub_defs: HashMap<String, SubDef>,
}

impl ProfileModel {
    /// Empty model.
    pub fn new() -> Self {
        Self::default()
    }

    /// File id → name map (same storage as [`Self::files`]).
    pub fn fid_names(&self) -> &HashMap<u32, String> {
        &self.files
    }

    /// Decode a product profile file (v5 or v6) and aggregate.
    ///
    /// Dual dispatch on wire magic/header:
    /// - `NYTPROF6` → always-inflate EVENT (+ FOOTER string-dict when present)
    /// - `NYTProf 5 …` text header → existing v5 decoder
    /// - otherwise fail closed ([`ModelError::UnsupportedProfile`])
    ///
    /// Aggregation rules (A1–A9) are format-agnostic once logical events exist.
    /// Not a wire freeze; not full E5 CLI claim; not E3-mixed multi-kind.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let events = decode_events_from_path(path)?;
        Self::from_events(&events)
    }

    /// Decode product profile bytes (v5 or v6) and aggregate.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let events = decode_events_from_bytes(bytes)?;
        Self::from_events(&events)
    }

    /// Aggregate from an already-decoded ordered event stream.
    ///
    /// Invalid tag argument shapes return [`ModelError::InvalidArgs`].
    pub fn from_events(events: &[Event]) -> Result<Self> {
        let mut model = Self::new();
        for ev in events {
            model.accumulate(ev)?;
        }
        Ok(model)
    }

    /// Aggregate from any iterator of events.
    pub fn from_event_iter<I>(events: I) -> Result<Self>
    where
        I: IntoIterator<Item = Event>,
    {
        let mut model = Self::new();
        for ev in events {
            model.accumulate(&ev)?;
        }
        Ok(model)
    }

    /// Stream one logical event into this model (exact A1–A9 definitions).
    pub fn accumulate(&mut self, ev: &Event) -> Result<()> {
        self.total_events = self.total_events.saturating_add(1);
        match ev.tag.as_str() {
            tags::TIME_LINE => {
                self.time_line_events = self.time_line_events.saturating_add(1);
                let ticks = as_i64(&ev.args, 0, &ev.tag, ev.seq)?;
                let fid = as_u32(&ev.args, 1, &ev.tag, ev.seq)?;
                let line = as_u32(&ev.args, 2, &ev.tag, ev.seq)?;
                let entry = self.line_totals.entry((fid, line)).or_default();
                entry.calls = entry.calls.saturating_add(1);
                entry.ticks = entry.ticks.saturating_add(ticks);
            }
            tags::TIME_BLOCK => {
                // Args: ticks, fid, line, block_line, sub_line
                self.time_block_events = self.time_block_events.saturating_add(1);
                let ticks = as_i64(&ev.args, 0, &ev.tag, ev.seq)?;
                let fid = as_u32(&ev.args, 1, &ev.tag, ev.seq)?;
                let line = as_u32(&ev.args, 2, &ev.tag, ev.seq)?;
                let block_line = as_u32(&ev.args, 3, &ev.tag, ev.seq)?;
                // A4: statement line field contributes to line_totals (same as TIME_LINE).
                let entry = self.line_totals.entry((fid, line)).or_default();
                entry.calls = entry.calls.saturating_add(1);
                entry.ticks = entry.ticks.saturating_add(ticks);
                // A4b: block start line from TIME_BLOCK only.
                let bentry = self.block_line_totals.entry((fid, block_line)).or_default();
                bentry.calls = bentry.calls.saturating_add(1);
                bentry.ticks = bentry.ticks.saturating_add(ticks);
            }
            tags::DISCOUNT => {
                self.discount_events = self.discount_events.saturating_add(1);
            }
            tags::SUB_ENTRY => {
                self.sub_entry_events = self.sub_entry_events.saturating_add(1);
            }
            tags::SUB_RETURN => {
                // Args: depth, incl_time, excl_time, subname
                self.sub_return_events = self.sub_return_events.saturating_add(1);
                let incl = as_f64(&ev.args, 1, &ev.tag, ev.seq)?;
                let excl = as_f64(&ev.args, 2, &ev.tag, ev.seq)?;
                let name = as_str(&ev.args, 3, &ev.tag, ev.seq)?;
                let entry = self.sub_return_totals.entry(name).or_default();
                entry.returns = entry.returns.saturating_add(1);
                entry.incl += incl;
                entry.excl += excl;
            }
            tags::SUB_CALLERS => {
                // Args: fid, line, count, incl, excl, reci, rec_depth, called, caller
                self.sub_callers_events = self.sub_callers_events.saturating_add(1);
                let count = as_u64(&ev.args, 2, &ev.tag, ev.seq)?;
                let incl = as_f64(&ev.args, 3, &ev.tag, ev.seq)?;
                let excl = as_f64(&ev.args, 4, &ev.tag, ev.seq)?;
                let reci = as_f64(&ev.args, 5, &ev.tag, ev.seq)?;
                let rec_depth = as_u32(&ev.args, 6, &ev.tag, ev.seq)?;
                let called = as_str(&ev.args, 7, &ev.tag, ev.seq)?;
                let caller = as_str(&ev.args, 8, &ev.tag, ev.seq)?;
                let entry = self
                    .call_edges
                    .entry((caller, called))
                    .or_default();
                entry.count = entry.count.saturating_add(count);
                entry.incl += incl;
                entry.excl += excl;
                entry.reci += reci;
                if rec_depth > entry.max_rec_depth {
                    entry.max_rec_depth = rec_depth;
                }
                entry.sites = entry.sites.saturating_add(1);
            }
            tags::SRC_LINE => {
                // Args: fid, line, text — last write wins on duplicate keys.
                self.src_line_events = self.src_line_events.saturating_add(1);
                let fid = as_u32(&ev.args, 0, &ev.tag, ev.seq)?;
                let line = as_u32(&ev.args, 1, &ev.tag, ev.seq)?;
                let text = as_str(&ev.args, 2, &ev.tag, ev.seq)?;
                self.source_lines.insert((fid, line), text);
            }
            tags::SUB_INFO => {
                // Args (ReadStream): fid, first_line, last_line, name — last write wins.
                self.sub_info_events = self.sub_info_events.saturating_add(1);
                let fid = as_u32(&ev.args, 0, &ev.tag, ev.seq)?;
                let first_line = as_u32(&ev.args, 1, &ev.tag, ev.seq)?;
                let last_line = as_u32(&ev.args, 2, &ev.tag, ev.seq)?;
                let name = as_str(&ev.args, 3, &ev.tag, ev.seq)?;
                self.sub_defs.insert(
                    name,
                    SubDef {
                        fid,
                        first_line,
                        last_line,
                    },
                );
            }
            tags::NEW_FID => {
                // Args: fid, eval_fid, eval_line, flags, size, mtime, name
                self.new_fid_events = self.new_fid_events.saturating_add(1);
                let fid = as_u32(&ev.args, 0, &ev.tag, ev.seq)?;
                let name = as_str(&ev.args, 6, &ev.tag, ev.seq)?;
                self.files.insert(fid, name);
            }
            tags::PID_START => {
                self.pid_start_events = self.pid_start_events.saturating_add(1);
            }
            tags::PID_END => {
                self.pid_end_events = self.pid_end_events.saturating_add(1);
            }
            tags::ATTRIBUTE => {
                if let (Some(k), Some(v)) = (
                    ev.args.first().and_then(|v| v.as_str()),
                    ev.args.get(1).and_then(|v| v.as_str()),
                ) {
                    self.attributes.insert(k.to_owned(), v.to_owned());
                }
            }
            tags::OPTION => {
                if let (Some(k), Some(v)) = (
                    ev.args.first().and_then(|v| v.as_str()),
                    ev.args.get(1).and_then(|v| v.as_str()),
                ) {
                    self.options.insert(k.to_owned(), v.to_owned());
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Whether the decoded stream is complete enough for default verify/report success.
    ///
    /// See [`Self::stream_incompleteness_reasons`] and
    /// `docs/contracts/COMPAT-010_INCOMPLETE_STREAM.md`.
    pub fn is_stream_complete(&self) -> bool {
        self.stream_incompleteness_reasons().is_empty()
    }

    /// Human-readable reasons the stream is incomplete for verify/report (empty if complete).
    ///
    /// Completeness rules (provisional INCOMPLETE-STREAM):
    /// 1. **PID balance:** if any `PID_START` was seen, require
    ///    `pid_end_events >= pid_start_events` (missing process end is incomplete).
    /// 2. **Statement timing:** require `time_line_events + time_block_events > 0`
    ///    (header-only / attributes-only streams are incomplete for normal profiles).
    ///
    /// Model load / dump may still succeed on incomplete streams; verify and report
    /// fail closed by default (opt-in salvage via `NYTPROF_ALLOW_INCOMPLETE=1`).
    pub fn stream_incompleteness_reasons(&self) -> Vec<&'static str> {
        let mut reasons = Vec::new();
        if self.pid_start_events > 0 && self.pid_end_events < self.pid_start_events {
            reasons.push("missing PID_END after PID_START");
        }
        if self
            .time_line_events
            .saturating_add(self.time_block_events)
            == 0
        {
            reasons.push("no statement timing events (TIME_LINE/TIME_BLOCK)");
        }
        reasons
    }

    /// Line total for `(fid, line)`, if any (A4: `TIME_LINE` + `TIME_BLOCK` statement line).
    pub fn line_total(&self, fid: u32, line: u32) -> Option<LineTotal> {
        self.line_totals.get(&(fid, line)).copied()
    }

    /// Block-line total for `(fid, block_line)`, if any (A4b: `TIME_BLOCK` only).
    pub fn block_line_total(&self, fid: u32, block_line: u32) -> Option<LineTotal> {
        self.block_line_totals.get(&(fid, block_line)).copied()
    }

    /// Subroutine return total for `name`, if any.
    pub fn sub_total(&self, name: &str) -> Option<SubTotal> {
        self.sub_return_totals.get(name).copied()
    }

    /// Lookup A5 totals for a subroutine name (alias of [`Self::sub_total`]).
    pub fn sub_returns(&self, name: &str) -> Option<&SubTotal> {
        self.sub_return_totals.get(name)
    }

    /// Call-edge total for `(caller, called)`, if any (A7).
    pub fn call_edge(&self, caller: &str, called: &str) -> Option<&CallEdgeTotal> {
        self.call_edges
            .get(&(caller.to_owned(), called.to_owned()))
    }

    /// Oracle / JSON key form for a call edge: `"caller -> called"`.
    pub fn call_edge_key(caller: &str, called: &str) -> String {
        format!("{caller} -> {called}")
    }

    /// Source text for `(fid, line)` from `SRC_LINE`, if recorded (A8).
    pub fn source_line(&self, fid: u32, line: u32) -> Option<&str> {
        self.source_lines.get(&(fid, line)).map(String::as_str)
    }

    /// Whether any source text was recorded for `(fid, line)`.
    pub fn has_source(&self, fid: u32, line: u32) -> bool {
        self.source_lines.contains_key(&(fid, line))
    }

    /// A8 stream count alias (`source_line_count` in oracle JSON).
    pub fn source_line_count(&self) -> u64 {
        self.src_line_events
    }

    /// Subroutine definition range for `name` from `SUB_INFO`, if any (A9).
    pub fn sub_def(&self, name: &str) -> Option<&SubDef> {
        self.sub_defs.get(name)
    }

    /// File path recorded for `fid`, if any.
    pub fn file_name(&self, fid: u32) -> Option<&str> {
        self.files.get(&fid).map(String::as_str)
    }

    /// Basename of the path recorded for `fid`, if known.
    pub fn fid_basename(&self, fid: u32) -> Option<&str> {
        self.file_name(fid).map(|p| {
            p.rsplit(['/', '\\'])
                .next()
                .filter(|s| !s.is_empty())
                .unwrap_or(p)
        })
    }

    /// Workload-focused sub names (A6): ending in `::leaf` / `::mid`, or
    /// matching / containing `main::leaf` / `main::mid`.
    pub fn workload_sub_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .sub_return_totals
            .keys()
            .filter(|n| is_workload_sub(n))
            .cloned()
            .collect();
        names.sort();
        names
    }

    /// Subset of A5 for workload names (A6).
    pub fn workload_subs(&self) -> HashMap<String, SubTotal> {
        self.sub_return_totals
            .iter()
            .filter(|(n, _)| is_workload_sub(n))
            .map(|(n, t)| (n.clone(), *t))
            .collect()
    }

    /// Compact debug summary for CLI / tests.
    pub fn debug_summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!(
            "events: total={} TIME_LINE={} TIME_BLOCK={} DISCOUNT={} SUB_ENTRY={} SUB_RETURN={} SUB_CALLERS={} SRC_LINE={} SUB_INFO={}",
            self.total_events,
            self.time_line_events,
            self.time_block_events,
            self.discount_events,
            self.sub_entry_events,
            self.sub_return_events,
            self.sub_callers_events,
            self.src_line_events,
            self.sub_info_events
        ));
        lines.push(format!(
            "maps: files={} line_totals={} block_line_totals={} sub_return_totals={} call_edges={} source_lines={} sub_defs={}",
            self.files.len(),
            self.line_totals.len(),
            self.block_line_totals.len(),
            self.sub_return_totals.len(),
            self.call_edges.len(),
            self.source_lines.len(),
            self.sub_defs.len()
        ));
        for name in self.workload_sub_names() {
            if let Some(t) = self.sub_total(&name) {
                lines.push(format!(
                    "  {name}: returns={} incl={} excl={}",
                    t.returns, t.incl, t.excl
                ));
            }
            if let Some(d) = self.sub_def(&name) {
                lines.push(format!(
                    "  {name} def: fid={} first={} last={}",
                    d.fid, d.first_line, d.last_line
                ));
            }
        }
        // Workload edges of interest when present.
        for (caller, called) in [
            ("main::mid", "main::leaf"),
            ("main::RUNTIME", "main::mid"),
        ] {
            if let Some(e) = self.call_edge(caller, called) {
                lines.push(format!(
                    "  edge {caller} -> {called}: count={} sites={}",
                    e.count, e.sites
                ));
            }
        }
        lines.join("\n")
    }
}

/// A6 name filter used by [`ProfileModel::workload_sub_names`].
pub fn is_workload_sub(name: &str) -> bool {
    name.ends_with("::leaf")
        || name.ends_with("::mid")
        || name == "main::leaf"
        || name == "main::mid"
        || name.contains("main::leaf")
        || name.contains("main::mid")
}

/// Compare two floating sums within absolute or relative epsilon `1e-9`.
pub fn f64_close(a: f64, b: f64) -> bool {
    if a == b {
        return true;
    }
    let diff = (a - b).abs();
    if diff <= 1e-9 {
        return true;
    }
    let scale = a.abs().max(b.abs()).max(1.0);
    diff / scale <= 1e-9
}

fn arg_err(tag: &str, seq: u64, detail: impl Into<String>) -> ModelError {
    ModelError::InvalidArgs {
        tag: tag.to_string(),
        seq,
        detail: detail.into(),
    }
}

fn as_u64(args: &[Value], idx: usize, tag: &str, seq: u64) -> Result<u64> {
    let v = args
        .get(idx)
        .ok_or_else(|| arg_err(tag, seq, format!("missing arg {idx}")))?;
    match v {
        Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                Ok(u)
            } else if let Some(i) = n.as_i64() {
                if i >= 0 {
                    Ok(i as u64)
                } else {
                    Err(arg_err(tag, seq, format!("arg {idx} negative: {i}")))
                }
            } else {
                Err(arg_err(tag, seq, format!("arg {idx} not integer: {v}")))
            }
        }
        _ => Err(arg_err(tag, seq, format!("arg {idx} not number: {v}"))),
    }
}

fn as_u32(args: &[Value], idx: usize, tag: &str, seq: u64) -> Result<u32> {
    let u = as_u64(args, idx, tag, seq)?;
    u.try_into()
        .map_err(|_| arg_err(tag, seq, format!("arg {idx} out of u32 range: {u}")))
}

fn as_i64(args: &[Value], idx: usize, tag: &str, seq: u64) -> Result<i64> {
    let v = args
        .get(idx)
        .ok_or_else(|| arg_err(tag, seq, format!("missing arg {idx}")))?;
    match v {
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i)
            } else if let Some(u) = n.as_u64() {
                i64::try_from(u)
                    .map_err(|_| arg_err(tag, seq, format!("arg {idx} out of i64 range: {u}")))
            } else {
                Err(arg_err(tag, seq, format!("arg {idx} not integer: {v}")))
            }
        }
        _ => Err(arg_err(tag, seq, format!("arg {idx} not number: {v}"))),
    }
}

fn as_f64(args: &[Value], idx: usize, tag: &str, seq: u64) -> Result<f64> {
    let v = args
        .get(idx)
        .ok_or_else(|| arg_err(tag, seq, format!("missing arg {idx}")))?;
    match v {
        Value::Number(n) => n
            .as_f64()
            .ok_or_else(|| arg_err(tag, seq, format!("arg {idx} not f64: {v}"))),
        _ => Err(arg_err(tag, seq, format!("arg {idx} not number: {v}"))),
    }
}

fn as_str(args: &[Value], idx: usize, tag: &str, seq: u64) -> Result<String> {
    let v = args
        .get(idx)
        .ok_or_else(|| arg_err(tag, seq, format!("missing arg {idx}")))?;
    match v {
        Value::String(s) => Ok(s.clone()),
        _ => Err(arg_err(tag, seq, format!("arg {idx} not string: {v}"))),
    }
}

#[cfg(test)]
mod model_tests;
#[cfg(test)]
mod v6_model_tests;
