//! Token-transfer example — demonstrates the **success path**.
//!
//! This program simulates a token transfer between two accounts.
//! It emits two events (`TransferInitiated`, `TransferCompleted`),
//! drains them before writing output, and exits normally.
//!
//! In a real LEZ program the NSSA helpers (`read_nssa_inputs`,
//! `write_nssa_outputs`) would be provided by the LEZ runtime.
//! Here we use simple in-process stubs so the example can be
//! compiled and run without a full LEZ node.

use borsh::BorshSerialize;
use lez_events::{drain_events, emit_event, impl_lez_event};

// ---------------------------------------------------------------------------
// Event types
// ---------------------------------------------------------------------------

#[derive(BorshSerialize)]
pub struct TransferInitiated {
    pub from: [u8; 32],
    pub to: [u8; 32],
    pub amount: u64,
}
impl_lez_event!(TransferInitiated, discriminant = 0x0001);

#[derive(BorshSerialize)]
pub struct TransferCompleted {
    pub from: [u8; 32],
    pub to: [u8; 32],
    pub amount: u64,
    pub new_sender_balance: u64,
}
impl_lez_event!(TransferCompleted, discriminant = 0x0002);

// ---------------------------------------------------------------------------
// Simulated LEZ runtime stubs
// ---------------------------------------------------------------------------

struct TransferInstruction {
    from: [u8; 32],
    to: [u8; 32],
    amount: u64,
    initial_balance: u64,
}

fn read_inputs() -> (TransferInstruction, [u8; 32]) {
    // Stub: fixed program_id and instruction data
    let program_id = [0xABu8; 32];
    let instruction = TransferInstruction {
        from: [0x01u8; 32],
        to: [0x02u8; 32],
        amount: 1_000,
        initial_balance: 5_000,
    };
    (instruction, program_id)
}

fn write_outputs(events: Vec<lez_events::EventRecord>) {
    // Stub: print summary to stdout
    println!("=== program output ===");
    println!("events committed: {}", events.len());
    for e in &events {
        println!(
            "  [seq={}] discriminant=0x{:04x} payload_len={}",
            e.sequence,
            e.discriminant,
            e.payload.len()
        );
    }
}

// ---------------------------------------------------------------------------
// Main program logic — success path
// ---------------------------------------------------------------------------

fn main() {
    let (instr, program_id) = read_inputs();

    // 1. Emit initial event
    emit_event(
        program_id,
        TransferInitiated {
            from: instr.from,
            to: instr.to,
            amount: instr.amount,
        },
    )
    .expect("emit TransferInitiated");

    // 2. Apply transfer logic
    assert!(
        instr.initial_balance >= instr.amount,
        "insufficient balance"
    );
    let new_balance = instr.initial_balance - instr.amount;

    // 3. Emit completion event
    emit_event(
        program_id,
        TransferCompleted {
            from: instr.from,
            to: instr.to,
            amount: instr.amount,
            new_sender_balance: new_balance,
        },
    )
    .expect("emit TransferCompleted");

    // 4. Drain events BEFORE writing output
    let events = drain_events();

    // 5. Write output (events are committed here)
    write_outputs(events);

    println!(
        "Transfer of {} tokens completed successfully.",
        instr.amount
    );
}
