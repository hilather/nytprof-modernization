//! Shared event types for the canonical ReadStream dump schema (v0).
//!
//! See `docs/schemas/canonical-event-dump-v0.md`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One logical profile event (one JSONL record, excluding synthetic `_END`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    /// Monotonic sequence number assigned by the dumper (0-based).
    pub seq: u64,
    /// ReadStream tag name (`VERSION`, `TIME_LINE`, …).
    pub tag: String,
    /// Tag-specific arguments in callback order (not necessarily wire order).
    pub args: Vec<Value>,
}

impl Event {
    pub fn new(seq: u64, tag: impl Into<String>, args: Vec<Value>) -> Self {
        Self {
            seq,
            tag: tag.into(),
            args,
        }
    }
}

/// Well-known tag name constants matching ReadStream / loader callbacks.
pub mod tags {
    pub const VERSION: &str = "VERSION";
    pub const COMMENT: &str = "COMMENT";
    pub const ATTRIBUTE: &str = "ATTRIBUTE";
    pub const OPTION: &str = "OPTION";
    pub const START_DEFLATE: &str = "START_DEFLATE";
    pub const PID_START: &str = "PID_START";
    pub const PID_END: &str = "PID_END";
    pub const NEW_FID: &str = "NEW_FID";
    pub const TIME_LINE: &str = "TIME_LINE";
    pub const TIME_BLOCK: &str = "TIME_BLOCK";
    pub const DISCOUNT: &str = "DISCOUNT";
    pub const SUB_ENTRY: &str = "SUB_ENTRY";
    pub const SUB_RETURN: &str = "SUB_RETURN";
    pub const SUB_INFO: &str = "SUB_INFO";
    pub const SUB_CALLERS: &str = "SUB_CALLERS";
    pub const SRC_LINE: &str = "SRC_LINE";
    pub const END: &str = "_END";
}
