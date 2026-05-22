use std::cell::RefCell;

use crate::{
    event::{LezEvent, MAX_EVENT_PAYLOAD_BYTES, MAX_EVENTS_PER_TX, MAX_TOTAL_EVENT_BYTES},
    EventError, EventRecord,
};

thread_local! {
    static EVENT_BUFFER: RefCell<Vec<EventRecord>> = const { RefCell::new(Vec::new()) };
}

/// Emit a structured event from a LEZ program.
///
/// # Errors
///
/// Returns `Err` (never panics) when:
/// - The Borsh-encoded payload exceeds [`MAX_EVENT_PAYLOAD_BYTES`] (→ `0xEE01`)
/// - The per-transaction event count reaches [`MAX_EVENTS_PER_TX`] (→ `0xEE02`)
/// - The cumulative payload bytes exceed [`MAX_TOTAL_EVENT_BYTES`] (→ `0xEE03`)
/// - Borsh serialisation fails (→ `0xEE04`)
pub fn emit_event<E: LezEvent>(program_id: [u8; 32], event: E) -> Result<(), EventError> {
    let payload = event.encode_payload()?;

    if payload.len() > MAX_EVENT_PAYLOAD_BYTES {
        return Err(EventError::PayloadTooLarge {
            limit: MAX_EVENT_PAYLOAD_BYTES,
            actual: payload.len(),
        });
    }

    EVENT_BUFFER.with(|buf| {
        let mut buf = buf.borrow_mut();

        if buf.len() as u32 >= MAX_EVENTS_PER_TX {
            return Err(EventError::TooManyEvents {
                limit: MAX_EVENTS_PER_TX,
                actual: buf.len() as u32,
            });
        }

        let current_total: usize = buf.iter().map(|r| r.payload.len()).sum();
        if current_total + payload.len() > MAX_TOTAL_EVENT_BYTES {
            return Err(EventError::TotalSizeTooLarge {
                limit: MAX_TOTAL_EVENT_BYTES,
                actual: current_total + payload.len(),
            });
        }

        let sequence = buf.len() as u32;
        buf.push(EventRecord {
            program_id,
            sequence,
            discriminant: E::DISCRIMINANT,
            schema_version: E::SCHEMA_VERSION,
            payload,
        });
        Ok(())
    })
}

/// Drain all buffered events for this transaction.
///
/// Call this immediately before `write_nssa_outputs()` so that events
/// are captured in the program output even when the program panics
/// (the drain + write happens *before* any potential panic).
pub fn drain_events() -> Vec<EventRecord> {
    EVENT_BUFFER.with(|buf| buf.borrow_mut().drain(..).collect())
}

/// Clear the event buffer without returning events.
///
/// Intended for test teardown — clears state between test cases that share
/// a thread.
pub fn clear_events() {
    EVENT_BUFFER.with(|buf| buf.borrow_mut().clear());
}

/// Return the number of currently buffered events (test helper).
pub fn event_count() -> usize {
    EVENT_BUFFER.with(|buf| buf.borrow().len())
}
