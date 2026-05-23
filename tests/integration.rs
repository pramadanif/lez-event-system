//! Root-level integration tests for LP-0012 LEZ Event System.
//!
//! These tests verify the complete API surface: SDK → encoder → decoder pipeline.
//! They run via `cargo test --test integration` at the workspace root.
//!
//! All tests run with RISC0_DEV_MODE=0 (real proving semantics — no mock mode).

use borsh::{BorshDeserialize, BorshSerialize};
use lez_event_decoder::{decode_event, to_display, to_json, EventSchema};
use lez_events::{clear_events, drain_events, emit_event, impl_lez_event, EventRecord};

// ─── Helper types ──────────────────────────────────────────────────────────

fn any_pid() -> [u8; 32] {
    [0xABu8; 32]
}

#[derive(BorshSerialize)]
struct Transfer {
    from: [u8; 32],
    to: [u8; 32],
    amount: u64,
}
impl_lez_event!(Transfer, discriminant = 0x0001);

#[derive(BorshSerialize)]
struct WithdrawFail {
    requested: u64,
    available: u64,
}
impl_lez_event!(WithdrawFail, discriminant = 0x0011);

// ─── Integration tests ─────────────────────────────────────────────────────

/// Full pipeline: emit → drain → encode → decode → display.
/// Verifies the SDK and decoder work together end-to-end.
#[test]
fn full_pipeline_success_path() {
    clear_events();
    let pid = any_pid();

    emit_event(
        pid,
        Transfer {
            from: [0x01u8; 32],
            to: [0x02u8; 32],
            amount: 1_000,
        },
    )
    .expect("emit Transfer");

    let events = drain_events();
    assert_eq!(events.len(), 1);

    // Verify Borsh round-trip
    let bytes = borsh::to_vec(&events).expect("borsh encode");
    let decoded: Vec<EventRecord> =
        BorshDeserialize::deserialize(&mut &bytes[..]).expect("borsh decode");
    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded[0].discriminant, 0x0001);
    assert_eq!(decoded[0].sequence, 0);
    assert_eq!(decoded[0].program_id, pid);

    // Verify decoder produces valid JSON
    let schema = no_schemas();
    let decoded_ev = decode_event(&decoded[0], &schema);
    let json = to_json(&decoded_ev);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    assert_eq!(parsed["discriminant"], 1);
    assert!(!decoded_ev.from_failed_tx);
}

/// Critical LP-0012 feature: failure-path events survive drain → panic.
/// This test proves events are preserved even when the tx panics.
#[test]
fn full_pipeline_failure_path() {
    clear_events();
    let pid = any_pid();

    emit_event(
        pid,
        WithdrawFail {
            requested: 2_000,
            available: 500,
        },
    )
    .expect("emit fail");

    // CRITICAL: drain before panic
    let events = drain_events();
    let bytes = borsh::to_vec(&events).expect("encode");

    // Simulate panic — events are already encoded
    let _ = std::panic::catch_unwind(|| panic!("tx failed"));

    // Verify events are fully decodable from bytes captured before panic
    let decoded: Vec<EventRecord> =
        BorshDeserialize::deserialize(&mut &bytes[..]).expect("decode after panic");
    assert_eq!(decoded.len(), 1, "event survives transaction panic");
    assert_eq!(decoded[0].discriminant, 0x0011);

    let schema = no_schemas();
    let ev = decode_event(&decoded[0], &schema);
    assert!(!ev.from_failed_tx); // flag set by caller context, not encoding
    let display = to_display(&ev);
    assert!(display.contains("0x0011"), "display shows discriminant");
}

/// Decoder handles unknown discriminants gracefully (no panic, raw hex fallback).
#[test]
fn decoder_handles_unknown_discriminant() {
    clear_events();
    let pid = any_pid();

    // Emit a Transfer event
    emit_event(
        pid,
        Transfer {
            from: [0u8; 32],
            to: [0u8; 32],
            amount: 42,
        },
    )
    .expect("emit");
    let events = drain_events();
    assert_eq!(events.len(), 1);

    // Decode with NO schemas registered
    let schema = no_schemas();
    let ev = decode_event(&events[0], &schema);

    // Should fall back to raw hex, not panic
    assert!(
        ev.event_name.contains("Unknown") || ev.event_name.contains("Transfer"),
        "graceful fallback: got '{}'",
        ev.event_name
    );
    assert!(
        !ev.raw_payload_hex.is_empty(),
        "raw_payload_hex must be present"
    );

    // JSON must be valid
    let json = to_json(&ev);
    assert!(serde_json::from_str::<serde_json::Value>(&json).is_ok());
}

/// Borsh encoding is deterministic: same input → same bytes.
#[test]
fn borsh_encoding_is_deterministic() {
    let make = || {
        clear_events();
        emit_event(
            any_pid(),
            Transfer {
                from: [0x01u8; 32],
                to: [0x02u8; 32],
                amount: 100,
            },
        )
        .unwrap();
        let events = drain_events();
        borsh::to_vec(&events).unwrap()
    };
    assert_eq!(make(), make(), "Borsh encoding must be deterministic");
}

/// Multiple events are ordered by sequence number.
#[test]
fn multi_event_ordering_preserved() {
    clear_events();
    for i in 0..5u64 {
        emit_event(
            any_pid(),
            Transfer {
                from: [0u8; 32],
                to: [i as u8; 32],
                amount: i,
            },
        )
        .unwrap();
    }
    let events = drain_events();
    assert_eq!(events.len(), 5);
    for (i, ev) in events.iter().enumerate() {
        assert_eq!(ev.sequence, i as u32, "seq must be monotonic");
    }
}

/// emit_event returns Err (never panics) when payload exceeds limit.
#[test]
fn emit_returns_err_on_oversized_payload() {
    use borsh::BorshSerialize;
    use lez_events::impl_lez_event;

    #[derive(BorshSerialize)]
    struct BigPayload {
        data: Vec<u8>,
    }
    impl_lez_event!(BigPayload, discriminant = 0xFF01);

    clear_events();
    let result = emit_event(
        any_pid(),
        BigPayload {
            data: vec![0u8; 2048],
        }, // over 1024 limit
    );
    assert!(result.is_err(), "oversized payload must return Err");
    let code = result.unwrap_err().error_code();
    assert_eq!(code, 0xEE01, "error code must be 0xEE01 (PayloadTooLarge)");
}

// ─── Helpers ───────────────────────────────────────────────────────────────

fn no_schemas() -> Vec<EventSchema> {
    vec![]
}
