# LP-0012 Submission Write-up

## What Was Built

A complete structured event system for Logos Execution Zone (LEZ) programs, consisting of:

1. **`lez-events` crate** — Core SDK for LEZ programs
   - `emit_event<E: LezEvent>(program_id, event) → Result<(), EventError>` — typed event emission, never panics on limits
   - `execute_program() → Vec<EventRecord>` — drain buffer before writing program output
   - `EventRecord` struct — Borsh-serializable, field-order frozen for wire compat
   - `LezEvent` trait + `impl_lez_event!` macro — reduce boilerplate
   - 4 stable error codes: `0xEE01–0xEE04`

2. **`lez-event-decoder` crate** — Decoder library + CLI
   - `decode_event()` — field-by-field Borsh decoding with schema support
   - `to_json()` / `to_display()` — JSON and human-readable terminal output
   - `lez-event-cli` — `decode`, `decode-raw`, `watch` subcommands

3. **`lez-events-runtime` crate** — Runtime Adapter
   - `execute_program` — panic-catching wrapper that automatically drains events and commits the framed journal.
   - `parse_journal` — host-side decoder that safely extracts the `LEZE` frame before standard `ProgramOutput` parsing.

4. **Example programs**
   - `token-transfer` — success path, 2 events managed transparently by the runtime adapter
   - `withdraw` — **critical failure path**: `execute_program` catches panic → flushes frame → resumes panic (events preserved)
   - `indexer` — reference indexer polling RPC for all tx events

5. **Test suite** — 32 tests, all passing

6. **CI/CD** — `.github/workflows/ci.yml` with `RISC0_DEV_MODE=0`

## Architecture Overview

```
Program                    Runtime Adapter (`execute_program`)       Sequencer/Host
───────                    ───────────────────────────────────       ──────────────
emit_event(ev1)
emit_event(ev2)
  ↓
[panic!("failed")!]  →     catches panic via `catch_unwind`
                           execute_program()
                           frame events with `LEZE` magic bytes
                           commit_slice(&frame)                   →  RISC0 journal sealed
                           resume_unwind(panic)
                             ↓
                           (Program aborts/reverts state)
                                                                     parse_journal() slices off LEZE frame
                                                                     events attached to TxReceipt
                                                                     deserialize ProgramOutput
```

## The Evolution of the Architecture (Before vs. After)

### Before (SDK-Centric)
Initially, developers were forced to manually manage draining before panics. This was problematic because:
1. It leaked transport logic into the developer's application code.
2. It required an immediate, massive structural change to LEZ's core `ProgramOutput` and `TxReceipt` structs, which broke upstream compatibility.

### After (Runtime Adapter)
We moved to a **Minimally-Invasive Deterministic Framed Transport**.
1. **Developer Experience**: Developers only call `emit_event()`. They wrap their main logic in `execute_program(|| { ... })`.
2. **Under the Hood**: If a panic occurs, the adapter catches it, drains the buffer, prepends a deterministic `LEZE` byte-frame to the RISC0 journal, and resumes the panic.
3. **Sequencer Integration**: The sequencer doesn't need to change `ProgramOutput`. It just runs `parse_journal` to slice off the `LEZE` frame before doing what it normally does. This minimizes the blast radius on the core team's codebase.

## Key Design Decisions

### Event Storage Mechanism (and why)

**Chosen**: A deterministic `LEZE` byte frame prepended to the RISC0 journal using a runtime adapter (`execute_program`).

**Reasoning**: LEZ programs write output to the RISC0 journal. By framing events at the start of the journal *before* a panic crashes the VM, we guarantee survival. By decoupling this from `ProgramOutput` structurally, we ensure that the LEZ sequencer can adopt this without rewriting core transaction primitives.

**Alternative considered**: Waiting for upstream `ProgramOutput` to support events natively. Rejected because it forced developers to manually manage `execute_program()` before they might panic, and required massive breaking changes to Logos Execution Zone's core structs.

### Encoding Format — Borsh (and why)

- **Already in LEZ workspace** — `borsh = "1.5.7"` is a workspace dependency
- **Deterministic** — same input always produces identical bytes (critical for Risc0 proving)
- **`no_std` compatible** — works inside Risc0 zkVM guest programs
- **Compact** — efficient binary format, no field name overhead
- **Widely used in blockchain** — Solana, NEAR use Borsh for exactly these reasons

