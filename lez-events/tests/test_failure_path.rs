//! Tests for the failure-path pattern.
//!
//! In a real LEZ program the pattern is:
//!   1. Emit events
//!   2. drain_events() + write output (commits to Risc0 journal)
//!   3. panic!()   ← state changes reverted, but journal already sealed
//!
//! Here we simulate this in-process to verify that the drain-before-panic
//! pattern preserves all emitted events.

use borsh::BorshSerialize;
use lez_events::{clear_events, drain_events, emit_event, impl_lez_event};
use std::panic;

#[derive(BorshSerialize)]
struct WithdrawAttempted {
    requested: u64,
}
impl_lez_event!(WithdrawAttempted, discriminant = 0x0010);

#[derive(BorshSerialize)]
struct InsufficientFunds {
    requested: u64,
    available: u64,
}
impl_lez_event!(InsufficientFunds, discriminant = 0x0011);

fn pid() -> [u8; 32] {
    [0u8; 32]
}

/// Simulates the failure path: emit → drain (preserving events) → panic.
///
/// Returns the events that were drained (i.e. those that would appear in
/// the TxReceipt) and the panic payload.
fn simulate_failed_withdraw(
    requested: u64,
    available: u64,
) -> (Vec<lez_events::EventRecord>, Box<dyn std::any::Any + Send>) {
    clear_events();

    // Emit attempt event
    emit_event(pid(), WithdrawAttempted { requested }).unwrap();

    // Balance check fails
    emit_event(
        pid(),
        InsufficientFunds {
            requested,
            available,
        },
    )
    .unwrap();

    // CRITICAL: drain BEFORE panic so events are preserved
    let events = drain_events();

    // Simulate panic (catch it so the test doesn't abort)
    let panic_result = panic::catch_unwind(|| {
        panic!("Insufficient funds");
    });

    (events, panic_result.unwrap_err())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn events_preserved_before_panic() {
    let (events, _panic) = simulate_failed_withdraw(2_000, 500);
    assert_eq!(events.len(), 2, "both events must be present despite panic");
}

#[test]
fn failed_tx_events_have_correct_discriminants() {
    let (events, _) = simulate_failed_withdraw(1_000, 0);
    assert_eq!(events[0].discriminant, 0x0010, "first event = WithdrawAttempted");
    assert_eq!(events[1].discriminant, 0x0011, "second event = InsufficientFunds");
}

#[test]
fn failed_tx_events_are_ordered() {
    let (events, _) = simulate_failed_withdraw(1_000, 0);
    assert_eq!(events[0].sequence, 0);
    assert_eq!(events[1].sequence, 1);
}

#[test]
fn buffer_empty_after_drain_even_on_panic_path() {
    clear_events();
    emit_event(pid(), WithdrawAttempted { requested: 1 }).unwrap();
    let drained = drain_events();
    // After drain the buffer is empty
    let remaining = drain_events();
    assert_eq!(drained.len(), 1);
    assert!(remaining.is_empty(), "buffer must be empty after drain");
}

#[test]
fn partial_events_preserved_when_panic_happens_after_drain() {
    clear_events();
    // Emit one event
    emit_event(pid(), WithdrawAttempted { requested: 42 }).unwrap();
    // Drain preserves it
    let events = drain_events();
    // "Panic" happens after drain — events still captured
    let _ = panic::catch_unwind(|| panic!("after drain"));
    assert_eq!(events.len(), 1, "partial events must survive");
}
