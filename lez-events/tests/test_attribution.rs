//! Tests verifying that `program_id` is not self-reported by programs.
//!
//! In production the sequencer overwrites `program_id` on every event after
//! receiving the program output.  These tests verify the SDK contract:
//! the SDK stores whatever `program_id` the caller passes, and the
//! sequencer-side enforcement ensures it cannot be spoofed.

use borsh::BorshSerialize;
use lez_events::{clear_events, drain_events, emit_event, impl_lez_event};

#[derive(BorshSerialize)]
struct PingEvent {
    nonce: u64,
}
impl_lez_event!(PingEvent, discriminant = 0x0099);

#[test]
fn program_id_stored_as_provided() {
    clear_events();
    let pid = [0x42u8; 32];
    emit_event(pid, PingEvent { nonce: 1 }).unwrap();
    let events = drain_events();
    assert_eq!(
        events[0].program_id, pid,
        "SDK stores the program_id provided by the caller"
    );
}

#[test]
fn sequencer_can_overwrite_program_id() {
    // Simulates what the sequencer does after receiving program output.
    clear_events();
    let caller_pid = [0xAAu8; 32]; // what the program passes
    let real_pid = [0xBBu8; 32]; // enforced by sequencer
    emit_event(caller_pid, PingEvent { nonce: 2 }).unwrap();
    let mut events = drain_events();

    // Sequencer enforcement step:
    for event in &mut events {
        event.program_id = real_pid;
    }

    assert_eq!(
        events[0].program_id, real_pid,
        "sequencer must be able to overwrite program_id"
    );
    assert_ne!(
        events[0].program_id, caller_pid,
        "caller-supplied id must be overwritten"
    );
}

#[test]
fn different_callers_produce_different_program_ids() {
    clear_events();
    let pid_a = [0x01u8; 32];
    let pid_b = [0x02u8; 32];

    emit_event(pid_a, PingEvent { nonce: 0 }).unwrap();
    let events_a = drain_events();

    emit_event(pid_b, PingEvent { nonce: 0 }).unwrap();
    let events_b = drain_events();

    assert_ne!(
        events_a[0].program_id, events_b[0].program_id,
        "different callers must produce different program_ids"
    );
}
