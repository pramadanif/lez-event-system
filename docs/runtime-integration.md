# Non-Invasive Runtime Integration

The LEZ Event System uses a **Deterministic Framed Event Transport**. This means the upstream `ProgramOutput` and `TxReceipt` structs do *not* need to be rewritten immediately. Instead, the runtime adapter injects a cleanly separated event frame at the beginning of the RISC0 execution journal.

This document outlines the minimal integration steps required in the `logos-execution-zone` sequencer.

## Integration Diff

The host sequencer simply needs to extract the event frame before parsing `ProgramOutput`.

```diff
  // inside the sequencer where the journal is parsed
  
  let journal_bytes = receipt.journal.bytes.as_slice();
  
+ // 1. Extract the deterministic event frame (if any)
+ let (events, remaining_bytes) = lez_events_runtime::parse_journal(journal_bytes)
+     .expect("Failed to parse event envelope");
+
- // 2. Deserialize ProgramOutput from the journal
- let program_output: ProgramOutput = risc0_zkvm::serde::from_slice(journal_bytes)?;
+ // 2. Deserialize ProgramOutput from the remaining bytes
+ let program_output: ProgramOutput = risc0_zkvm::serde::from_slice(remaining_bytes)?;

+ // 3. Attach events to your block/receipt storage as needed
+ save_events_to_receipt(tx_hash, events);
```

## Why this Architecture Wins

1. **Zero Impact on Proof Verification**: The events are part of the journal. The zkVM guest simply executes `env::commit_slice` before executing the standard `env::commit`. The guest code is proven, and the host can deterministically reconstruct the state.
2. **Failure-Path Resilience**: If the guest panics (e.g., `panic!("Insufficient funds")`), the `lez-events-runtime` wrapper automatically catches it, flushes the event frame to the journal, and allows the panic to proceed. The host receives the events even if `success=false`.
3. **No Struct Migrations Required**: Since we don't modify `ProgramOutput`, there's no need to coordinate a massive breaking change across the entire Logos stack.
4. **Deterministic and Versioned**: The `LEZE` magic bytes and wire version ensure that future upgrades to the event format won't break the parser.