### Failure Path Preservation — how events survive panic

The pattern that makes this work:

```rust
execute_program(|| {
    // 1. Emit events (goes to thread-local buffer)
    emit_event(program_id, WithdrawAttempted { ... })?;
    
    // 2. Panic happens!
    panic!("Insufficient funds");
});
// 3. UNDER THE HOOD:
//    execute_program catches the panic, drains the buffer,
//    frames it, commits to RISC0 journal, and resumes the panic.
//    State changes are reverted, but events in the journal remain.
```

The sequencer reads the journal entry before deciding to revert state, slices off the `LEZE` frame using `parse_journal`, and attaches the events to the receipt.

### Size Limits — values chosen and why

| Limit | Value | Reasoning |
|-------|-------|-----------|
| Max payload bytes | 1,024 | Matches common blockchain event payload limits; keeps Risc0 journal size manageable |
| Max events per tx | 64 | Prevents abuse while allowing rich event traces |
| Max total bytes | 65,536 | 64 × 1,024; keeps total journal overhead bounded |

All limits produce `EventError` (not panic, not silent truncation). Error codes are stable and documented.

## Trade-offs Considered

| Option | Pro | Con | Decision |
|--------|-----|-----|----------|
| Thread-local buffer | Simple, no Risc0 plumbing needed | Not shared across threads (LEZ is single-threaded) | ✓ Chosen |
| `env::write()` per event | Events committed immediately | Harder to sequence; more Risc0 plumbing | ✗ Rejected |
| Borsh encoding | Deterministic, compact, in-tree | Requires schema versioning discipline | ✓ Chosen |
| JSON encoding | Human-readable | Non-deterministic (key order), verbose, no `no_std` | ✗ Rejected |

## Privacy Considerations

Events are **public on-chain data** visible to all network participants:

- **Never emit**: private keys, viewing keys, hidden balances, nullifier secrets
- **Safe to emit**: error types, public amounts, status codes, program IDs

For private (ZKP) execution: the event payload is committed to the Risc0 journal and becomes part of the public proof. Programs must be explicit about what data is safe to include.

The decoder CLI and indexer example are designed to work with public data only.

## Security Assumptions

1. **program_id enforcement**: The SDK stores whatever `program_id` the caller passes. In production, the sequencer **must** overwrite `event.program_id = running_program_id` after receiving program output. This prevents programs from attributing events to other programs.

2. **Single-threaded execution**: The thread-local buffer assumes Risc0 zkVM guest programs are single-threaded (which they are). Multi-threaded hosts (e.g., tests) must call `clear_events()` between test cases.

3. **Journal immutability**: Once `env::commit()` is called, the Risc0 journal cannot be modified. Events committed before a panic are permanent.

## Known Limitations

1. **Sequencer integration pending**: The runtime integration is designed but not yet merged into the LEZ sequencer. The 3-line sequencer-side code diff to extract events is documented in `docs/runtime-integration.md`.

2. **No on-chain event indexing**: Events are available via RPC receipt but not stored in any global state. An external indexer (like the provided example) is needed for querying.

3. **Compute unit tracking**: Real CU costs require a running LEZ sequencer with CU metering. See `docs/benchmarks.md` for current estimates.

## Integration Instructions for Other LEZ Programs

```toml
# Cargo.toml
[dependencies]
lez-events = { git = "https://github.com/pramadanif/lez-event-system" }
borsh = "1.5.0"
```

```rust
// In your LEZ program:
use lez_events::{emit_event, impl_lez_event};
use lez_events_runtime::execute_program;
use borsh::BorshSerialize;

#[derive(BorshSerialize)]
pub struct MyEvent { pub value: u64 }
impl_lez_event!(MyEvent, discriminant = 0x0001);

fn main() {
    execute_program(|| {
        let program_id = [/* your program id */; 32];
        
        emit_event(program_id, MyEvent { value: 42 }).expect("emit");
        
        // Write standard program output here
        
        // Optional: panic — the runtime catches it and flushes events!
    });
}
```

## Benchmark Results

See [`docs/benchmarks.md`](benchmarks.md) for detailed measurements.

Summary: `emit_event()` with a 64-byte payload takes ~800 ns wall time. `execute_program()` with 64 events takes ~300 ns. Total overhead is negligible compared to Risc0 proving time.
