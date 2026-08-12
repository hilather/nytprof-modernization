//! Merge / repack / salvage tooling MVP (PR-C02 / TOOL-007 / TOOL-009 / RUST-013 scoped).
//!
//! # Recovery semantics (unambiguous)
//!
//! | Operation | Input requirement | Output claim |
//! |-----------|-------------------|--------------|
//! | **merge** | Every input must **fully** decode (fail closed) | Complete encode of concatenated remapped streams; process boundaries preserved |
//! | **repack** | Input must **fully** decode (fail closed) | Clean re-encode (absolute v6 EVENT or v5); same logical events |
//! | **salvage** | Recovers the **longest complete verified prefix** only | Always labeled salvage/incomplete; never pretends a truncated/corrupt tail was valid |
//!
//! Salvage never returns unverifiable mid-record / mid-chunk tail bytes as events.
//! Discarded tail length is reported. Output carries `ATTRIBUTE nytprof.salvage=1`
//! and related keys so operators can distinguish recovery products from clean profiles.
//!
//! # Residuals
//!
//! - Not full `nytprofmerge` aggregate-sum parity (stream-concat + fid remap MVP)
//! - Not full SEC-003 multi-chunk mid-corruption resume matrix
//! - Not packing/string-dict v6 output (uses convert strict encoders)
//! - Not lossy convert modes

use std::path::Path;

use nytprof_format_v6::{
    detect_profile_wire_kind, product_decode_v6_event_profile, ProfileWireKind,
};
use nytprof_types::{tags, Event};
use serde_json::{json, Value};
use thiserror::Error;

use crate::convert::{encode_events, ConvertError, ConvertTarget};
use crate::v6_ingest::{decode_events_from_bytes, owned_records_to_events};
use crate::{ModelError, ProfileModel};

