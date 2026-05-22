use thiserror::Error;

/// Errors returned by [`crate::emit_event`].
///
/// Error codes (0xEE01–0xEE04) are stable and must never change.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum EventError {
    #[error("Event payload size {actual} bytes exceeds limit of {limit} bytes")]
    PayloadTooLarge { limit: usize, actual: usize },

    #[error("Transaction event count {actual} exceeds limit of {limit}")]
    TooManyEvents { limit: u32, actual: u32 },

    #[error("Total event bytes {actual} exceeds per-transaction limit of {limit}")]
    TotalSizeTooLarge { limit: usize, actual: usize },

    #[error("Failed to encode event: {0}")]
    EncodingFailed(String),
}

impl EventError {
    /// Deterministic numeric error code for each variant.
    /// These codes are stable and documented — never change them.
    pub const fn error_code(&self) -> u32 {
        match self {
            Self::PayloadTooLarge { .. } => 0xEE01,
            Self::TooManyEvents { .. } => 0xEE02,
            Self::TotalSizeTooLarge { .. } => 0xEE03,
            Self::EncodingFailed(_) => 0xEE04,
        }
    }
}
