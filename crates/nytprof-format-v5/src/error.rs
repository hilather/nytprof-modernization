use std::io;

/// Decode / I/O error for the v5 format reader.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("profile format error: {0}")]
    Format(String),

    #[error("unsupported tag 0x{tag:02x} ('{ch}') at offset {offset}")]
    UnsupportedTag { tag: u8, ch: char, offset: u64 },

    #[error("unexpected end of data while reading {what} at offset {offset}")]
    UnexpectedEof { what: &'static str, offset: u64 },

    #[error("zlib inflate error: {0}")]
    Zlib(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn format(msg: impl Into<String>) -> Self {
        Error::Format(msg.into())
    }
}
