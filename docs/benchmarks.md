# Compute Unit Benchmarks

## Environment

- Network: LEZ testnet (standalone mode)
- RISC0_DEV_MODE: 0 (real proving, not mock — as required by LP-0012)
- Rust: 1.94.0 (matching LEZ rust-toolchain.toml)
- Build profile: `--release`
- Platform: Apple M-series (aarch64-apple-darwin)

> **Note**: Compute unit numbers below are estimates based on operation complexity.
> Real CU measurements from LEZ testnet require a running sequencer with CU metering.
> LEZ's per-transaction compute budget may change during testnet.

## `emit_event()` Cost

The cost of `emit_event()` is dominated by Borsh serialization of the payload.

| Payload Size | Estimated CU per event | Notes |
|---|---|---|
| 64 bytes | ~800 ns wall time | Tiny event (e.g. status code only) |
| 256 bytes | ~1.2 µs wall time | Small event (3-4 fields) |
| 512 bytes | ~1.8 µs wall time | Medium event |
| 1,024 bytes (max) | ~2.5 µs wall time | Maximum payload size |

## `drain_events()` Cost

| Event Count | Estimated Time |
|---|---|
| 1 event | ~50 ns |
| 10 events | ~100 ns |
| 64 events (max) | ~300 ns |

## Per-Transaction Overhead Summary

| Events | Total Payload | Approx Overhead |
|---|---|---|
| 1 event, 64B payload | 64 bytes | Negligible |
| 10 events, 64B each | 640 bytes | < 10 µs |
| 64 events, 1024B each | 65,536 bytes | < 200 µs |

## Memory Usage

- Thread-local buffer overhead: ~56 bytes per event (metadata) + payload bytes
- Maximum buffer size: 64 events × (56 + 1024) bytes ≈ 69 KB

## Notes on LEZ CU Metering

The `emit_event()` function itself does not call any Risc0 system calls — it only:
1. Serializes the event with Borsh (CPU-bound)
2. Appends to a thread-local `Vec<EventRecord>` (heap allocation)

The only Risc0 CU cost occurs at `drain_events()` + `ProgramOutput::write()`, which calls `env::commit()` to seal the Risc0 journal. This cost scales with the total byte count of all events serialized into `ProgramOutput`.

Real CU numbers will be added once programs are deployed to LEZ testnet with CU tracking enabled.
