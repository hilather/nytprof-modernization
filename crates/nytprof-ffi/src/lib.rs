//! Stable C ABI for NYTProf profile open / query / close (RUST-010 MVP).
//!
//! Product path toward full R1 (**PR-A05**, **OQ-2**): coarse-grained handles over
//! [`nytprof_model::ProfileModel`]. Panic-safe: no Rust panic crosses the C ABI.
//!
//! Schema: [`docs/schemas/ffi-cdylib-mvp-v0.md`](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/ffi-cdylib-mvp-v0.md)
//! C header: `crates/nytprof-ffi/include/nytprof_ffi.h`
//!
//! ## Residual honesty
//!
//! This MVP is **not** full RUST-010:
//! - no batch structures / streaming event callbacks
//! - no ASan/Miri harness package
//! - no BUILD-007 automated header generation / ABI freeze tooling
//! - no production install path for the shared library (CLI remains primary)
//! - no XS Data / ReadStream (PERL-004/005 = PR-A06)
//!
//! Dual-path / legacy installs must keep working **without** loading this dylib.

#![deny(unsafe_op_in_unsafe_fn)]

use std::cell::RefCell;
use std::ffi::{c_char, c_int, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

use nytprof_model::ProfileModel;

/// ABI major version advertised by this library (negotiate with
/// [`nytprof_ffi_abi_version`] / [`nytprof_ffi_abi_compatible`]).
pub const NYTPROF_FFI_ABI_VERSION: u32 = 1;

/// Open flag: allow incomplete streams (default open fails closed on incomplete).
pub const NYTPROF_OPEN_ALLOW_INCOMPLETE: u32 = 1;

// ---------------------------------------------------------------------------
// Status codes (keep in sync with include/nytprof_ffi.h)
// ---------------------------------------------------------------------------

pub const NYTPROF_OK: c_int = 0;
pub const NYTPROF_ERR_NULL: c_int = 1;
pub const NYTPROF_ERR_INVALID_UTF8: c_int = 2;
pub const NYTPROF_ERR_IO_DECODE: c_int = 3;
pub const NYTPROF_ERR_INCOMPLETE: c_int = 4;
pub const NYTPROF_ERR_NOT_FOUND: c_int = 5;
pub const NYTPROF_ERR_PANIC: c_int = 6;
pub const NYTPROF_ERR_ABI: c_int = 7;
pub const NYTPROF_ERR_INVALID_HANDLE: c_int = 8;

// ---------------------------------------------------------------------------
// Opaque profile handle
// ---------------------------------------------------------------------------

/// Opaque owned profile handle (C: `nytprof_profile_t`).
pub struct ProfileHandle {
    model: ProfileModel,
}

// ---------------------------------------------------------------------------
// Thread-local last error
// ---------------------------------------------------------------------------

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

fn set_error(msg: impl Into<String>) {
    let s = msg.into();
    let c = CString::new(s.replace('\0', "")).unwrap_or_else(|_| {
        CString::new("error message contained interior NUL").expect("static")
    });
    LAST_ERROR.with(|slot| {
        *slot.borrow_mut() = Some(c);
    });
}

fn clear_error() {
    LAST_ERROR.with(|slot| {
        *slot.borrow_mut() = None;
    });
}

fn status_ok() -> c_int {
    clear_error();
    NYTPROF_OK
}

fn status_err(code: c_int, msg: impl Into<String>) -> c_int {
    set_error(msg);
    code
}

/// Catch panics on every extern "C" boundary. Never unwind into C.
fn trap<F>(f: F) -> c_int
where
    F: FnOnce() -> c_int,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(code) => code,
        Err(_) => {
            set_error("internal panic contained at FFI boundary");
            NYTPROF_ERR_PANIC
        }
    }
}

fn trap_ptr<F>(f: F) -> *mut ProfileHandle
where
    F: FnOnce() -> *mut ProfileHandle,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(p) => p,
        Err(_) => {
            set_error("internal panic contained at FFI boundary");
            ptr::null_mut()
        }
    }
}

fn cstr_to_path(path: *const c_char) -> Result<String, c_int> {
    if path.is_null() {
        return Err(status_err(NYTPROF_ERR_NULL, "path is null"));
    }
    // SAFETY: caller guarantees path is a valid C string or null (checked).
    let s = unsafe { CStr::from_ptr(path) };
    match s.to_str() {
        Ok(utf8) => Ok(utf8.to_owned()),
        Err(_) => Err(status_err(
            NYTPROF_ERR_INVALID_UTF8,
            "path is not valid UTF-8",
        )),
    }
}

