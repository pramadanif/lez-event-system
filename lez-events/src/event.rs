use borsh::{BorshDeserialize, BorshSerialize};

pub const MAX_EVENT_PAYLOAD_BYTES: usize = 1024;
pub const MAX_EVENTS_PER_TX: u32 = 64;
/// Per-transaction total bytes cap (64 events × 1024 bytes each)
pub const MAX_TOTAL_EVENT_BYTES: usize = MAX_EVENTS_PER_TX as usize * MAX_EVENT_PAYLOAD_BYTES;

/// Canonical on-chain event record for LEZ programs.
///
/// Field order is frozen — changing it breaks Borsh encoding compatibility.
/// `program_id` is enforced by the sequencer; programs cannot set it.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub struct EventRecord {
    /// Program that emitted this event. Overwritten by sequencer.
    pub program_id: [u8; 32],
    /// 0-indexed position in this transaction's event list.
    pub sequence: u32,
    /// Event type identifier — defined by the program author.
    pub discriminant: u64,
    /// Schema version for forward compatibility; always 1 for v1.
    pub schema_version: u8,
    /// Borsh-encoded event payload; max `MAX_EVENT_PAYLOAD_BYTES`.
    pub payload: Vec<u8>,
}

/// Implemented by every event struct that can be emitted via [`crate::emit_event`].
///
/// Use the [`impl_lez_event!`] macro to reduce boilerplate.
pub trait LezEvent: BorshSerialize {
    const DISCRIMINANT: u64;
    const SCHEMA_VERSION: u8 = 1;

    /// Borsh-encode `self` into the payload bytes.
    fn encode_payload(&self) -> Result<Vec<u8>, crate::EventError> {
        borsh::to_vec(self).map_err(|e| crate::EventError::EncodingFailed(e.to_string()))
    }
}
