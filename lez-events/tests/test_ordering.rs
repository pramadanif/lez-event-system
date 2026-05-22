//! Integration tests verifying that events are assigned monotonically
//! increasing sequence numbers starting at zero.

use borsh::BorshSerialize;
use lez_events::{clear_events, drain_events, emit_event, impl_lez_event};

#[derive(BorshSerialize)]
struct DummyEvent {
    value: u64,
}
impl_lez_event!(DummyEvent, discriminant = 0x9001);

#[test]
fn first_event_has_sequence_zero() {
    clear_events();
    let pid = [0u8; 32];
    emit_event(pid, DummyEvent { value: 1 }).unwrap();
    let events = drain_events();
    assert_eq!(events[0].sequence, 0);
}

#[test]
fn sequence_increments_monotonically() {
    clear_events();
    let pid = [0u8; 32];
    for i in 0..5u64 {
        emit_event(pid, DummyEvent { value: i }).unwrap();
    }
    let events = drain_events();
    assert_eq!(events.len(), 5);
    for (i, e) in events.iter().enumerate() {
        assert_eq!(e.sequence, i as u32, "sequence at index {i}");
    }
}

#[test]
fn drain_resets_sequence_for_next_tx() {
    clear_events();
    let pid = [0u8; 32];
    emit_event(pid, DummyEvent { value: 0 }).unwrap();
    emit_event(pid, DummyEvent { value: 1 }).unwrap();
    let _ = drain_events(); // simulates end of tx

    // New transaction — sequence should restart at 0
    emit_event(pid, DummyEvent { value: 2 }).unwrap();
    let events = drain_events();
    assert_eq!(events[0].sequence, 0, "sequence must restart after drain");
}
