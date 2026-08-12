//! Independent streaming **decoder** and strict **encoder** for Devel::NYTProf
//! **v5** profile files.
//!
//! Wire format is derived from oracle 6.15 `FileHandle.xs` / `NYTProf.xs`
//! (`read_u32`, `read_str`, `load_profile_data_from_stream`) and COL-006
//! `nytp_sink_v5` for the encode path.
//!
//! Event argument order matches ReadStream loader callbacks (see
//! `docs/schemas/canonical-event-dump-v0.md`), not always the on-wire field
//! order (notably `SUB_INFO` and `SUB_CALLERS`).
//!
//! Encode is **strict** (no silent truncation); used by convert tooling (PR-C01).

mod error;
mod reader;
mod varint;
mod writer;

pub use error::{Error, Result};
pub use reader::{decode_all, decode_path, EventIter};
pub use varint::{decode_u32, encode_u32, read_i32, read_u32};
pub use writer::{encode_all, encode_all_as_v5};