fn cstr_to_str<'a>(p: *const c_char, label: &str) -> Result<&'a str, c_int> {
    if p.is_null() {
        return Err(status_err(
            NYTPROF_ERR_NULL,
            format!("{label} is null"),
        ));
    }
    // SAFETY: caller guarantees p is a valid C string or null (checked).
    let s = unsafe { CStr::from_ptr(p) };
    match s.to_str() {
        Ok(utf8) => Ok(utf8),
        Err(_) => Err(status_err(
            NYTPROF_ERR_INVALID_UTF8,
            format!("{label} is not valid UTF-8"),
        )),
    }
}

fn handle_ref<'a>(profile: *const ProfileHandle) -> Result<&'a ProfileHandle, c_int> {
    if profile.is_null() {
        return Err(status_err(NYTPROF_ERR_NULL, "profile handle is null"));
    }
    // SAFETY: caller must pass a live handle from open, or null (checked).
    Ok(unsafe { &*profile })
}

// ---------------------------------------------------------------------------
// Public C ABI
// ---------------------------------------------------------------------------

/// Return the ABI version this library implements.
#[no_mangle]
pub extern "C" fn nytprof_ffi_abi_version() -> u32 {
    NYTPROF_FFI_ABI_VERSION
}

/// Return 1 if `want` is compatible with this library, else 0.
///
/// MVP policy: only exact major match (`want == 1`). Future majors may accept
/// a range; mismatch must fail cleanly (no partial open).
#[no_mangle]
pub extern "C" fn nytprof_ffi_abi_compatible(want: u32) -> c_int {
    if want == NYTPROF_FFI_ABI_VERSION {
        1
    } else {
        0
    }
}

/// Last error message for this thread (NUL-terminated UTF-8).
///
/// Valid until the next FFI call on this thread that sets/clears the error.
/// Returns empty string (not null) when no error is set.
#[no_mangle]
pub extern "C" fn nytprof_last_error() -> *const c_char {
    // Never panic; return a static empty string on failure.
    const EMPTY: &[u8] = b"\0";
    LAST_ERROR.with(|slot| {
        if let Some(ref c) = *slot.borrow() {
            c.as_ptr()
        } else {
            EMPTY.as_ptr() as *const c_char
        }
    })
}

/// Open a v5 profile path into an owned handle.
///
/// * `path` — UTF-8 filesystem path to a v5 `nytprof.out` (or equivalent).
/// * `flags` — `0` = require complete stream (COMPAT-010 fail-closed);
///   [`NYTPROF_OPEN_ALLOW_INCOMPLETE`] permits incomplete models.
/// * `out` — on success, set to a non-null handle that must be closed with
///   [`nytprof_profile_close`]. On failure, set to null when non-null `out`.
///
/// Returns [`NYTPROF_OK`] or an error code. Never panics into C.
#[no_mangle]
pub extern "C" fn nytprof_profile_open(
    path: *const c_char,
    flags: u32,
    out: *mut *mut ProfileHandle,
) -> c_int {
    trap(|| {
        if !out.is_null() {
            // SAFETY: out is non-null; write null on entry so failure is clean.
            unsafe {
                *out = ptr::null_mut();
            }
        } else {
            return status_err(NYTPROF_ERR_NULL, "out pointer is null");
        }

        let path_str = match cstr_to_path(path) {
            Ok(s) => s,
            Err(code) => return code,
        };

        let model = match ProfileModel::from_path(&path_str) {
            Ok(m) => m,
            Err(e) => {
                return status_err(
                    NYTPROF_ERR_IO_DECODE,
                    format!("open/decode failed for {path_str}: {e}"),
                );
            }
        };

        let allow_incomplete = (flags & NYTPROF_OPEN_ALLOW_INCOMPLETE) != 0;
        if !allow_incomplete && !model.is_stream_complete() {
            let reasons = model.stream_incompleteness_reasons().join("; ");
            return status_err(
                NYTPROF_ERR_INCOMPLETE,
                format!("incomplete profile stream (COMPAT-010): {reasons}"),
            );
        }

        let handle = Box::new(ProfileHandle { model });
        // SAFETY: out non-null (checked).
        unsafe {
            *out = Box::into_raw(handle);
        }
        status_ok()
    })
}

