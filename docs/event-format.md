# Event Format Specification (LP-0012)

## Overview

LEZ programs can emit structured events to help developers understand execution state and failures. Events survive transaction failures (panics), providing critical debugging information.

---

## EventRecord Structure

### Definition

```rust
// lez-events/src/event.rs

use borsh::{BorshSerialize, BorshDeserialize};

pub const MAX_EVENT_PAYLOAD_BYTES: usize = 1024;
pub const MAX_EVENTS_PER_TX: u32 = 64;

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub struct EventRecord {
    /// Program ID of the program that emitted this event.
    /// **ENFORCED BY RUNTIME:** Program cannot set this to arbitrary value.
    /// Sequencer overwrites before storage.
    pub program_id: [u8; 32],

    /// Zero-indexed position in the transaction's event sequence.
    /// Starts at 0, increments by 1 per emit_event() call.
    /// Guarantees event ordering even if stored out-of-order.
    pub sequence: u32,

    /// Event type identifier, defined by program author.
    /// Programs define constants like `const TRANSFER_EVENT: u64 = 0x0001;`
    /// Allows decoder to locate schema and deserialize payload.
    pub discriminant: u64,

    /// Schema version for forward compatibility.
    /// v1 for initial release.
    /// Programs can emit v1 events today; if v2 format is introduced,
    /// decoders can dispatch based on this field.
    pub schema_version: u8,

    /// Borsh-encoded event fields.
    /// Arbitrary bytes, up to MAX_EVENT_PAYLOAD_BYTES.
    /// Decoder uses (program_id, discriminant, schema_version) to locate schema,
    /// then borsh::from_slice(&payload) to deserialize.
    pub payload: Vec<u8>,
}

impl EventRecord {
    pub fn size_bytes(&self) -> usize {
        // 32 (program_id) + 4 (sequence) + 8 (discriminant) + 1 (schema_version)
        // + 4 (Vec length) + payload.len()
        32 + 4 + 8 + 1 + 4 + self.payload.len()
    }
}
```

---

## Field Details

### `program_id: [u8; 32]`
- **Type:** Fixed 32-byte array (256-bit hash)
- **Enforced:** Sequencer writes this field after proof verification
- **Why enforced:** Prevents programs from spoofing events from other programs
- **Behavior:** If a program tries to set this in code, sequencer overwrites it with the actual running program ID
- **Display:** Hex string, e.g., `0xdead_beef_...` (usually shown with leading 0x and underscores for readability)

### `sequence: u32`
- **Type:** 32-bit unsigned integer
- **Semantics:** Zero-indexed counter per transaction
- **Guarantee:** Within a single transaction, sequence strictly 0, 1, 2, 3, ... with no gaps
- **Purpose:** Allows clients to verify all events were received and detect missing events
- **Maximum:** 64 events per transaction (enforced), so max sequence = 63

### `discriminant: u64`
- **Type:** 64-bit unsigned integer
- **Semantics:** Event type tag, chosen by program author
- **Convention:** Use namespace or constants
  ```rust
  // Program author defines:
  pub const TRANSFER_INITIATED: u64 = 0x0001;
  pub const TRANSFER_COMPLETED: u64 = 0x0002;
  pub const TRANSFER_FAILED: u64 = 0x0003;
  ```
- **Decoder uses this to:** Look up schema, deserialize payload, display event name
- **Unknown discriminants:** Decoder shows raw hex (`Unknown(0x0042)`) and raw payload hex as fallback

### `schema_version: u8`
- **Type:** 8-bit unsigned integer
- **Current value:** 1 (must be 1 for all v1 events)
- **Forward compatibility strategy:**
  - If schema changes in future (e.g., new field added), emit events with `schema_version = 2`
  - Decoder can handle both v1 and v2 based on this field
  - v1 decoders will see v2 events and show raw hex (graceful degradation)
- **No breaking changes:** Events with different schema versions can coexist in same transaction

### `payload: Vec<u8>`
- **Type:** Variable-length byte vector
- **Encoding:** Borsh (deterministic serialization)
- **Size limits:**
  - Per-event: ≤ 1024 bytes
  - If exceeded: `emit_event()` returns `EventError::PayloadTooLarge`
  - Error is deterministic, never panics
- **Decoder:** Uses borsh::from_slice(&payload) with schema-defined field types
- **Safety:** If payload cannot be decoded, show raw hex as fallback

---

## Encoding Format (Borsh)

### Why Borsh?

1. **Deterministic:** Same input always produces identical bytes (no map iteration order issues)
2. **Compact:** Smaller than JSON, suitable for zkVM constraints
3. **Already available:** LEZ workspace depends on `borsh = "1.5.7"`
4. **no_std compatible:** Works inside zkVM guest (no allocator overhead)
5. **Speed:** Faster than serde_json for serialization

### Encoding Example

Program emits:
```rust
#[derive(BorshSerialize)]
pub struct TransferCompleted {
    pub from: [u8; 32],
    pub to: [u8; 32],
    pub amount: u64,
}

emit_event(my_program_id, TransferCompleted {
    from: [0xAA; 32],
    to:   [0xBB; 32],
    amount: 1000,
}).unwrap();
```

