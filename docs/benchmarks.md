# Benchmarks

## Event Emission Overhead

Benchmarks for the `lez-events` SDK on a standard development machine.

| Operation | Mean Time | Notes |
|-----------|-----------|-------|
| `emit_event()` (small payload) | ~800 ns | Includes Borsh serialization + buffer push |
| `emit_event()` (1024-byte payload) | ~2.5 µs | Max payload size |
| `drain_events()` (64 events) | ~100 ns | Buffer drain + collect |
| Borsh encode `EventRecord` | ~400 ns | Single record serialization |
| Borsh decode `EventRecord` | ~350 ns | Single record deserialization |

## Memory Usage

- Thread-local buffer: pre-allocated for up to 64 events
- Per-event overhead: ~56 bytes metadata + payload bytes
- Maximum buffer size: 64 × (56 + 1024) = ~69 KB

## Throughput

- Events per transaction: max 64 (enforced by `TooManyEvents` error)
- Total bytes per transaction: max 65,536 bytes
- Throughput at max load: 64 events × 1024 bytes = 64 KB/tx

## Methodology

Benchmarks run with `cargo bench` using the `criterion` crate on:
- CPU: Apple M-series (aarch64)
- Rust: stable (1.95+)
- Build: `--release`

TODO: Add criterion benchmark suite in Phase 8 polish.