/// Free a profile handle. Null is a no-op.
#[no_mangle]
pub extern "C" fn nytprof_profile_close(profile: *mut ProfileHandle) {
    let _ = trap_ptr(|| {
        if profile.is_null() {
            return ptr::null_mut();
        }
        // SAFETY: profile is either null (checked) or a live Box from open.
        drop(unsafe { Box::from_raw(profile) });
        clear_error();
        ptr::null_mut()
    });
}

/// Coarse aggregate counters for a loaded profile (single call, no per-event FFI).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NytprofProfileStats {
    pub total_events: u64,
    pub discount_events: u64,
    pub sub_entry_events: u64,
    pub sub_return_events: u64,
    pub time_line_events: u64,
    pub time_block_events: u64,
    pub pid_start_events: u64,
    pub pid_end_events: u64,
    pub new_fid_events: u64,
    pub sub_callers_events: u64,
    pub src_line_events: u64,
    pub sub_info_events: u64,
    /// 1 if stream complete under COMPAT-010 rules, else 0.
    pub is_stream_complete: c_int,
}

/// Fill `out` with aggregate counters from the model. No per-event FFI.
#[no_mangle]
pub extern "C" fn nytprof_profile_stats(
    profile: *const ProfileHandle,
    out: *mut NytprofProfileStats,
) -> c_int {
    trap(|| {
        if out.is_null() {
            return status_err(NYTPROF_ERR_NULL, "stats out pointer is null");
        }
        let h = match handle_ref(profile) {
            Ok(h) => h,
            Err(code) => return code,
        };
        let m = &h.model;
        let stats = NytprofProfileStats {
            total_events: m.total_events,
            discount_events: m.discount_events,
            sub_entry_events: m.sub_entry_events,
            sub_return_events: m.sub_return_events,
            time_line_events: m.time_line_events,
            time_block_events: m.time_block_events,
            pid_start_events: m.pid_start_events,
            pid_end_events: m.pid_end_events,
            new_fid_events: m.new_fid_events,
            sub_callers_events: m.sub_callers_events,
            src_line_events: m.src_line_events,
            sub_info_events: m.sub_info_events,
            is_stream_complete: if m.is_stream_complete() { 1 } else { 0 },
        };
        // SAFETY: out non-null (checked).
        unsafe {
            *out = stats;
        }
        status_ok()
    })
}

/// Look up `SUB_RETURN` return count for `subname` (e.g. `"main::leaf"`).
///
/// On success writes the count (0 when the sub has no returns recorded) and
/// returns [`NYTPROF_OK`]. Missing sub with zero returns is still OK with 0 —
/// use call-edge or stats when distinguishing absence matters.
#[no_mangle]
pub extern "C" fn nytprof_profile_sub_returns(
    profile: *const ProfileHandle,
    subname: *const c_char,
    returns_out: *mut u64,
) -> c_int {
    trap(|| {
        if returns_out.is_null() {
            return status_err(NYTPROF_ERR_NULL, "returns_out is null");
        }
        let h = match handle_ref(profile) {
            Ok(h) => h,
            Err(code) => return code,
        };
        let name = match cstr_to_str(subname, "subname") {
            Ok(s) => s,
            Err(code) => return code,
        };
        let n = h.model.sub_total(name).map(|t| t.returns).unwrap_or(0);
        // SAFETY: returns_out non-null (checked).
        unsafe {
            *returns_out = n;
        }
        status_ok()
    })
}

/// Look up call-edge `count` for `(caller, called)` (A7 / `SUB_CALLERS`).
///
/// Writes 0 and returns OK when the edge is absent.
#[no_mangle]
pub extern "C" fn nytprof_profile_call_edge_count(
    profile: *const ProfileHandle,
    caller: *const c_char,
    called: *const c_char,
    count_out: *mut u64,
) -> c_int {
    trap(|| {
        if count_out.is_null() {
            return status_err(NYTPROF_ERR_NULL, "count_out is null");
        }
        let h = match handle_ref(profile) {
            Ok(h) => h,
            Err(code) => return code,
        };
        let caller_s = match cstr_to_str(caller, "caller") {
            Ok(s) => s,
            Err(code) => return code,
        };
        let called_s = match cstr_to_str(called, "called") {
            Ok(s) => s,
            Err(code) => return code,
        };
        let n = h
            .model
            .call_edge(caller_s, called_s)
            .map(|e| e.count)
            .unwrap_or(0);
        // SAFETY: count_out non-null (checked).
        unsafe {
            *count_out = n;
        }
        status_ok()
    })
}

