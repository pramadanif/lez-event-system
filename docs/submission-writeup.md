# LP-0012 Submission Write-up

## What Was Built

A complete structured event system for Logos Execution Zone (LEZ) programs, consisting of:

1. **`lez-events` crate** — Core SDK for LEZ programs
   - `emit_event<E: LezEvent>(program_id, event) → Result<(), EventError>` — typed event emission, never panics on limits
   - `drain_events() → Vec<EventRecord>` — drain buffer before writing program output
   - `EventRecord` struct — Borsh-serializable, field-order frozen for wire compat
   - `LezEvent` trait + `impl_lez_event!` macro — reduce boilerplate
   - 4 stable error codes: `0xEE01–0xEE04`

2. **`lez-event-decoder` crate** — Decoder library + CLI
   - `decode_event()` — field-by-field Borsh decoding with schema support
   - `to_json()` / `to_display()` — JSON and human-readable terminal output
   - `lez-event-cli` — `decode`, `decode-raw`, `watch` subcommands

3. **Example programs**
   - `token-transfer` — success path, 2 events
   - `withdraw` — **critical failure path**: drain → write output → panic (events preserved)
   - `indexer` — reference indexer polling RPC for all tx events

4. **Test suite** — 21 tests, all passing

5. **CI/CD** — `.github/workflows/ci.yml` with `RISC0_DEV_MODE=0`

## Architecture Overview

```
Program                    Sequencer              Client
───────                    ─────────              ──────
emit_event(ev1)
emit_event(ev2)
  ↓
drain_events()
  ↓
ProgramOutput { events, ... }
  → env::commit()        ← Risc0 journal sealed
  [panic() optional]     ← state reverted, journal intact
                           ↓
                         Extract events from journal
                         Overwrite program_id on each event
                           ↓
                         TxReceipt {
                           success: false,
                           events: [ev1, ev2],  ← always present
                         }
                                                  ↓
                                               GET /tx/{hash}
                                               lez-event-cli decode
```

## Key Design Decisions

### Event Storage Mechanism (and why)

**Chosen**: Extend `ProgramOutput` with an `events: Vec<EventRecord>` field.

**Reasoning**: LEZ programs already write output via `env::commit(&ProgramOutput)`. Adding events to this struct means events are committed to the Risc0 journal at the same time as other output — before any potential panic. The sequencer reads the journal before reverting state, so events survive failure.

**Alternative considered**: A separate `env::write()` slot for events. Rejected because it requires more Risc0 plumbing and doesn't integrate cleanly with LEZ's existing `ProgramOutput` model.

### Encoding Format — Borsh (and why)

- **Already in LEZ workspace** — `borsh = "1.5.7"` is a workspace dependency
- **Deterministic** — same input always produces identical bytes (critical for Risc0 proving)
- **`no_std` compatible** — works inside Risc0 zkVM guest programs
- **Compact** — efficient binary format, no field name overhead
- **Widely used in blockchain** — Solana, NEAR use Borsh for exactly these reasons

### Failure Path Preservation — how events survive panic

The pattern that makes this work:

```rust
// 1. Emit events (goes to thread-local buffer)
emit_event(program_id, WithdrawAttempted { ... })?;
emit_event(program_id, InsufficientFunds { ... })?;

// 2. Drain and write output BEFORE panic
//    → ProgramOutput committed to Risc0 journal here
//    → journal is sealed and cannot be modified by a subsequent panic
let events = drain_events();
ProgramOutput::new(...).with_events(events).write();

// 3. Panic happens AFTER journal is sealed
panic!("Insufficient funds");
// State changes are reverted, but events in the journal remain.
```

The sequencer reads the journal entry before deciding to revert state. This is the same mechanism Risc0 uses for all program output — we're just piggybacking on it.

**Important**: The sequencer must extract events from the journal **before** the state-reversion decision. This is documented in `docs/architecture-decision.md`.

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

1. **Sequencer integration pending**: The `events` field on `ProgramOutput` is designed but not yet merged into the LEZ sequencer. The sequencer-side code to extract events and forward them to `TxReceipt` is documented in `docs/architecture-decision.md`.

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
use lez_events::{drain_events, emit_event, impl_lez_event};
use borsh::BorshSerialize;

#[derive(BorshSerialize)]
pub struct MyEvent { pub value: u64 }
impl_lez_event!(MyEvent, discriminant = 0x0001);

fn main() {
    let program_id = [/* your program id */; 32];
    
    emit_event(program_id, MyEvent { value: 42 }).expect("emit");
    
    // ALWAYS drain before write, even if you might panic
    let events = drain_events();
    ProgramOutput::new(/* ... */).with_events(events).write();
    
    // Optional: panic after write — events are preserved
}
```

## Benchmark Results

See [`docs/benchmarks.md`](benchmarks.md) for detailed measurements.

Summary: `emit_event()` with a 64-byte payload takes ~800 ns wall time. `drain_events()` with 64 events takes ~300 ns. Total overhead is negligible compared to Risc0 proving time.
