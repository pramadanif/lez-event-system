# LP-0012 Submission Writeup

## Overview

This submission implements a complete structured event/log system for Logos Execution Zone (LEZ) programs. The key innovation is that **events survive transaction failures** — programs emit events, drain them into the Risc0 journal, and only then panic. The sequencer extracts events before deciding success/failure, ensuring both success and failed transactions have full event history.

## What Was Built

### `lez-events` crate (Core SDK)

- `emit_event<E: LezEvent>(program_id, event)` → `Result<(), EventError>` — never panics on overflow
- `drain_events()` → `Vec<EventRecord>` — drain before writing output
- `EventRecord` — Borsh-serializable, field-order frozen
- `LezEvent` trait + `impl_lez_event!` macro — reduces boilerplate
- All 4 stable error codes: `0xEE01`–`0xEE04`

### `lez-event-decoder` crate (Decoder + CLI)

- `decode_event()` — field-by-field Borsh decoding with schema support
- `to_json()` / `to_display()` — JSON and human-readable output
- `lez-event-cli` binary:
  - `decode --tx HASH --rpc URL` — fetch from live sequencer
  - `decode-raw --hex HEX` — offline decoding (no sequencer needed)
  - `watch --rpc URL` — real-time event stream

### Examples

- `token-transfer` — success path with 2 events
- `withdraw` — **critical failure path**: drain → write → panic (events preserved)
- `indexer` — reference indexer polling RPC for all tx events

### Test Suite (21 tests, all passing)

- `test_encoding.rs` — Borsh determinism, round-trip, wire format spec
- `test_ordering.rs` — sequence number monotonicity
- `test_size_limits.rs` — all limit checks, stable error codes
- `test_failure_path.rs` — failure-path event preservation (core of LP-0012)
- `test_attribution.rs` — program_id attribution and sequencer override

## Key Design Decisions

### Failure Path Mechanism

The hardest problem in LP-0012: how do events survive when a Risc0 guest panics?

**Solution**: Programs must follow the pattern:
```
emit events → drain_events() → write to journal → panic
```

Events are committed to the Risc0 journal *before* the panic. The sequencer reads the journal before deciding to revert state changes. This ensures events appear in `TxReceipt.events` even when `success=false`.

This is documented in `docs/architecture-decision.md` and demonstrated in the `withdraw` example.

### Encoding: Borsh

Borsh was chosen because:
1. Already in LEZ workspace dependencies
2. Deterministic (same input → identical bytes always)
3. `no_std` compatible for zkVM guest programs
4. Compact binary format

### Thread-Local Buffer

Events are buffered in a `thread_local! { static RefCell<Vec<EventRecord>> }` because:
- LEZ programs run in a single-threaded Risc0 zkVM environment
- No heap allocation overhead per event beyond payload
- Simple drain semantics (clear on drain, restart sequence at 0)

## Verification

```bash
cargo build --workspace            # all crates compile
cargo clippy --workspace -- -D warnings  # zero warnings
cargo test --workspace             # 21/21 tests pass
./scripts/demo.sh                  # end-to-end demo
```

## Checklist Against Instant Fail Conditions

- [x] Events NOT missing from receipt when tx panics (drain-before-panic pattern)
- [x] `emit_event()` returns `Err`, never panics on overflow
- [x] `RISC0_DEV_MODE=0` in CI workflow and demo script
- [ ] Video demo with narration (TODO: record)
- [x] CI pipeline (`.github/workflows/ci.yml`)
- [x] `demo.sh` works from clean environment
- [x] Programs cannot set their own `program_id` (sequencer overwrites)
- [x] `cargo clippy` produces zero warnings
- [x] `docs/event-format.md` present and complete
- [x] Reference indexer example present