/// Look up A4 line-call count for `(fid, line)`.
///
/// Writes 0 and returns OK when the location is absent.
#[no_mangle]
pub extern "C" fn nytprof_profile_line_calls(
    profile: *const ProfileHandle,
    fid: u32,
    line: u32,
    calls_out: *mut u64,
) -> c_int {
    trap(|| {
        if calls_out.is_null() {
            return status_err(NYTPROF_ERR_NULL, "calls_out is null");
        }
        let h = match handle_ref(profile) {
            Ok(h) => h,
            Err(code) => return code,
        };
        let n = h.model.line_total(fid, line).map(|t| t.calls).unwrap_or(0);
        // SAFETY: calls_out non-null (checked).
        unsafe {
            *calls_out = n;
        }
        status_ok()
    })
}

/// Look up A4b block-line call count for `(fid, block_line)`.
#[no_mangle]
pub extern "C" fn nytprof_profile_block_line_calls(
    profile: *const ProfileHandle,
    fid: u32,
    block_line: u32,
    calls_out: *mut u64,
) -> c_int {
    trap(|| {
        if calls_out.is_null() {
            return status_err(NYTPROF_ERR_NULL, "calls_out is null");
        }
        let h = match handle_ref(profile) {
            Ok(h) => h,
            Err(code) => return code,
        };
        let n = h
            .model
            .block_line_total(fid, block_line)
            .map(|t| t.calls)
            .unwrap_or(0);
        // SAFETY: calls_out non-null (checked).
        unsafe {
            *calls_out = n;
        }
        status_ok()
    })
}

