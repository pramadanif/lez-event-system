//! Withdraw example — demonstrates the **failure path**.
//!
//! # Critical pattern
//!
//! Events are drained and written to the program output **before** the
//! program panics.  This ensures that the Risc0 journal contains the events
//! even when the transaction ultimately fails:
//!
//! ```text
//! emit(WithdrawAttempted)
//! emit(InsufficientFunds)        ← only if balance check fails
//! panic!("Insufficient funds")   
//!
//! Runtime wrapper catches panic, flushes frame, and resumes panic.
//! ```
//!
//! The sequencer reads the journal before deciding to revert state, so
//! both events appear in the `TxReceipt.events` field even though
//! `success = false`.

use borsh::BorshSerialize;
use lez_events::{emit_event, impl_lez_event};
use lez_events_runtime::execute_program;

// ---------------------------------------------------------------------------
// Event types
// ---------------------------------------------------------------------------

#[derive(BorshSerialize)]
pub struct WithdrawAttempted {
    pub account: [u8; 32],
    pub requested: u64,
}
impl_lez_event!(WithdrawAttempted, discriminant = 0x0010);

#[derive(BorshSerialize)]
pub struct InsufficientFunds {
    pub account: [u8; 32],
    pub requested: u64,
    pub available: u64,
}
impl_lez_event!(InsufficientFunds, discriminant = 0x0011);

#[derive(BorshSerialize)]
pub struct WithdrawCompleted {
    pub account: [u8; 32],
    pub amount: u64,
    pub remaining: u64,
}
impl_lez_event!(WithdrawCompleted, discriminant = 0x0012);

// ---------------------------------------------------------------------------
// Simulated LEZ runtime stubs
// ---------------------------------------------------------------------------

struct WithdrawInstruction {
    account: [u8; 32],
    amount: u64,
    balance: u64,
}

fn read_inputs() -> (WithdrawInstruction, [u8; 32]) {
    // Stub: simulate an account with insufficient funds (balance < requested)
    let program_id = [0xCDu8; 32];
    let instruction = WithdrawInstruction {
        account: [0x03u8; 32],
        amount: 2_000,
        balance: 500, // ← too low → will trigger failure path
    };
    (instruction, program_id)
}

// ---------------------------------------------------------------------------
// Main program logic — failure path
// ---------------------------------------------------------------------------

fn main() {
    execute_program(|| {
        let (instr, program_id) = read_inputs();

        // Event 1: always emitted — records the attempt
        emit_event(
            program_id,
            WithdrawAttempted {
                account: instr.account,
                requested: instr.amount,
            },
        )
        .expect("emit WithdrawAttempted");

        if instr.balance < instr.amount {
            // Event 2: emitted before panic so it is preserved in the journal
            emit_event(
                program_id,
                InsufficientFunds {
                    account: instr.account,
                    requested: instr.amount,
                    available: instr.balance,
                },
            )
            .expect("emit InsufficientFunds");

            // CRITICAL: Panic here!
            // The runtime wrapper `execute_program` automatically catches this panic,
            // frames the events, and writes them to the journal before aborting.
            panic!(
                "Insufficient funds: requested {}, available {}",
                instr.amount, instr.balance
            );
        }

        // Success path
        let remaining = instr.balance - instr.amount;
        emit_event(
            program_id,
            WithdrawCompleted {
                account: instr.account,
                amount: instr.amount,
                remaining,
            },
        )
        .expect("emit WithdrawCompleted");

        println!("Withdrawal of {} tokens completed.", instr.amount);
    });
}
