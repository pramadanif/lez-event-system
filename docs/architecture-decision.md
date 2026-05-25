# Phase 1.2: Storage Mechanism Decision

## Options Evaluated

### Option A: Extend Risc0 Journal ✓ CHOSEN
Extend `ProgramOutput` struct to include `events: Vec<EventRecord>` field.

**Mechanism:**
1. Program calls `emit_event()` → thread-local buffer
2. Before calling `write_nssa_outputs()`, program calls `execute_program()`
3. Events passed to output writer
4. `ProgramOutput::write()` commits entire struct (with events) to journal
5. Sequencer deserializes journal, extracts events
6. Events included in receipt regardless of state diff success/failure

**Pros:**
- Minimal changes to LEZ runtime (just extend struct)
- Events are part of cryptographic proof (immutable)
- Works with existing Risc0 verification flow
- Natural integration: events = part of program output
- Already in Journal when sequencer receives it

**Cons:**
- Requires serializing events → small proof overhead (~0.1-1% for typical txs)
- Events committed AFTER main logic, so panic still means events lost
  - **Solution:** Defensive write pattern (emit before potential failure point)

**Failure Path Handling:**
```rust
// Program structure:
emit_event(program_id, Event1 { ... }).unwrap();  // Always runs

if can_fail_here() {
    emit_event(program_id, FailureEvent { ... }).unwrap();
    let events = execute_program();
    write_nssa_outputs_with_events(..., events);  // Write now, before panic
    panic!("reason");  // Events already in journal
}

emit_event(program_id, SuccessEvent { ... }).unwrap();
let events = execute_program();
write_nssa_outputs_with_events(..., events);  // Normal success path
```

Events are committed to journal before panic signal. When Risc0 zkVM panics, the journal is already finalized and can be read.

---

### Option B: Separate Output Channel (rejected)
Use a separate Risc0 output slot reserved for events.

**Cons:**
- Requires changes to Risc0 guest environment setup
- Events not part of main proof (external channel)
- Harder to verify integrity
- LEZ would need new mechanisms to route this channel

**Decision:** Rejected. Option A is simpler and proven.

---

### Option C: Assumption-Based Output (rejected)
Use Risc0 assumptions to write events to external output.

**Cons:**
- Very complex, requires deep Risc0 integration
- Proof verification must check assumptions
- Breaks LEZ assumption model
- Overkill for this use case

**Decision:** Rejected.

---

## Final Architecture

### Struct Extension
```rust
// nssa/core/src/program.rs
pub struct ProgramOutput {
    // ... existing fields ...
    pub events: Vec<EventRecord>,  // NEW
}
```

### Encoding
- Borsh serialization (existing in LEZ)
- Events included in journal commit
- Deterministic byte order (same input → same bytes)

### Sequencer Integration
1. Deserialize `ProgramOutput` from journal
2. Extract `.events` field
3. Include in `TxReceipt` even on failure
4. Never lose events (they're in proof)

### Failure Path
- Program must emit events BEFORE panic
- Use `execute_program()` to flush before potential failure point
- Defensively write output before any panic
- Sequencer extracts events from journal regardless of state diff result

### Size Constraints
- Max 1024 bytes per event payload
- Max 64 events per transaction
- Total per-tx overhead: ~65KB worst case (acceptable for zkVM)

---

## Risk Analysis

**Risk: Events lost on panic**
- Mitigation: Require emit-before-fail pattern, document in SDK
- Test: `test_failure_path` integration test

**Risk: Journal space**
- Worst case: 64 × (16 bytes overhead + 1024 bytes payload) = ~66KB per tx
- Acceptable: zkVM guest has ~40MB available per tx

**Risk: Determinism**
- Borsh is deterministic by design
- All inputs to events (program_id, discriminant, payload) are controlled
- No floating point or randomness in event encoding

---

## Validation

This decision satisfies all LP-0012 requirements:
- ✓ Events survive panic (via defensive write pattern)
- ✓ Deterministic encoding (Borsh)
- ✓ Schema versioning (u8 field)
- ✓ Program attribution (program_id enforced by sequencer)
- ✓ Size limits (1024 + 64)
- ✓ RPC access (new endpoint)
- ✓ Privacy safe (events are public, program choice what to emit)

---

**Status:** ✓ Approved for Phase 1.3 (EventRecord struct design)