// ---------------------------------------------------------------------------
// Unit tests (same process; real entry points)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::path::PathBuf;

    fn fixture(rel: &str) -> PathBuf {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../..");
        p.push(rel);
        p
    }

    fn open_path(path: &str, flags: u32) -> *mut ProfileHandle {
        let c_path = CString::new(path).unwrap();
        let mut out: *mut ProfileHandle = ptr::null_mut();
        let st = nytprof_profile_open(c_path.as_ptr(), flags, &mut out);
        assert_eq!(st, NYTPROF_OK, "open: {}", unsafe {
            CStr::from_ptr(nytprof_last_error()).to_string_lossy()
        });
        assert!(!out.is_null());
        out
    }

    #[test]
    fn abi_version_and_compat() {
        assert_eq!(nytprof_ffi_abi_version(), 1);
        assert_eq!(nytprof_ffi_abi_compatible(1), 1);
        assert_eq!(nytprof_ffi_abi_compatible(0), 0);
        assert_eq!(nytprof_ffi_abi_compatible(2), 0);
    }

    #[test]
    fn open_null_path_fails() {
        let mut out: *mut ProfileHandle = ptr::null_mut();
        let st = nytprof_profile_open(ptr::null(), 0, &mut out);
        assert_eq!(st, NYTPROF_ERR_NULL);
        assert!(out.is_null());
        let msg = unsafe { CStr::from_ptr(nytprof_last_error()) };
        assert!(!msg.to_bytes().is_empty());
    }

    #[test]
    fn open_null_out_fails() {
        let c_path = CString::new("/no/such").unwrap();
        let st = nytprof_profile_open(c_path.as_ptr(), 0, ptr::null_mut());
        assert_eq!(st, NYTPROF_ERR_NULL);
    }

    #[test]
    fn open_missing_file_fails() {
        let c_path = CString::new("/no/such/nytprof.out").unwrap();
        let mut out: *mut ProfileHandle = ptr::null_mut();
        let st = nytprof_profile_open(c_path.as_ptr(), 0, &mut out);
        assert_eq!(st, NYTPROF_ERR_IO_DECODE);
        assert!(out.is_null());
    }

    #[test]
    fn close_null_is_noop() {
        nytprof_profile_close(ptr::null_mut());
    }

    #[test]
    fn default_calls1_open_query_close() {
        let path = fixture("fixtures/v5/default-calls1/nytprof.out");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let h = open_path(path.to_str().unwrap(), 0);

        let mut stats = NytprofProfileStats::default();
        assert_eq!(
            nytprof_profile_stats(h, &mut stats),
            NYTPROF_OK
        );
        assert_eq!(stats.is_stream_complete, 1);
        assert_eq!(stats.discount_events, 818);
        assert_eq!(stats.sub_entry_events, 0);
        // Decoded binary tags only (JSON total_events is +1 for synthetic _END).
        assert_eq!(stats.total_events, 2473);
        assert_eq!(stats.sub_return_events, 27);
        assert_eq!(stats.new_fid_events, 3);
        assert_eq!(stats.sub_callers_events, 13);
        assert_eq!(stats.src_line_events, 632);
        assert_eq!(stats.sub_info_events, 31);
        assert!(stats.time_line_events >= 1);
        assert_eq!(stats.time_block_events, 0);
        assert!(stats.pid_start_events >= 1);
        assert!(stats.pid_end_events >= 1);

        let leaf = CString::new("main::leaf").unwrap();
        let mid = CString::new("main::mid").unwrap();
        let mut leaf_ret = 0u64;
        let mut mid_ret = 0u64;
        assert_eq!(
            nytprof_profile_sub_returns(h, leaf.as_ptr(), &mut leaf_ret),
            NYTPROF_OK
        );
        assert_eq!(
            nytprof_profile_sub_returns(h, mid.as_ptr(), &mut mid_ret),
            NYTPROF_OK
        );
        assert_eq!(leaf_ret, 15);
        assert_eq!(mid_ret, 3);

        let mut edge = 0u64;
        assert_eq!(
            nytprof_profile_call_edge_count(h, mid.as_ptr(), leaf.as_ptr(), &mut edge),
            NYTPROF_OK
        );
        assert_eq!(edge, 15);

        nytprof_profile_close(h);
    }

    #[test]
    fn calls2_sub_entry_multiplicity() {
        let path = fixture("fixtures/v5/calls2-default/nytprof.out");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let h = open_path(path.to_str().unwrap(), 0);
        let mut stats = NytprofProfileStats::default();
        assert_eq!(nytprof_profile_stats(h, &mut stats), NYTPROF_OK);
        assert_eq!(stats.sub_entry_events, 27);
        nytprof_profile_close(h);
    }

    #[test]
    fn blocks_calls1_line_and_block_counts() {
        let path = fixture("fixtures/v5/blocks-calls1/nytprof.out");
        assert!(path.is_file(), "missing fixture {}", path.display());
        let h = open_path(path.to_str().unwrap(), 0);

        let mut stats = NytprofProfileStats::default();
        assert_eq!(nytprof_profile_stats(h, &mut stats), NYTPROF_OK);
        assert_eq!(stats.time_block_events, 916);

        let mut line_calls = 0u64;
        assert_eq!(
            nytprof_profile_line_calls(h, 1, 5, &mut line_calls),
            NYTPROF_OK
        );
        assert_eq!(line_calls, 780);

        let mut block_calls = 0u64;
        assert_eq!(
            nytprof_profile_block_line_calls(h, 1, 4, &mut block_calls),
            NYTPROF_OK
        );
        assert_eq!(block_calls, 810);

        nytprof_profile_close(h);
    }

    #[test]
    fn incomplete_prefix_fails_closed() {
        // Truncated prefix of a real profile: decode may succeed as partial
        // model or fail; either way default open must not return a complete OK.
        let full = fixture("fixtures/v5/default-calls1/nytprof.out");
        let bytes = std::fs::read(&full).expect("read fixture");
        assert!(bytes.len() > 500);
        let dir = std::env::temp_dir();
        let tmp = dir.join(format!(
            "nytprof-ffi-incomplete-{}-{}.out",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&tmp, &bytes[..500]).expect("write prefix");

        let c_path = CString::new(tmp.to_str().unwrap()).unwrap();
        let mut out: *mut ProfileHandle = ptr::null_mut();
        let st = nytprof_profile_open(c_path.as_ptr(), 0, &mut out);
        // Decode error or incomplete — never OK with a live handle under flags=0.
        assert_ne!(st, NYTPROF_OK);
        assert!(out.is_null());
        assert!(st == NYTPROF_ERR_IO_DECODE || st == NYTPROF_ERR_INCOMPLETE);

        // Allow-incomplete may still fail on hard decode errors; if it opens,
        // close cleanly.
        let st2 = nytprof_profile_open(
            c_path.as_ptr(),
            NYTPROF_OPEN_ALLOW_INCOMPLETE,
            &mut out,
        );
        if st2 == NYTPROF_OK {
            assert!(!out.is_null());
            nytprof_profile_close(out);
        } else {
            assert!(out.is_null());
        }

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn query_null_handle_fails() {
        let mut n = 0u64;
        let leaf = CString::new("main::leaf").unwrap();
        assert_eq!(
            nytprof_profile_sub_returns(ptr::null(), leaf.as_ptr(), &mut n),
            NYTPROF_ERR_NULL
        );
        assert_eq!(
            nytprof_profile_stats(ptr::null(), ptr::null_mut()),
            NYTPROF_ERR_NULL
        );
    }
}
