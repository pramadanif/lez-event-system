# Phase 1.1: LEZ Codebase Research Notes

Date: May 22, 2026  
Based on: logos-execution-zone repository

## Key Findings

### Q1: Type signature of `write_nssa_outputs`?
**A:** No function called `write_nssa_outputs`. Instead: `ProgramOutput::write()` method.

```rust
// nssa/core/src/program.rs
impl ProgramOutput {
    pub fn write(self) {
        env::commit(&self);  // Writes to Risc0 journal
    }
}
```

### Q2: Fields of `ProgramOutput`?
**A:** Located in `nssa/core/src/program.rs` (~L422):

```rust
pub struct ProgramOutput {
    pub self_program_id: ProgramId,
    pub caller_program_id: Option<ProgramId>,
    pub instruction_data: InstructionData,
    pub pre_states: Vec<AccountWithMetadata>,
    pub post_states: Vec<AccountPostState>,        // State changes
    pub chained_calls: Vec<ChainedCall>,           // Calls to other programs
    pub block_validity_window: BlockValidityWindow,
    pub timestamp_validity_window: TimestampValidityWindow,
}
```

**KEY:** No `events` field currently. Must add.

### Q3: How does sequencer receive program output?
**A:** Via Risc0 journal read-back:

1. Program runs in zkVM guest, calls `ProgramOutput.write()` → `env::commit(&self)` to journal
2. Sequencer receives Risc0 receipt + proof
3. Extracts journal data (borsh-deserialize into `ProgramOutput`)
4. Applies state diff if valid

Path: `sequencer/core/src/lib.rs` → `transition_from_public_transaction()` → `ValidatedStateDiff::from_public_transaction()`

### Q4: Does program use `env::commit_slice()` for output?
**A:** Close, but not exactly. Uses `env::commit(&self)` where `self: ProgramOutput` is Borsh-serialized internally.

```rust
impl Serialize for ProgramOutput {
    // borsh serialization
}
// Then: env::commit(&self) serializes via Borsh
```

This is Risc0 zkVM pattern: `env::commit()` takes `Serialize` impl.

### Q5: Panicking program output?
**A:** **CRITICAL FOR FAILURE PATH:**

When a Risc0 guest panics:
- Proof generation **fails**
- Journal is **empty** (never committed)
- Sequencer receives `Err` from proof verification
- Transaction is silently **skipped** from block

**Implication:** Must write events to journal BEFORE any potential panic, or use catch-panic wrapper.

### Q6: How does sequencer distinguish success vs failure?
**A:** Not via receipts. Implicit model:

- **Success:** Transaction included in block → all state changes applied
- **Failure:** Transaction NOT in block → silently skipped from mempool
- Log written: `error!("Transaction {tx_hash} failed execution check with error: {err}")` but no structured receipt

**Problem:** No way to query "what failed" after rejection.

### Q7: Per-tx metadata storage?
**A:** **NO.**

Currently:
- Only successful state changes are stored
- Failed transactions produce only log output to stderr
- No structured error receipts persisted
- No event log stored anywhere

**This is what LP-0012 adds.**

### Q8: RPC endpoints for tx receipts?
**A:** Available endpoints (`sequencer/service/rpc/src/lib.rs`):

| Endpoint | Returns |
|----------|---------|
| `sendTransaction(tx)` | `tx_hash` |
| `getTransaction(tx_hash)` | `Option<NSSATransaction>` or error if not found |
| `getBlock(block_id)` | Full `Block` struct including all txs |
| `getBlockRange(start, end)` | Vec of blocks |

**NO dedicated receipt/status endpoint.** To check if tx failed:
1. Call `sendTransaction()` → get hash
2. Call `getTransaction(hash)` → `None` means not in any block (failed)

### Q9: Integration test pattern?
**A:** (`integration_tests/tests/token.rs`)

1. Create `TestContext` (local sequencer + wallet)
2. Submit tx via `wallet::cli::execute_subcommand(...Command::Token(...))`
3. Sleep to wait for block creation: `tokio::time::sleep(Duration::from_secs(TIME_TO_WAIT_FOR_BLOCK_SECONDS))`
4. Query result: `ctx.sequencer_client().get_account(account_id).await?`
5. Parse account data to verify state change

No receipt object. Only post-hoc state verification.

---

## Architecture Implications

### Current State Model
```
Program
  ↓ write()
Risc0 Journal (ProgramOutput)
  ↓ proof verification
Sequencer receives output
  ↓ apply ValidatedStateDiff
Block (success) OR skip (failure)
```

### Required Extension for LP-0012
```
Program
  ↓ emit_event() → thread-local buffer
  ↓ execute_program() before write()
Risc0 Journal (ProgramOutput + events)
  ↓ proof verification
Sequencer extracts events BEFORE revert decision
  ↓ apply ValidatedStateDiff
Block + TxReceipt { success, events } (both success AND failure)
  ↓ new RPC endpoint
Client queries receipt including events
```

---

## Failure Path Strategy Decision

**Problem:** If program panics inside Risc0, journal is empty → events lost.

**Solution:** Use **defensive write pattern**:
```rust
// Inside program
emit_event(program_id, AttemptedEvent { ... }).unwrap();
if balance < amount {
    emit_event(program_id, InsufficientFundsEvent { ... }).unwrap();
    
    // CRITICAL: use execute_program wrapper to catch panic
    execute_program(|| {
        emit_event(program_id, InsufficientFundsEvent { ... }).unwrap();
        panic!("Insufficient funds");  // Adapter catches this, seals journal, resumes panic
    });
}
```

Journal already has events committed before panic signal. Sequencer can extract them.

**Alternative (rejected):** Risc0 assumption-based output → too complex, requires changes to LEZ proof verification.

---

## Storage Format Decision

- **Encoding:** Borsh (already in LEZ deps, deterministic, `no_std` compatible)
- **Per-event:** Include discriminant (type ID), sequence (ordering), schema_version
- **Per-tx:** Collect in `Vec<EventRecord>` field in extended `ProgramOutput`
- **Size limits:** 1024 bytes per event payload, max 64 events per tx (tuned for zkVM guest constraints)

---

## Next Steps

1. ✓ Research complete → Phase 1.2: Choose storage mechanism
2. Design `EventRecord` struct
3. Document event format
4. Implement `lez-events` SDK
