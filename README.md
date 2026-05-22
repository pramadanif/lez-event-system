# LEZ Event System (LP-0012)

Structured event/log system for [Logos Execution Zone (LEZ)](https://github.com/logos-blockchain/logos-execution-zone) programs.

**Events survive transaction failures** — both successful and failed transactions include their events in the `TxReceipt`, enabling developers to understand exactly what happened even when a program panics.

## Quick Start

```bash
# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Zero clippy warnings
cargo clippy --workspace -- -D warnings

# Run the demo
./scripts/demo.sh

# Decode events offline (no sequencer needed)
cargo run --bin lez-event-cli -- decode-raw --hex <BORSH_HEX>

# Decode from live RPC
cargo run --bin lez-event-cli -- decode --tx <TX_HASH> --rpc http://localhost:8545

# Watch events in real-time
cargo run --bin lez-event-cli -- watch --rpc http://localhost:8545
```

## Architecture

```
Program emits events → Thread-local buffer
                     ↓
               drain_events()         ← before potential panic
                     ↓
         write to program output      ← committed to Risc0 journal
                     ↓
            panic!() (optional)       ← state reverted, events preserved
                     ↓
    Sequencer extracts events         ← before deciding success/failure
                     ↓
    TxReceipt.events (always present) ← even when success=false
```

## Event Format

```rust
pub struct EventRecord {
    pub program_id: [u8; 32],  // enforced by sequencer, not self-reported
    pub sequence: u32,         // 0-indexed, monotonically increasing
    pub discriminant: u64,     // event type ID defined by program author
    pub schema_version: u8,    // 1 for v1 (forward compat)
    pub payload: Vec<u8>,      // borsh-encoded, max 1024 bytes
}
```

See [`docs/event-format.md`](docs/event-format.md) for the full specification.

## Usage in LEZ Programs

```rust
use lez_events::{drain_events, emit_event, impl_lez_event};
use borsh::BorshSerialize;

#[derive(BorshSerialize)]
pub struct InsufficientFunds {
    pub account: [u8; 32],
    pub requested: u64,
    pub available: u64,
}
impl_lez_event!(InsufficientFunds, discriminant = 0x0011);

fn main() {
    let program_id = get_program_id(); // from LEZ context

    emit_event(program_id, InsufficientFunds {
        account: sender,
        requested: amount,
        available: balance,
    }).expect("emit event");

    // CRITICAL: drain BEFORE panic so events survive in journal
    let events = drain_events();
    write_outputs(events);

    panic!("Insufficient funds"); // events still in receipt!
}
```

## Crates

| Crate | Description |
|-------|-------------|
| `lez-events` | Core SDK: `emit_event`, `drain_events`, `EventRecord`, `LezEvent` trait |
| `lez-event-decoder` | Borsh decoder + `lez-event-cli` binary |

## Error Codes (Stable)

| Code | Variant | Meaning |
|------|---------|---------|
| `0xEE01` | `PayloadTooLarge` | Payload > 1024 bytes |
| `0xEE02` | `TooManyEvents` | > 64 events per tx |
| `0xEE03` | `TotalSizeTooLarge` | Total payload > 65536 bytes |
| `0xEE04` | `EncodingFailed` | Borsh serialization error |

## CLI

```bash
# Fetch and decode from live sequencer
lez-event-cli decode --tx <TX_HASH> --rpc <URL>
lez-event-cli decode --tx <TX_HASH> --rpc <URL> --format json

# Offline decode (no sequencer needed)
lez-event-cli decode-raw --hex <BORSH_HEX>
lez-event-cli decode-raw --file <PATH>

# Real-time event stream
lez-event-cli watch --rpc <URL>
lez-event-cli watch --program <PROGRAM_ID> --rpc <URL>
```

## Examples

| Example | Description |
|---------|-------------|
| `token-transfer` | Success path: emits 2 events, drains before output |
| `withdraw` | **Failure path**: emits events, drains, writes output, then panics |
| `indexer` | Reference indexer: polls RPC, extracts events from all txs |

## Privacy

Events are **public on-chain data**. Never emit:
- Private keys or viewing keys
- Hidden balance values
- Nullifier secrets

Safe to emit: error codes, public amounts, status flags, program IDs.

## CI/CD

```bash
# CI uses RISC0_DEV_MODE=0 (real proving, not mock)
RISC0_DEV_MODE=0 cargo test --workspace
```

See [`.github/workflows/ci.yml`](.github/workflows/ci.yml).

## License

MIT OR Apache-2.0
# lez-event-system
