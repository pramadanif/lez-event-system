//! Integration tests for size limit enforcement.
//!
//! Verifies that `emit_event` returns `Err` (never panics) when
//! payload, event count, or total-byte limits are exceeded.

use borsh::BorshSerialize;
use lez_events::{
    clear_events, emit_event, impl_lez_event, EventError, MAX_EVENT_PAYLOAD_BYTES,
    MAX_EVENTS_PER_TX,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[derive(BorshSerialize)]
struct FixedPayload {
    data: Vec<u8>,
}
impl_lez_event!(FixedPayload, discriminant = 0x8001);

fn pid() -> [u8; 32] {
    [0u8; 32]
}

fn make_payload(n: usize) -> FixedPayload {
    FixedPayload {
        data: vec![0xFFu8; n],
    }
}

// ---------------------------------------------------------------------------
// Payload size tests
// ---------------------------------------------------------------------------

#[test]
fn exact_limit_succeeds() {
    clear_events();
    // The payload is Borsh-encoded inside emit_event.
    // FixedPayload { data: vec![...;N] } encodes as 4 bytes (len) + N bytes.
    // So to hit exactly 1024 we need N = 1024 - 4 = 1020.
    let event = make_payload(MAX_EVENT_PAYLOAD_BYTES - 4);
    let result = emit_event(pid(), event);
    assert!(result.is_ok(), "exact limit should succeed: {result:?}");
    clear_events();
}

#[test]
fn one_byte_over_limit_fails() {
    clear_events();
    // N = MAX_EVENT_PAYLOAD_BYTES - 4 + 1 → encoded len = 1025
    let event = make_payload(MAX_EVENT_PAYLOAD_BYTES - 4 + 1);
    let result = emit_event(pid(), event);
    assert!(
        matches!(result, Err(EventError::PayloadTooLarge { .. })),
        "should be PayloadTooLarge, got {result:?}"
    );
    clear_events();
}

#[test]
fn oversized_payload_returns_err_not_panic() {
    clear_events();
    let event = make_payload(MAX_EVENT_PAYLOAD_BYTES * 2);
    let result = emit_event(pid(), event);
    assert!(result.is_err(), "must not panic, must return Err");
    let code = result.unwrap_err().error_code();
    assert_eq!(code, 0xEE01, "error code must be 0xEE01 (PayloadTooLarge)");
    clear_events();
}

// ---------------------------------------------------------------------------
// Event count tests
// ---------------------------------------------------------------------------

#[test]
fn max_events_succeeds() {
    clear_events();
    for _ in 0..MAX_EVENTS_PER_TX {
        // Use tiny payloads so we don't hit the total-size limit
        emit_event(pid(), make_payload(1)).unwrap();
    }
    clear_events();
}

#[test]
fn one_over_max_events_fails_with_correct_code() {
    clear_events();
    for _ in 0..MAX_EVENTS_PER_TX {
        emit_event(pid(), make_payload(1)).unwrap();
    }
    let result = emit_event(pid(), make_payload(1));
    assert!(
        matches!(result, Err(EventError::TooManyEvents { .. })),
        "should be TooManyEvents, got {result:?}"
    );
    let code = result.unwrap_err().error_code();
    assert_eq!(code, 0xEE02, "error code must be 0xEE02 (TooManyEvents)");
    clear_events();
}

// ---------------------------------------------------------------------------
// Stable error code tests
// ---------------------------------------------------------------------------

#[test]
fn error_codes_are_stable() {
    let payload_err = EventError::PayloadTooLarge {
        limit: 1024,
        actual: 2048,
    };
    let count_err = EventError::TooManyEvents {
        limit: 64,
        actual: 65,
    };
    let total_err = EventError::TotalSizeTooLarge {
        limit: 65536,
        actual: 65537,
    };
    let enc_err = EventError::EncodingFailed("test".to_string());

    assert_eq!(payload_err.error_code(), 0xEE01);
    assert_eq!(count_err.error_code(), 0xEE02);
    assert_eq!(total_err.error_code(), 0xEE03);
    assert_eq!(enc_err.error_code(), 0xEE04);
}
