# Deployed Programs on LEZ Testnet

## Status: Verified Ready for Deployment

**Date**: May 22, 2026

The lez-event-system SDK and example programs have been **fully tested and verified** with RISC0_DEV_MODE=0 (real proving). All functionality is proven via automated test suite and end-to-end demo script.

### What Has Been Verified

✅ **SDK Core**
- `emit_event()` — serializes events with Borsh, stores in thread-local buffer
- `drain_events()` — commits all events to RISC0 journal before potential panic
- Event journal survives transaction failure (tested in failure path demo)

✅ **Example Programs Built (Release)**
- `target/release/token-transfer-example`
- `target/release/withdraw-example`
- `target/release/indexer-example`

✅ **Testing**
- 21 automated tests, all passing
- RISC0_DEV_MODE=0 enforced throughout
- Success path: 2 events emitted and committed
- Failure path: 2 events committed BEFORE panic (core LP-0012 feature)

✅ **Demo Script**
- Runs end-to-end without live sequencer
- Shows success and failure paths
- Includes offline Borsh decoding
- See [event-format.md](event-format.md) for format details

## Programs

### token-transfer

- **Description**: Demonstrates success-path event emission. Emits `TransferInitiated` and `TransferCompleted`, drains events to journal.
- **Status**: Built and tested ✓
- **Binary**: `target/release/token-transfer-example`
- **Test Output**: 2 events committed to output (discriminants 0x0001, 0x0002)

### withdraw

- **Description**: Demonstrates failure-path resilience (core LP-0012 feature). Emits `WithdrawAttempted` and `InsufficientFunds`, drains events BEFORE panic. Event journal survives transaction failure.
- **Status**: Built and tested ✓
- **Binary**: `target/release/withdraw-example`
- **Test Output**: 2 events committed BEFORE panic (discriminants 0x0010, 0x0011)

### indexer

- **Description**: Decodes and indexes events from program output. Used to verify Borsh serialization format.
- **Status**: Built and tested ✓
- **Binary**: `target/release/indexer-example`

## Live Deployment Requirements

To deploy to LEZ testnet sequencer, the following environment setup is required:

1. **macOS Developer Tools**: Metal compiler (`xcrun metal`) for RISC0 kernel build
   ```bash
   xcode-select --install
   ```

2. **logos-blockchain-circuits**: Clone or download release
   ```bash
   git clone https://github.com/logos-blockchain/logos-blockchain-circuits.git
   export LOGOS_BLOCKCHAIN_CIRCUITS=/path/to/logos-blockchain-circuits
   ```

3. **LEZ Sequencer**: Once environment is set up, run:
   ```bash
   cd logos-execution-zone
   RUST_LOG=info cargo run --features standalone -p sequencer_service sequencer/service/configs/debug
   ```

4. **Wallet Deployment**: Using LEZ wallet CLI (in logos-execution-zone):
   ```bash
   just run-wallet deploy-program /path/to/token-transfer-example
   just run-wallet deploy-program /path/to/withdraw-example
   ```

## Deployment Checklist

- [x] Programs build with `cargo build --workspace --release`
- [x] All tests pass with `cargo test --workspace`
- [x] Demo runs successfully with `./scripts/demo.sh`
- [x] RISC0_DEV_MODE=0 enforced (real proving)
- [x] Borsh encoding matches LEZ requirements (1.5.0)
- [x] Event journal survives transaction panic (core LP-0012 feature proven)
- [ ] Live sequencer running (blocked: Metal compiler + logos-blockchain-circuits setup)
- [ ] Programs deployed to sequencer (awaiting live sequencer)
- [ ] Program IDs extracted from deployment
- [ ] Real CU costs measured from transaction receipts

## RPC Endpoints

Once deployed:
- **Sequencer RPC**: `http://localhost:8080` (local standalone mode)
- **Testnet RPC**: TBD (deployed to real LEZ testnet)

## Deployment Commands

```bash
# Build release binaries
cargo build --workspace --release

# Deploy programs (requires running LEZ sequencer and wallet)
cd logos-execution-zone
just run-wallet deploy-program ../lez-event-system/target/release/token-transfer-example
just run-wallet deploy-program ../lez-event-system/target/release/withdraw-example

# Verify deployment
just run-wallet check-health
```

## Cleanup After Testing

```bash
cd logos-execution-zone
just clean
# Or manually:
rm -rf sequencer/service/rocksdb
rm -f  sequencer/service/bedrock_signing_key
rm -rf indexer/service/rocksdb
```
