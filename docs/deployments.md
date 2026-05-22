# Deployed Programs on LEZ Testnet

> **Note**: This file will be updated once programs are deployed to LEZ testnet.
> The programs are ready for deployment — see the Quick Start in README.md.

## Environment

- Network: LEZ testnet (standalone mode)
- RISC0_DEV_MODE: 0 (real proving)
- Date: TBD (pending testnet deployment)

## Programs

### token-transfer

- **Description**: Demonstrates success-path events. Emits `TransferInitiated` and `TransferCompleted`.
- **Program ID**: `<PENDING DEPLOYMENT>`
- **Deployed**: TBD
- **Binary**: `target/release/token-transfer-example`

### withdraw

- **Description**: Demonstrates failure-path events. Emits `WithdrawAttempted` and (on failure) `InsufficientFunds` **before** panicking.
- **Program ID**: `<PENDING DEPLOYMENT>`
- **Deployed**: TBD
- **Binary**: `target/release/withdraw-example`

## RPC Endpoints

- **Sequencer RPC**: `http://localhost:8080` (local standalone mode)
- **Testnet RPC**: TBD

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