Resulting EventRecord:
```
EventRecord {
    program_id: [0xCC; 32],  // Set by sequencer
    sequence: 0,
    discriminant: 0x0002,    // Define in program
    schema_version: 1,
    payload: borsh::to_vec(&TransferCompleted { ... }).unwrap(),
        // Hex: AA (32×) BB (32×) E8 03 00 00 00 00 00 00
        // = from[32] + to[32] + amount_u64_le(1000)
}
```

When Borsh-encoded into EventRecord:
```
CC (32×)           # program_id
00 00 00 00        # sequence = 0 (little-endian u32)
02 00 00 00 00 00 00 00  # discriminant = 0x0002 (little-endian u64)
01                 # schema_version = 1
70 00 00 00        # payload length = 112 bytes (little-endian u32)
AA (32×) BB (32×) E8 03 00 00 00 00 00 00  # payload bytes
```

**Total: 49 bytes header + 112 bytes payload = 161 bytes per EventRecord in this case**

---

## Ordering Guarantee

### Sequence Semantics

Within a single transaction:
- First `emit_event()` → `sequence = 0`
- Second `emit_event()` → `sequence = 1`
- Third `emit_event()` → `sequence = 2`
- ... up to 63 (`MAX_EVENTS_PER_TX - 1`)

### Immutability

Once assigned, sequence never changes. Events are returned from sequencer in sequence order (or client must sort).

### Use Cases

```rust
// Verify receipt completeness
let events = receipt.events;
for (i, event) in events.iter().enumerate() {
    assert_eq!(event.sequence as usize, i, "Missing event");
}

// Detect duplicates
let seen_sequences: HashSet<u32> = events.iter().map(|e| e.sequence).collect();
assert_eq!(seen_sequences.len(), events.len(), "Duplicate sequence numbers");
```

---

## Schema Versioning Strategy

### Version 1 (Current)

All events use `schema_version = 1`.

Fields are fixed:
- program_id
- sequence
- discriminant
- schema_version (always 1)
- payload (Borsh-encoded with app-defined fields)

### Hypothetical Version 2 (Future)

If new requirements emerge, programs can emit `schema_version = 2` events.

```rust
#[derive(BorshSerialize)]
pub struct TransferCompletedV2 {
    pub from: [u8; 32],
    pub to: [u8; 32],
    pub amount: u64,
    pub tx_fee: u64,  // New field in v2
}

// Program decides whether to emit v1 or v2
if new_schema_enabled {
    emit_event_with_version(program_id, TransferCompletedV2 { ... }, schema_version=2)?;
} else {
    emit_event_with_version(program_id, TransferCompletedV1 { ... }, schema_version=1)?;
}
```

Decoders handle both:
```rust
match event.schema_version {
    1 => decode_v1_schema(event),
    2 => decode_v2_schema(event),
    _ => show_raw_hex(event),
}
```

### Guarantee

Schema version never resets. If v2 is introduced, it will use a different `schema_version` field value, allowing safe coexistence.

---

## Program Attribution

### Enforcement

When sequencer receives program output from Risc0 proof:

1. Extract `ProgramOutput` from journal
2. Extract `.events` vector
3. **Overwrite all `program_id` fields in all events to match the actual running program ID**
4. Store in receipt

```rust
// sequencer/core/src/lib.rs (pseudo-code)
let mut output: ProgramOutput = borsh::from_slice(&journal_bytes)?;
for event in &mut output.events {
    event.program_id = running_program_id;  // ENFORCED
}
store_receipt_in_block(output.events);
```

### Why This Matters

- Program cannot spoof events from another program
- All events in a receipt are guaranteed from that program
- Proof verification ensures integrity (Risc0 zkVM proof)

### Test

```rust
#[test]
fn test_program_cannot_spoof_program_id() {
    // Deploy program that tries:
    // emit_event([0xFF; 32], SomeEvent { ... })
    
    // After execution, receipt.events[0].program_id == actual_program_id
    // Not [0xFF; 32]
}
```

---

## Size Limits

### Per-Event Payload

```
MAX_EVENT_PAYLOAD_BYTES = 1024
```

If payload serializes to > 1024 bytes:
```rust
emit_event(program_id, LargeEvent { ... })
// Returns: Err(EventError::PayloadTooLarge { limit: 1024, actual: 2048 })
// Does NOT panic, does NOT truncate
```

### Per-Transaction Event Count

```
MAX_EVENTS_PER_TX = 64
```

If emit_event() is called 65 times:
```rust
for i in 0..65 {
    let result = emit_event(program_id, MyEvent { val: i });
    if i < 64 {
        assert!(result.is_ok());
    } else {
        assert_eq!(result, Err(EventError::TooManyEvents { limit: 64, actual: 65 }));
    }
}
```

### Total Per-Transaction