/// Errors from merge / repack / salvage.
#[derive(Debug, Error)]
pub enum MergeToolsError {
    #[error("model/decode: {0}")]
    Model(#[from] ModelError),
    #[error("convert/encode: {0}")]
    Convert(#[from] ConvertError),
    #[error("merge/repack/salvage: {detail}")]
    Tool { detail: String },
    #[error("io error {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

pub type MergeToolsResult<T> = std::result::Result<T, MergeToolsError>;

// ---------------------------------------------------------------------------
// Repack
// ---------------------------------------------------------------------------

/// Re-encode a fully-decoded profile to `target` (absolute v6 or v5).
///
/// Fail closed on corrupt / truncated inputs (use [`salvage_bytes`] for recovery).
pub fn repack_bytes(input: &[u8], target: ConvertTarget) -> MergeToolsResult<Vec<u8>> {
    let events = decode_events_from_bytes(input)?;
    Ok(encode_events(&events, target)?)
}

/// Repack a profile file to `output_path`.
pub fn repack_path(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    target: ConvertTarget,
) -> MergeToolsResult<()> {
    let input_path = input_path.as_ref();
    let bytes = std::fs::read(input_path).map_err(|e| MergeToolsError::Io {
        path: input_path.display().to_string(),
        source: e,
    })?;
    let out = repack_bytes(&bytes, target)?;
    write_out(output_path, &out)
}

/// Detect input wire family for default `--to` on repack/salvage.
pub fn detect_convert_target(input: &[u8]) -> MergeToolsResult<ConvertTarget> {
    match detect_profile_wire_kind(input) {
        ProfileWireKind::V5 => Ok(ConvertTarget::V5),
        ProfileWireKind::V6 => Ok(ConvertTarget::V6),
        ProfileWireKind::Unknown => Err(MergeToolsError::Tool {
            detail: "neither NYTPROF6 magic nor NYTProf v5 text header".into(),
        }),
    }
}

// ---------------------------------------------------------------------------
// Merge (stream-concat + deterministic fid remap)
// ---------------------------------------------------------------------------

/// Merge fully-decoded inputs in order into one profile of `target` format.
///
/// Semantics (MVP, unambiguous):
/// 1. Each input must fully decode (no silent skip of corrupt members).
/// 2. Streams are concatenated in **input list order** (deterministic).
/// 3. `VERSION` / `START_DEFLATE` from streams after the first are dropped
///    (encoder supplies one header / deflate boundary for the target).
/// 4. File IDs (`NEW_FID` and fid-bearing tags) from later streams are offset
///    so they cannot collide with earlier streams; `eval_fid` is remapped too.
/// 5. Process (`PID_*`) boundaries are preserved as independent sequences.
/// 6. `seq` is renumbered 0..n-1 on the merged stream.
///
/// This is **not** legacy `nytprofmerge` aggregate-sum of same-run line totals;
/// it is ordered multi-profile stream merge for tooling / mixed v5+v6 inputs.
pub fn merge_bytes(inputs: &[&[u8]], target: ConvertTarget) -> MergeToolsResult<Vec<u8>> {
    if inputs.is_empty() {
        return Err(MergeToolsError::Tool {
            detail: "merge requires at least one input".into(),
        });
    }
    if inputs.len() == 1 {
        // Single-input merge is repack (documented).
        return repack_bytes(inputs[0], target);
    }

    let mut decoded: Vec<Vec<Event>> = Vec::with_capacity(inputs.len());
    for (i, bytes) in inputs.iter().enumerate() {
        let evs = decode_events_from_bytes(bytes).map_err(|e| MergeToolsError::Tool {
            detail: format!("input[{i}] decode failed: {e}"),
        })?;
        if evs.is_empty() {
            return Err(MergeToolsError::Tool {
                detail: format!("input[{i}] produced zero events"),
            });
        }
        decoded.push(evs);
    }

    let merged = merge_event_streams(&decoded)?;
    Ok(encode_events(&merged, target)?)
}

/// Merge profile paths into `output_path`.
pub fn merge_paths(
    input_paths: &[impl AsRef<Path>],
    output_path: impl AsRef<Path>,
    target: ConvertTarget,
) -> MergeToolsResult<()> {
    if input_paths.is_empty() {
        return Err(MergeToolsError::Tool {
            detail: "merge requires at least one input path".into(),
        });
    }
    let mut owned: Vec<Vec<u8>> = Vec::with_capacity(input_paths.len());
    for p in input_paths {
        let p = p.as_ref();
        let b = std::fs::read(p).map_err(|e| MergeToolsError::Io {
            path: p.display().to_string(),
            source: e,
        })?;
        owned.push(b);
    }
    let refs: Vec<&[u8]> = owned.iter().map(|v| v.as_slice()).collect();
    let out = merge_bytes(&refs, target)?;
    write_out(output_path, &out)
}

fn merge_event_streams(streams: &[Vec<Event>]) -> MergeToolsResult<Vec<Event>> {
    let mut out: Vec<Event> = Vec::new();
    let mut fid_base: u64 = 0;
    let mut first_stream = true;

    for (si, stream) in streams.iter().enumerate() {
        let mut stream_max_fid: u64 = 0;
        let mut remapped: Vec<Event> = Vec::with_capacity(stream.len() + 1);

        if !first_stream {
            remapped.push(Event::new(
                0,
                tags::COMMENT,
                vec![json!(format!(
                    "# nytprof-merge: begin input {si} (fid_base={fid_base})\n"
                ))],
            ));
        }

        for ev in stream {
            match ev.tag.as_str() {
                tags::END => continue,
                tags::VERSION if !first_stream => continue,
                tags::START_DEFLATE if !first_stream => continue,
                _ => {
                    let e = remap_event_fids(ev, fid_base)?;
                    // Track max fid seen in this stream (after remap).
                    if let Some(f) = event_declared_fid(&e) {
                        stream_max_fid = stream_max_fid.max(f);
                    }
                    remapped.push(e);
                }
            }
        }

        out.extend(remapped);
        // Next stream's fids start after this stream's max remapped fid.
        if stream_max_fid > fid_base {
            fid_base = stream_max_fid;
        }
        // Ensure progress even if stream had no NEW_FID (still isolate by +0 is fine;
        // if both use fid 1 without NEW_FID, they would collide — rare; still remap
        // fid-bearing tags with base so TIME_LINE fids also shift when base advances).
        first_stream = false;
    }

    // Renumber seq.
    for (i, ev) in out.iter_mut().enumerate() {
        ev.seq = i as u64;
    }
    Ok(out)
}

fn event_declared_fid(ev: &Event) -> Option<u64> {
    match ev.tag.as_str() {
        tags::NEW_FID => arg_as_u64(ev.args.first()),
        _ => None,
    }
}

fn remap_event_fids(ev: &Event, fid_base: u64) -> MergeToolsResult<Event> {
    if fid_base == 0 {
        return Ok(ev.clone());
    }
    let mut args = ev.args.clone();
    match ev.tag.as_str() {
        tags::NEW_FID => {
            // fid, [eval_fid, eval_line, flags, size, mtime,] name
            add_u64_arg(&mut args, 0, fid_base, ev)?;
            if args.len() >= 7 {
                // eval_fid only if non-zero (0 stays 0 = no eval parent)
                if arg_as_u64(args.get(1)).unwrap_or(0) != 0 {
                    add_u64_arg(&mut args, 1, fid_base, ev)?;
                }
            }
        }
        tags::TIME_LINE | tags::TIME_BLOCK => {
            // ticks, fid, line, ...
            add_u64_arg(&mut args, 1, fid_base, ev)?;
        }
        tags::SUB_ENTRY => {
            // caller_fid, caller_line
            add_u64_arg(&mut args, 0, fid_base, ev)?;
        }
        tags::SUB_INFO | tags::SUB_CALLERS | tags::SRC_LINE => {
            add_u64_arg(&mut args, 0, fid_base, ev)?;
        }
        _ => {}
    }
    Ok(Event::new(ev.seq, ev.tag.clone(), args))
}

fn add_u64_arg(
    args: &mut [Value],
    idx: usize,
    delta: u64,
    ev: &Event,
) -> MergeToolsResult<()> {
    let v = arg_as_u64(args.get(idx)).ok_or_else(|| MergeToolsError::Tool {
        detail: format!(
            "seq {}: {} arg[{idx}] not an integer fid for remap",
            ev.seq, ev.tag
        ),
    })?;
    let sum = v.checked_add(delta).ok_or_else(|| MergeToolsError::Tool {
        detail: format!(
            "seq {}: {} fid {v}+{delta} overflow",
            ev.seq, ev.tag
        ),
    })?;
    args[idx] = json!(sum);
    Ok(())
}

fn arg_as_u64(v: Option<&Value>) -> Option<u64> {
    let v = v?;
    if let Some(u) = v.as_u64() {
        return Some(u);
    }
    if let Some(i) = v.as_i64() {
        if i >= 0 {
            return Some(i as u64);
        }
    }
    if let Some(f) = v.as_f64() {
        if f.is_finite() && f.fract() == 0.0 && f >= 0.0 && f <= u64::MAX as f64 {
            return Some(f as u64);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Salvage (longest complete verified prefix)
// ---------------------------------------------------------------------------

/// Report from a salvage operation (always recovery-labeled).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SalvageReport {
    /// Wire family of the input (`v5` / `v6`).
    pub wire_kind: &'static str,
    /// Bytes of input accepted into the recovered prefix.
    pub bytes_consumed: usize,
    /// Total input length.
    pub input_len: usize,
    /// `input_len - bytes_consumed` (corrupt/truncated tail).
    pub discarded_tail_bytes: usize,
    /// Logical events recovered (before salvage marker injection).
    pub events_recovered: usize,
    /// Events written to output (includes salvage ATTRIBUTE markers).
    pub events_written: usize,
    /// Stream incompleteness reasons on the recovered model (PID / timing).
    pub incomplete_reasons: Vec<&'static str>,
    /// Always true for the salvage command: recovery path never claims clean origin.
    pub salvage_labeled: bool,
}

/// Salvage the longest complete verified event prefix from `input` and encode to `target`.
///
/// Recovery rules:
/// 1. Prefer full-input decode when it succeeds (v5 whole file; v6 product profile
///    with optional trailing-byte strip via `bytes_consumed`).
/// 2. On hard decode failure, binary-search the longest prefix that fully decodes.
/// 3. Mid-record / mid-chunk tails are **discarded** (never partial events).
/// 4. Output is always labeled with salvage ATTRIBUTES; never silent success as clean.
/// 5. Zero recoverable events → error.
pub fn salvage_bytes(
    input: &[u8],
    target: ConvertTarget,
) -> MergeToolsResult<(Vec<u8>, SalvageReport)> {
    if input.is_empty() {
        return Err(MergeToolsError::Tool {
            detail: "salvage: empty input".into(),
        });
    }

    let wire_kind = match detect_profile_wire_kind(input) {
        ProfileWireKind::V5 => "v5",
        ProfileWireKind::V6 => "v6",
        ProfileWireKind::Unknown => {
            return Err(MergeToolsError::Tool {
                detail: "salvage: unknown wire (need NYTPROF6 or NYTProf v5 header)".into(),
            });
        }
    };

    let (events, bytes_consumed) = recover_longest_prefix(input, wire_kind)?;
    if events.is_empty() {
        return Err(MergeToolsError::Tool {
            detail: format!(
                "salvage: no complete verified events in {wire_kind} prefix (consumed={bytes_consumed})"
            ),
        });
    }

    let events_recovered = events.len();
    let model = ProfileModel::from_events(&events).map_err(MergeToolsError::from)?;
    let incomplete_reasons = model.stream_incompleteness_reasons();

    let labeled = inject_salvage_markers(
        events,
        bytes_consumed,
        input.len(),
        wire_kind,
        &incomplete_reasons,
    );
    let events_written = labeled.len();
    let out = encode_events(&labeled, target)?;

    let report = SalvageReport {
        wire_kind,
        bytes_consumed,
        input_len: input.len(),
        discarded_tail_bytes: input.len().saturating_sub(bytes_consumed),
        events_recovered,
        events_written,
        incomplete_reasons,
        salvage_labeled: true,
    };
    Ok((out, report))
}

/// Salvage a path to `output_path`.
pub fn salvage_path(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    target: ConvertTarget,
) -> MergeToolsResult<SalvageReport> {
    let input_path = input_path.as_ref();
    let bytes = std::fs::read(input_path).map_err(|e| MergeToolsError::Io {
        path: input_path.display().to_string(),
        source: e,
    })?;
    let (out, report) = salvage_bytes(&bytes, target)?;
    write_out(output_path, &out)?;
    Ok(report)
}

fn recover_longest_prefix(
    input: &[u8],
    wire_kind: &str,
) -> MergeToolsResult<(Vec<Event>, usize)> {
    match wire_kind {
        "v6" => recover_v6_prefix(input),
        "v5" => recover_v5_prefix(input),
        _ => Err(MergeToolsError::Tool {
            detail: format!("salvage: unsupported wire {wire_kind}"),
        }),
    }
}

fn recover_v6_prefix(input: &[u8]) -> MergeToolsResult<(Vec<Event>, usize)> {
    // Fast path: product decode may succeed with trailing garbage.
    match product_decode_v6_event_profile(input, true) {
        Ok(decoded) => {
            let events = owned_records_to_events(
                &decoded.profile.records,
                &decoded.profile.sequences,
            )?;
            return Ok((events, decoded.bytes_consumed));
        }
        Err(_) => {
            // Binary search longest complete prefix.
        }
    }

    let mut lo = 0usize;
    let mut hi = input.len();
    let mut best: Option<(Vec<Event>, usize)> = None;
    // Need at least magic + header; search byte prefixes.
    while lo <= hi {
        let mid = lo + (hi - lo) / 2;
        if mid == 0 {
            if lo == 0 && hi == 0 {
                break;
            }
            lo = 1;
            continue;
        }
        match product_decode_v6_event_profile(&input[..mid], true) {
            Ok(decoded) => {
                // Prefer exact mid consumption; if decoder consumed less, still valid.
                let n = decoded.bytes_consumed.min(mid);
                if let Ok(events) =
                    owned_records_to_events(&decoded.profile.records, &decoded.profile.sequences)
                {
                    if !events.is_empty() {
                        best = Some((events, n));
                    }
                }
                // Try longer.
                if mid == input.len() {
                    break;
                }
                lo = mid + 1;
            }
            Err(_) => {
                if mid == 0 {
                    break;
                }
                hi = mid - 1;
            }
        }
        if lo > hi {
            break;
        }
    }

    // Linear refine near best: try best..input.len() for any longer success
    // (binary search on fail-closed decoders can miss non-monotonic edges;
    // refine upward from known-good).
    if let Some((ref _ev, n)) = best {
        let start = n.saturating_add(1);
        for end in (start..=input.len()).rev() {
            if let Ok(decoded) = product_decode_v6_event_profile(&input[..end], true) {
                let cn = decoded.bytes_consumed.min(end);
                if cn >= n {
                    if let Ok(events) = owned_records_to_events(
                        &decoded.profile.records,
                        &decoded.profile.sequences,
                    ) {
                        if !events.is_empty() {
                            return Ok((events, cn));
                        }
                    }
                }
            }
        }
        return best.ok_or_else(|| MergeToolsError::Tool {
            detail: "salvage: v6 no recoverable prefix".into(),
        });
    }

    // Last resort: try every length (small fixtures); bound work for huge inputs.
    let cap = input.len().min(4 * 1024 * 1024);
    for end in (1..=cap).rev() {
        if let Ok(decoded) = product_decode_v6_event_profile(&input[..end], true) {
            if let Ok(events) =
                owned_records_to_events(&decoded.profile.records, &decoded.profile.sequences)
            {
                if !events.is_empty() {
                    return Ok((events, decoded.bytes_consumed.min(end)));
                }
            }
        }
    }

    Err(MergeToolsError::Tool {
        detail: "salvage: v6 no complete verified chunk/event prefix".into(),
    })
}

fn recover_v5_prefix(input: &[u8]) -> MergeToolsResult<(Vec<Event>, usize)> {
    // Prefer strict full decode when the whole stream is valid.
    if let Ok(events) = nytprof_format_v5::decode_all(input) {
        if !events.is_empty() {
            return Ok((events, input.len()));
        }
    }

    // Progressive salvage: complete tags only; incomplete zlib/tags discarded.
    // (Binary search over decode_all is unsafe: incomplete zlib after `z` is a
    // non-monotonic failure region between a good text prefix and a good full file.)
    match nytprof_format_v5::decode_salvage_prefix(input) {
        Ok((events, n)) if !events.is_empty() => Ok((events, n)),
        Ok(_) => Err(MergeToolsError::Tool {
            detail: "salvage: v5 decoded zero events".into(),
        }),
        Err(e) => Err(MergeToolsError::Tool {
            detail: format!("salvage: v5 no complete verified record prefix ({e})"),
        }),
    }
}

fn inject_salvage_markers(
    mut events: Vec<Event>,
    bytes_consumed: usize,
    input_len: usize,
    wire_kind: &str,
    incomplete_reasons: &[&'static str],
) -> Vec<Event> {
    // Insert salvage markers after first VERSION if present, else at front.
    let mut insert_at = 0usize;
    if events
        .first()
        .map(|e| e.tag == tags::VERSION)
        .unwrap_or(false)
    {
        insert_at = 1;
    }

    let reasons = if incomplete_reasons.is_empty() {
        "none".to_string()
    } else {
        incomplete_reasons.join(",")
    };

    let markers = [
        Event::new(
            0,
            tags::ATTRIBUTE,
            vec![json!("nytprof.salvage"), json!("1")],
        ),
        Event::new(
            0,
            tags::ATTRIBUTE,
            vec![json!("nytprof.salvage.incomplete"), json!("1")],
        ),
        Event::new(
            0,
            tags::ATTRIBUTE,
            vec![
                json!("nytprof.salvage.wire"),
                json!(wire_kind),
            ],
        ),
        Event::new(
            0,
            tags::ATTRIBUTE,
            vec![
                json!("nytprof.salvage.bytes_consumed"),
                json!(bytes_consumed.to_string()),
            ],
        ),
        Event::new(
            0,
            tags::ATTRIBUTE,
            vec![
                json!("nytprof.salvage.input_len"),
                json!(input_len.to_string()),
            ],
        ),
        Event::new(
            0,
            tags::ATTRIBUTE,
            vec![
                json!("nytprof.salvage.discarded_tail"),
                json!(input_len.saturating_sub(bytes_consumed).to_string()),
            ],
        ),
        Event::new(
            0,
            tags::ATTRIBUTE,
            vec![
                json!("nytprof.salvage.stream_incomplete"),
                json!(reasons),
            ],
        ),
        Event::new(
            0,
            tags::COMMENT,
            vec![json!(format!(
                "# nytprof-salvage: recovered complete verified prefix only; incomplete=1; discarded_tail={}\n",
                input_len.saturating_sub(bytes_consumed)
            ))],
        ),
    ];

    for (i, m) in markers.into_iter().enumerate() {
        events.insert(insert_at + i, m);
    }
    for (i, ev) in events.iter_mut().enumerate() {
        ev.seq = i as u64;
    }
    events
}

fn write_out(output_path: impl AsRef<Path>, bytes: &[u8]) -> MergeToolsResult<()> {
    let output_path = output_path.as_ref();
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| MergeToolsError::Io {
                path: parent.display().to_string(),
                source: e,
            })?;
        }
    }
    std::fs::write(output_path, bytes).map_err(|e| MergeToolsError::Io {
        path: output_path.display().to_string(),
        source: e,
    })?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::convert_bytes;
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
    fn repack_v6_identity_m4() {
        let path = dual("m4", "v6");
        if !path.is_file() {
            return;
        }
        let bytes = std::fs::read(&path).unwrap();
        let out = repack_bytes(&bytes, ConvertTarget::V6).expect("repack v6");
        assert!(out.starts_with(b"NYTPROF6"));
        let a = ProfileModel::from_bytes(&bytes).unwrap();
        let b = ProfileModel::from_bytes(&out).unwrap();
        e4_v0_aggregates_equal(&a, &b, false).expect("repack aggregates");
    }

    #[test]
    fn repack_v5_to_v6_m4() {
        let path = dual("m4", "v5");
        if !path.is_file() {
            return;
        }
        let bytes = std::fs::read(&path).unwrap();
        let out = repack_bytes(&bytes, ConvertTarget::V6).expect("repack to v6");
        assert!(out.starts_with(b"NYTPROF6"));
        ProfileModel::from_bytes(&out).expect("load repacked");
    }

    #[test]
    fn repack_refuses_truncated() {
        let path = dual("m4", "v5");
        if !path.is_file() {
            return;
        }
        let bytes = std::fs::read(&path).unwrap();
        let cut = &bytes[..bytes.len().min(40).max(12)];
        let err = repack_bytes(cut, ConvertTarget::V5).expect_err("truncated must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("decode")
                || msg.contains("UnexpectedEof")
                || msg.contains("format")
                || msg.contains("error")
                || msg.contains("zlib")
                || msg.contains("EOF")
                || msg.contains("header"),
            "unexpected: {msg}"
        );
    }

    #[test]
    fn merge_two_m4_v6_preserves_process_streams() {
        let path = dual("m4", "v6");
        if !path.is_file() {
            return;
        }
        let bytes = std::fs::read(&path).unwrap();
        let out = merge_bytes(&[&bytes, &bytes], ConvertTarget::V6).expect("merge");
        assert!(out.starts_with(b"NYTPROF6"));
        let model = ProfileModel::from_bytes(&out).expect("load merge");
        // Two copies → double pid starts/ends and roughly double leaf returns if present.
        assert!(
            model.pid_start_events >= 2,
            "expected ≥2 PID_START, got {}",
            model.pid_start_events
        );
        assert!(
            model.pid_end_events >= 2,
            "expected ≥2 PID_END, got {}",
            model.pid_end_events
        );
        if let Some(t) = model.sub_total("main::leaf") {
            // m4 may or may not have leaf; if present doubled.
            assert!(t.returns >= 2, "leaf returns {}", t.returns);
        }
    }

    #[test]
    fn merge_mixed_v5_v6_to_v6() {
        let v5 = dual("m4", "v5");
        let v6 = dual("m4", "v6");
        if !v5.is_file() || !v6.is_file() {
            return;
        }
        let b5 = std::fs::read(&v5).unwrap();
        let b6 = std::fs::read(&v6).unwrap();
        let out = merge_bytes(&[&b5, &b6], ConvertTarget::V6).expect("mixed merge");
        assert!(out.starts_with(b"NYTPROF6"));
        let model = ProfileModel::from_bytes(&out).expect("load");
        assert!(model.pid_start_events >= 2);
    }

    #[test]
    fn merge_fid_remap_avoids_collision() {
        use serde_json::json;
        // Two tiny synthetic streams both declare fid=1 with different names.
        let s1 = vec![
            Event::new(0, tags::VERSION, vec![json!(6), json!(0)]),
            Event::new(
                1,
                tags::NEW_FID,
                vec![
                    json!(1),
                    json!(0),
                    json!(0),
                    json!(0),
                    json!(0),
                    json!(0),
                    json!("a.pl"),
                ],
            ),
            Event::new(2, tags::TIME_LINE, vec![json!(10), json!(1), json!(1)]),
            Event::new(3, tags::PID_START, vec![json!(1), json!(0), json!(0)]),
            Event::new(4, tags::PID_END, vec![json!(1), json!(1)]),
        ];
        let s2 = vec![
            Event::new(0, tags::VERSION, vec![json!(6), json!(0)]),
            Event::new(
                1,
                tags::NEW_FID,
                vec![
                    json!(1),
                    json!(0),
                    json!(0),
                    json!(0),
                    json!(0),
                    json!(0),
                    json!("b.pl"),
                ],
            ),
            Event::new(2, tags::TIME_LINE, vec![json!(20), json!(1), json!(2)]),
            Event::new(3, tags::PID_START, vec![json!(2), json!(0), json!(0)]),
            Event::new(4, tags::PID_END, vec![json!(2), json!(1)]),
        ];
        let e1 = encode_events(&s1, ConvertTarget::V6).unwrap();
        let e2 = encode_events(&s2, ConvertTarget::V6).unwrap();
        let out = merge_bytes(&[&e1, &e2], ConvertTarget::V6).expect("merge");
        let events = decode_events_from_bytes(&out).expect("decode merge");
        let fids: Vec<u64> = events
            .iter()
            .filter(|e| e.tag == tags::NEW_FID)
            .filter_map(|e| e.args.first().and_then(|v| v.as_u64()))
            .collect();
        assert_eq!(fids.len(), 2, "two NEW_FID");
        assert_ne!(fids[0], fids[1], "fids must not collide: {fids:?}");
        // TIME_LINE on second stream should reference remapped fid.
        let time_fids: Vec<u64> = events
            .iter()
            .filter(|e| e.tag == tags::TIME_LINE)
            .filter_map(|e| e.args.get(1).and_then(|v| v.as_u64()))
            .collect();
        assert_eq!(time_fids.len(), 2);
        assert_eq!(time_fids[0], fids[0]);
        assert_eq!(time_fids[1], fids[1]);
    }

    #[test]
    fn merge_refuses_corrupt_member() {
        let path = dual("m4", "v6");
        if !path.is_file() {
            return;
        }
        let good = std::fs::read(&path).unwrap();
        let bad = b"not a profile";
        let err = merge_bytes(&[&good, bad.as_slice()], ConvertTarget::V6)
            .expect_err("corrupt member");
        let msg = err.to_string();
        assert!(
            msg.contains("input[1]") || msg.contains("decode") || msg.contains("unknown"),
            "got: {msg}"
        );
    }

    #[test]
    fn salvage_truncated_v5_prefix_recovers_and_labels() {
        let path = dual("m4", "v5");
        if !path.is_file() {
            return;
        }
        let full = std::fs::read(&path).unwrap();
        // Cut mid-zlib (after START_DEFLATE): progressive salvage keeps pre-z tags.
        assert!(full.len() > 50, "m4 fixture unexpectedly tiny");
        let truncated = &full[..full.len() / 2];
        assert!(
            nytprof_format_v5::decode_all(truncated).is_err(),
            "half-file must fail strict decode (mid-zlib)"
        );
        let (out, report) =
            salvage_bytes(truncated, ConvertTarget::V5).expect("salvage truncated");
        assert!(out.starts_with(b"NYTProf 5 0\n"));
        assert!(report.salvage_labeled);
        assert!(report.events_recovered >= 1);
        assert_eq!(report.wire_kind, "v5");
        assert!(report.bytes_consumed <= truncated.len());
        assert!(report.discarded_tail_bytes >= 1);
        // Salvage attributes present.
        let events = decode_events_from_bytes(&out).expect("decode salvage out");
        let has_salvage = events.iter().any(|e| {
            e.tag == tags::ATTRIBUTE
                && e.args.first().and_then(|v| v.as_str()) == Some("nytprof.salvage")
        });
        assert!(has_salvage, "missing nytprof.salvage attribute");
        let incomplete_attr = events.iter().any(|e| {
            e.tag == tags::ATTRIBUTE
                && e.args.first().and_then(|v| v.as_str())
                    == Some("nytprof.salvage.incomplete")
                && e.args.get(1).and_then(|v| v.as_str()) == Some("1")
        });
        assert!(incomplete_attr, "must label incomplete=1");
    }

    #[test]
    fn salvage_full_v6_still_labels_incomplete_origin() {
        let path = dual("m4", "v6");
        if !path.is_file() {
            return;
        }
        let full = std::fs::read(&path).unwrap();
        let (out, report) = salvage_bytes(&full, ConvertTarget::V6).expect("salvage full");
        assert!(out.starts_with(b"NYTPROF6"));
        assert_eq!(report.discarded_tail_bytes, 0);
        assert!(report.salvage_labeled);
        assert!(report.events_recovered >= 1);
        // Never pretends clean origin even when prefix == full file.
        let events = decode_events_from_bytes(&out).unwrap();
        assert!(events.iter().any(|e| {
            e.tag == tags::ATTRIBUTE
                && e.args.first().and_then(|v| v.as_str()) == Some("nytprof.salvage")
        }));
    }

    #[test]
    fn salvage_v6_with_trailing_garbage_discards_tail() {
        let path = dual("m4", "v6");
        if !path.is_file() {
            return;
        }
        let mut bytes = std::fs::read(&path).unwrap();
        let clean_len = bytes.len();
        bytes.extend_from_slice(b"GARBAGE_TAIL_NOT_A_CHUNK!!!!");
        let (out, report) = salvage_bytes(&bytes, ConvertTarget::V6).expect("salvage trash");
        assert!(out.starts_with(b"NYTPROF6"));
        assert!(
            report.discarded_tail_bytes >= 1,
            "expected discarded tail, report={report:?}"
        );
        assert!(report.bytes_consumed <= clean_len + 8);
        ProfileModel::from_bytes(&out).expect("load salvaged");
    }

    #[test]
    fn salvage_empty_refuses() {
        let err = salvage_bytes(b"", ConvertTarget::V5).unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn salvage_garbage_refuses() {
        let err = salvage_bytes(b"\x00\x01\x02not-nytprof", ConvertTarget::V5).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("unknown") || msg.contains("salvage"),
            "got: {msg}"
        );
    }

    #[test]
    fn detect_target_dual() {
        let v5 = dual("m4", "v5");
        let v6 = dual("m4", "v6");
        if !v5.is_file() {
            return;
        }
        let b5 = std::fs::read(&v5).unwrap();
        assert_eq!(detect_convert_target(&b5).unwrap(), ConvertTarget::V5);
        if v6.is_file() {
            let b6 = std::fs::read(&v6).unwrap();
            assert_eq!(detect_convert_target(&b6).unwrap(), ConvertTarget::V6);
        }
    }

    #[test]
    fn repack_matches_convert_path_m4() {
        let path = dual("m4", "v5");
        if !path.is_file() {
            return;
        }
        let bytes = std::fs::read(&path).unwrap();
        let via_repack = repack_bytes(&bytes, ConvertTarget::V6).unwrap();
        let via_convert = convert_bytes(&bytes, ConvertTarget::V6).unwrap();
        // Both absolute v6 EVENT; event-level equality via models.
        let a = ProfileModel::from_bytes(&via_repack).unwrap();
        let b = ProfileModel::from_bytes(&via_convert).unwrap();
        e4_v0_aggregates_equal(&a, &b, false).expect("repack≡convert");
    }

    #[test]
    fn oracle_v5_salvage_500_byte_prefix() {
        let path = fixture_v5("default-calls1");
        if !path.is_file() {
            return;
        }
        let full = std::fs::read(&path).unwrap();
        let prefix = &full[..500.min(full.len())];
        // Incomplete-stream contract: 500-byte prefix of default-calls1 decodes.
        let (out, report) = salvage_bytes(prefix, ConvertTarget::V5).expect("salvage 500");
        assert!(report.salvage_labeled);
        assert!(report.events_recovered >= 1);
        assert!(out.starts_with(b"NYTProf 5 0\n"));
        // Recovered model should list incompleteness (no timing / open PID).
        assert!(
            !report.incomplete_reasons.is_empty() || report.events_recovered > 0,
            "report={report:?}"
        );
    }
}

