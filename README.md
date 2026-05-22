# LEZ Event System (LP-0012)

Structured event/log system for [Logos Execution Zone (LEZ)](https://github.com/logos-blockchain/logos-execution-zone) programs.
**Events survive transaction failures** — even when a program panics, all events emitted before the panic are preserved in the `TxReceipt`.

## Overview

LEZ programs currently provide no structured feedback to clients. This library adds a first-class event system inspired by Solana's `meta.logMessages` and Cosmos SDK ABCI events, enabling:
- **Developers** to debug failing transactions
- **Wallets** to show meaningful post-transaction narratives
- **Indexers** to reliably classify on-chain activity

## Architecture

```
LEZ Program calls emit_event(...)
         ↓
Thread-local event buffer (Vec<EventRecord>)
         ↓
drain_events() — called BEFORE any potential panic
         ↓
ProgramOutput { events, ... } written to Risc0 journal
         ↓
program panics (optional) ← state reverted, but journal already sealed
         ↓
Sequencer reads journal → overwrites program_id on all events
         ↓
TxReceipt { success: bool, events: Vec<EventRecord> }
                                    ↑ always present, even when success=false
```

## Quick Start

### Prerequisites

- **Rust 1.94.0** (pinned in `rust-toolchain.toml`, matching LEZ)
- **RISC0** toolchain: `curl -L https://risczero.com/install | bash && rzup install`
- **LEZ repository** cloned for integration tests:  
  `git clone https://github.com/logos-blockchain/logos-execution-zone`

### Installation

Add to your LEZ program's `Cargo.toml`:

```toml
[dependencies]
lez-events = { git = "https://github.com/pramadanif/lez-event-system", tag = "v0.1.0" }
borsh = "1.5.0"
```

### Emit Events from a LEZ Program

```rust
use lez_events::{drain_events, emit_event, impl_lez_event};
use borsh::BorshSerialize;

// Define your event type
#[derive(BorshSerialize)]
pub struct InsufficientFunds {
    pub account: [u8; 32],
    pub requested: u64,
    pub available: u64,
}
impl_lez_event!(InsufficientFunds, discriminant = 0x0011);

fn main() {
    let (ProgramInput { pre_states, instruction: Withdraw { amount } }, instruction_data)
        = read_nssa_inputs::<Instruction>();

    let program_id = get_program_id(); // from LEZ context
    let balance = read_balance(&pre_states[0].account);

    emit_event(program_id, InsufficientFunds {
        account: pre_states[0].account.address,
        requested: amount,
        available: balance,
    }).expect("emit event");

    // CRITICAL: drain BEFORE panic — events survive in Risc0 journal
    let events = drain_events();
    ProgramOutput::new(program_id, None, instruction_data, pre_states, vec![])
        .with_events(events)  // attach events to output
        .write();

    panic!("Insufficient funds"); // tx fails, but events are in receipt!
}
```

### Retrieve Events After Execution

Events are returned in the `TxReceipt.events` field from the LEZ RPC:

```
GET /tx/{tx_hash}
Response: { "success": false, "events": [...], ... }
```

Even when `success=false`, the `events` array is non-empty if the program emitted events.

### Decode Events (CLI)

```bash
# Install the CLI
cargo install --path lez-event-decoder

# Decode from live RPC
lez-event-cli decode --tx <TX_HASH> --rpc http://localhost:8080

# JSON output
lez-event-cli decode --tx <TX_HASH> --rpc http://localhost:8080 --format json

# Offline decode (no sequencer needed)
lez-event-cli decode-raw --hex <BORSH_HEX>
lez-event-cli decode-raw --file events.bin

# Watch events in real-time
lez-event-cli watch --rpc http://localhost:8080
lez-event-cli watch --program <PROGRAM_ID> --rpc http://localhost:8080
```

## API Reference

### `emit_event<E: LezEvent>(program_id: [u8; 32], event: E) -> Result<(), EventError>`

Emit a typed event from a LEZ program. Serializes `event` with Borsh and appends to the thread-local buffer.

**Never panics** on size violations — returns `Err(EventError)` instead.

| Limit | Value |
|-------|-------|
| Max payload bytes | 1,024 |
| Max events per tx | 64 |
| Max total bytes per tx | 65,536 |

### `drain_events() -> Vec<EventRecord>`

Drain all buffered events. Call this **immediately before** writing program output, so events are included in the Risc0 journal even if the program panics afterward.

### `clear_events()`

Clear the event buffer without returning events. Used in test teardown.

### `EventRecord` struct

```rust
pub struct EventRecord {
    pub program_id: [u8; 32],  // overwritten by sequencer — cannot be spoofed
    pub sequence: u32,         // 0-indexed, monotonically increasing per tx
    pub discriminant: u64,     // event type identifier (program-defined)
    pub schema_version: u8,    // 1 for v1; forward-compatible
    pub payload: Vec<u8>,      // Borsh-encoded event fields, max 1024 bytes
}
```

Field order is **frozen** — changing it breaks Borsh wire compatibility.

### `EventError` variants and error codes

| Error | Code | Condition |
|-------|------|-----------|
| `PayloadTooLarge { limit, actual }` | `0xEE01` | Payload > 1024 bytes |
| `TooManyEvents { limit, actual }` | `0xEE02` | > 64 events per tx |
| `TotalSizeTooLarge { limit, actual }` | `0xEE03` | Sum > 65,536 bytes |
| `EncodingFailed(String)` | `0xEE04` | Borsh serialization error |

Error codes are **stable** and will never change.

### `LezEvent` trait

```rust
pub trait LezEvent: BorshSerialize {
    const DISCRIMINANT: u64;
    const SCHEMA_VERSION: u8 = 1;
}
```

Implement via the `impl_lez_event!` macro:

```rust
impl_lez_event!(MyEvent, discriminant = 0x0001);
// or with custom schema version:
impl_lez_event!(MyEventV2, discriminant = 0x0001, schema_version = 2);
```

## Deployment

### Deploy to LEZ Testnet

```bash
# Build release binaries
cargo build --workspace --release

# Deploy example programs (requires LEZ wallet)
cd logos-execution-zone
just run-wallet deploy-program ../target/release/token-transfer-example
just run-wallet deploy-program ../target/release/withdraw-example
```

### Deployed Program Addresses

See [`docs/deployments.md`](docs/deployments.md) for testnet program IDs and RPC endpoints.

## Running Tests

### Unit tests (no sequencer required)

```bash
cargo test --workspace
```

All 21 tests should pass:
- `test_encoding` — Borsh determinism, round-trip, wire format
- `test_ordering` — sequence number monotonicity (0, 1, 2, ...)
- `test_size_limits` — all limits return `Err`, never panic
- `test_failure_path` — events survive panic (drain-before-panic pattern)
- `test_attribution` — program_id attribution and sequencer override

### Integration tests (requires LEZ sequencer)

```bash
# Start LEZ standalone sequencer first
cd logos-execution-zone
RUST_LOG=info cargo run --features standalone -p sequencer_service sequencer/service/configs/debug &

# Run integration tests
cd lez-event-system
RISC0_DEV_MODE=0 ./scripts/run-integration-tests.sh
```

## Running the Demo (`./scripts/demo.sh`)

```bash
# Set RISC0_DEV_MODE=0 (required — real proving, not mock)
export RISC0_DEV_MODE=0

# Set path to LEZ clone
export LEZ_DIR=/path/to/logos-execution-zone

# Run
./scripts/demo.sh
```

The demo:
1. Builds all workspace crates
2. Starts LEZ standalone sequencer
3. Deploys `token-transfer` and `withdraw` programs
4. Submits a successful token transfer → shows events in receipt
5. Submits a failing withdraw (amount > balance) → shows events **still in receipt**
6. Demonstrates JSON output with `from_failed_tx: true`
7. Runs all tests

## Privacy Considerations

Events are **public on-chain data**. Programs should never emit:
- Private keys or viewing keys
- Hidden balance values
- Nullifier secrets

Safe to emit: error codes, public amounts, status flags, program IDs, transaction metadata.

For **private (ZKP) execution**: only emit data that is safe to reveal publicly. The event payload is committed to the Risc0 journal and becomes visible to all network participants.

## Known Limitations

1. **Sequencer integration**: The `events` field on `ProgramOutput` requires a sequencer that reads and forwards events. The SDK is ready; sequencer-side changes to forward events to `TxReceipt` are required.
2. **Program ID**: Currently programs pass their own `program_id`. Full enforcement requires sequencer to overwrite it (code documented in `docs/architecture-decision.md`).
3. **No on-chain indexing**: Events are available via RPC on the receipt but not stored in global state.

## License

MIT OR Apache-2.0