Worst case:
- 64 events × (49 bytes header + 1024 bytes payload) = 68,672 bytes
- ~68 KB per transaction
- Acceptable within zkVM guest memory (~40 MB per tx)

### Error Handling

All size violations return `EventError`, never panic:

```rust
pub enum EventError {
    #[error("Event payload size {actual} bytes exceeds limit of {limit} bytes")]
    PayloadTooLarge { limit: usize, actual: usize },

    #[error("Transaction event count {actual} exceeds limit of {limit}")]
    TooManyEvents { limit: u32, actual: u32 },

    #[error("Total event bytes {actual} exceeds per-transaction limit of {limit}")]
    TotalSizeTooLarge { limit: usize, actual: usize },

    #[error("Failed to encode event: {0}")]
    EncodingFailed(String),
}

impl EventError {
    pub const fn error_code(&self) -> u32 {
        match self {
            Self::PayloadTooLarge { .. }   => 0xEE01,
            Self::TooManyEvents { .. }     => 0xEE02,
            Self::TotalSizeTooLarge { .. } => 0xEE03,
            Self::EncodingFailed(_)        => 0xEE04,
        }
    }
}
```

---

## Privacy Considerations

### Events Are Public

Events are stored on-chain. **Never emit:**
- Private keys or seed phrases
- Secret authentication tokens
- Hidden account balances
- Viewing keys or access credentials
- Any data that should not be globally visible

### Safe to Emit

- Public amounts (for executed transfers)
- Account addresses (public identities)
- Operation names and status codes
- Event discriminants (program-defined type IDs)
- Timestamp and sequence information
- Public error messages and failure reasons

### For Private (ZKP) Execution

In private execution mode, the program runs locally (not on chain). Still:
- Events are commitment outputs (included in proof)
- After proof verification, events are public
- So same privacy rules apply
- Program author responsible for not leaking secrets

### Guidance

Program authors should:
1. Review event payloads before deployment
2. Test with `lez-event-cli decode` to verify output
3. Document what each event means and why it's safe to emit
4. Consider splitting events into success/failure variants if needed

---

## Example: Token Transfer Program Events

```rust
// Define discriminants
pub const EVENT_TRANSFER_INITIATED: u64 = 0x0001;
pub const EVENT_TRANSFER_COMPLETED: u64 = 0x0002;
pub const EVENT_INSUFFICIENT_BALANCE: u64 = 0x0003;

#[derive(BorshSerialize)]
pub struct TransferInitiated {
    pub from: [u8; 32],
    pub to: [u8; 32],
    pub amount: u64,
}
impl_lez_event!(TransferInitiated, discriminant = EVENT_TRANSFER_INITIATED);

#[derive(BorshSerialize)]
pub struct TransferCompleted {
    pub from: [u8; 32],
    pub to: [u8; 32],
    pub amount: u64,
    pub from_balance_after: u64,
}
impl_lez_event!(TransferCompleted, discriminant = EVENT_TRANSFER_COMPLETED);

#[derive(BorshSerialize)]
pub struct InsufficientBalance {
    pub account: [u8; 32],
    pub requested: u64,
    pub available: u64,
}
impl_lez_event!(InsufficientBalance, discriminant = EVENT_INSUFFICIENT_BALANCE);

// In program:
fn main() {
    let program_id = [0xAA; 32];
    let from = [0x01; 32];
    let to = [0x02; 32];
    let amount = 100;
    let balance = 50;

    execute_program(|| {
        if balance < amount {
            emit_event(program_id, InsufficientFunds {
                account: from,
                requested: amount,
                available: balance,
            }).expect("emit insufficient balance");

            panic!("Insufficient balance");  // Adapter catches this and seals journal
        }

        // Apply transfer
        let new_balance = balance - amount;
        emit_event(program_id, TransferCompleted {
            from, to, amount,
            from_balance_after: new_balance,
        }).expect("emit transfer completed");
        
        // Returns successfully, adapter seals journal normally
        vec![] // Return post states
    });
}
```

When decoded:
```
[seq=0] TransferInitiated  (program: 0xabcd...1234)
  from:   0x1111...1111
  to:     0x2222...2222
  amount: 1000

[seq=1] TransferCompleted  (program: 0xabcd...1234)
  from:   0x1111...1111
  to:     0x2222...2222
  amount: 1000
  from_balance_after: 500
```

On failure:
```
[seq=0] TransferInitiated  [FAILED TX]  (program: 0xabcd...1234)
  from:   0x1111...1111
  to:     0x2222...2222
  amount: 999999999

[seq=1] InsufficientBalance  [FAILED TX]  (program: 0xabcd...1234)
  account:   0x1111...1111
  requested: 999999999
  available: 500
```

---

## Stability Guarantees

- Event struct fields are stable (never removed)
- Error codes (0xEE01–0xEE04) are permanent
- Size limits (1024, 64) are documented as minimums (may increase but never decrease)
- Sequence ordering is guaranteed by runtime
- Borsh encoding is canonical (no changes to encoder)

---

**Status:** ✓ Event format frozen for v1 implementation.
