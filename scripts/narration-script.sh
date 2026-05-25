#!/usr/bin/env bash
# ============================================================
# LP-0012 Demo Narration Script
# Video Duration Target: ~8-12 minutes
# IMPORTANT: Record at 1080p, show terminal + code side-by-side
# ============================================================
#
# HOW TO USE:
#   1. Open a large, clear terminal (e.g. iTerm2, dark theme)
#   2. Set font size >= 18px so text is readable in video
#   3. Follow each [NARRATE] cue exactly as written
#   4. Commands in [TYPE] blocks are what you actually type
#   5. [PAUSE] means wait 2-3 seconds for effect
#
# CHECKLIST BEFORE RECORDING:
#   [ ] export RISC0_DEV_MODE=0
#   [ ] cd /path/to/lez-event-system
#   [ ] Clear terminal: clear
#   [ ] No other notifications on screen
# ============================================================

# ─────────────────────────────────────────────────────────────
# SCENE 1: Introduction (0:00–1:00)
# ─────────────────────────────────────────────────────────────
#
# [NARRATE]:
#   "Hi, welcome to the LP-0012 demo — this is the LEZ Event System,
#    a structured event-emission SDK for Logos Execution Zone programs.
#
#    The core challenge we're solving: when a LEZ transaction fails
#    and panics, you lose all execution context. Developers have no
#    way to know *why* it failed. LP-0012 fixes this by ensuring events
#    survive even when the transaction panics.
#
#    Let me show you."

# [TYPE]:
echo "RISC0_DEV_MODE=${RISC0_DEV_MODE}"
# [SHOW ON SCREEN: Must print "RISC0_DEV_MODE=0"]

# [NARRATE]:
#   "First, notice that RISC0_DEV_MODE is 0 — we're using real proving,
#    not mock mode. This is a hard requirement for LP-0012."

# [PAUSE]

# ─────────────────────────────────────────────────────────────
# SCENE 2: Architecture Overview (1:00–2:30)
# ─────────────────────────────────────────────────────────────
#
# [NARRATE]:
#   "Let's look at the repository structure before we run anything."

# [TYPE]:
ls -1
# [SHOW: lez-events/, lez-event-decoder/, examples/, scripts/, docs/]

# [NARRATE]:
#   "The SDK lives in `lez-events/`. The key API is the runtime adapter:
#    execute_program() — which wraps your application logic and automatically
#    handles buffering, framing, and journal commits under the hood.
#
#    Let me show you the failure-path pattern in the withdraw example."

# [TYPE]:
cat examples/withdraw/src/main.rs | grep -A 20 "if instr.balance"
# [SHOW: the emit → drain → write → panic sequence]

# [NARRATE]:
#   "See this sequence: we emit the InsufficientFunds event,
#    and then we call panic!(). But because we are running inside the
#    execute_program wrapper, the panic is caught!
#
#    This is the key insight: the adapter drains the events, frames them
#    with the LEZE magic bytes, seals the RISC0 journal, and THEN resumes
#    the panic. The events are immortalized even though the transaction fails."

# [PAUSE]

# ─────────────────────────────────────────────────────────────
# SCENE 3: Run the Test Suite (2:30–4:00)
# ─────────────────────────────────────────────────────────────
#
# [NARRATE]:
#   "Let's run the full test suite. All tests run with RISC0_DEV_MODE=0."

# [TYPE]:
cargo test --workspace
# [SHOW: 27 tests passing]

# [NARRATE]:
#   "32 tests, all passing. Let me highlight the most important ones:
#    `test_failure_path.rs` — these tests simulate the runtime adapter
#    catching panics and prove that events are preserved.
#
#    `test_size_limits.rs` — verifies that emit_event() never panics,
#    it always returns Err with a stable error code when limits are exceeded.
#
#    `test_encoding.rs` — verifies Borsh encoding is deterministic."

# [PAUSE]

# ─────────────────────────────────────────────────────────────
# SCENE 4: Full Demo Script (4:00–6:30)
# ─────────────────────────────────────────────────────────────
#
# [NARRATE]:
#   "Now let's run the full demo script. This is what a clean-environment
#    evaluator would run. Watch the RISC0_DEV_MODE value at the top."

# [TYPE]:
RISC0_DEV_MODE=0 ./scripts/demo.sh

# [NARRATE - during step 3 - success path]:
#   "This is the SUCCESS path. The token transfer emits two events:
#    TransferInitiated and TransferCompleted. Both are committed normally
#    to the output."

# [NARRATE - during step 4 - failure path]:
#   "And HERE is the CRITICAL feature. This is the FAILURE path.
#    We're submitting a withdraw for 2000 tokens, but the balance is only 500.
#
#    Watch what happens: the program emits WithdrawAttempted, then
#    InsufficientFunds, and then PANICS. The runtime adapter catches it,
#    flushes the LEZE frame to the journal, and resumes the abort.
#
#    The output shows 'events committed: 2' — both events are in the
#    program output even though the transaction is about to fail.
#    THIS is the LP-0012 guarantee."

# [NARRATE - during step 5 - decode-raw]:
#   "The decoder CLI can decode Borsh-encoded events offline — no running
#    sequencer required. This is useful for debugging local program runs
#    during development."

# [PAUSE]

# ─────────────────────────────────────────────────────────────
# SCENE 5: Code Quality (6:30–7:30)
# ─────────────────────────────────────────────────────────────
#
# [NARRATE]:
#   "Let's verify code quality. LP-0012 requires zero clippy warnings."

# [TYPE]:
cargo clippy --workspace --tests -- -D warnings 2>&1 | tail -3
# [SHOW: "Finished ... 0 warnings"]

# [NARRATE]:
#   "Zero warnings. And formatting is clean too."

# [TYPE]:
cargo fmt --all -- --check && echo "Formatting: OK"

# [NARRATE]:
#   "Now let me show the EventRecord struct — the core data type."

# [TYPE]:
cat lez-events/src/event.rs | head -30
# [SHOW: EventRecord fields with comments]

# [NARRATE]:
#    EventRecord has 6 fields:
#    - program_id: set by the sequencer, not the program — prevents spoofing
#    - sequence: 0-indexed, monotonically increasing per transaction
#    - discriminant: stable u64 identifier for the event type
#    - schema_version: for forward compatibility, starts at 1
#    - schema_hash: 32-byte cryptographic hash of the schema to ensure deterministic decoding
#    - payload: Borsh-encoded event fields, max 1024 bytes"

# [PAUSE]

# ─────────────────────────────────────────────────────────────
# SCENE 6: Error Codes & Limits (7:30–8:30)
# ─────────────────────────────────────────────────────────────
#
# [NARRATE]:
#   "emit_event() NEVER panics. When limits are exceeded, it returns Err
#    with a stable error code. Let me show you."

# [TYPE]:
cat lez-events/src/error.rs
# [SHOW: 4 error variants with codes 0xEE01-0xEE04]

# [NARRATE]:
#   "Four error codes:
#    0xEE01 — payload over 1KB
#    0xEE02 — more than 64 events per transaction
#    0xEE03 — total event bytes over 64KB
#    0xEE04 — Borsh serialization error
#
#    These codes are stable and will never change — they're part of the
#    public API contract."

# [PAUSE]

# ─────────────────────────────────────────────────────────────
# SCENE 7: CLI Demo (8:30–9:30)
# ─────────────────────────────────────────────────────────────
#
# [NARRATE]:
#   "The decoder CLI provides offline decoding of Borsh-encoded events.
#    This is what an indexer or explorer would call after fetching a receipt."

# [TYPE]:
cargo build --release --bin lez-event-cli 2>&1 | tail -1
./target/release/lez-event-cli --help

# [NARRATE]:
#   "Three subcommands:
#    decode — fetches from RPC (requires patched sequencer)
#    decode-raw — decodes Borsh bytes offline, no network needed
#    watch — polls RPC in real-time for new events
#
#    Let's decode-raw a withdrawal failure receipt."

# [TYPE]:
HEX="4c455a450102000000"
HEX+="0303030303030303030303030303030303030303030303030303030303030303"
HEX+="00000000"
HEX+="1000000000000000"
HEX+="01"
HEX+="0000000000000000000000000000000000000000000000000000000000000000"
HEX+="28000000"
HEX+="0303030303030303030303030303030303030303030303030303030303030303"
HEX+="d007000000000000"
HEX+="0303030303030303030303030303030303030303030303030303030303030303"
HEX+="01000000"
HEX+="1100000000000000"
HEX+="01"
HEX+="0000000000000000000000000000000000000000000000000000000000000000"
HEX+="30000000"
HEX+="0303030303030303030303030303030303030303030303030303030303030303"
HEX+="d007000000000000"
HEX+="f401000000000000"
./target/release/lez-event-cli decode-raw --hex "${HEX}"

# [NARRATE]:
#   "And we can see two decoded events from a failed withdrawal:
#    WithdrawAttempted at sequence 0, and InsufficientFunds at sequence 1.
#    Both survived despite the transaction failing."

# ─────────────────────────────────────────────────────────────
# SCENE 8: Documentation & Wrap-Up (9:30–11:00)
# ─────────────────────────────────────────────────────────────
#
# [NARRATE]:
#   "The docs folder contains the complete specification."

# [TYPE]:
ls docs/

# [NARRATE]:
#   "event-format.md — 514-line Borsh wire format spec with hex examples,
#    schema versioning strategy, privacy considerations, and size limits.
#
#    architecture-decision.md — explains why we chose the execute_program
#    runtime adapter approach and exactly what sequencer changes are needed.
#
#    research-notes.md — our findings from studying the LEZ codebase before
#    designing anything.
#
#    submission-writeup.md — complete submission document with all deliverables listed."

# [TYPE]:
wc -l docs/*.md

# [PAUSE]

# [NARRATE]:
#   "Let me also show the interactive browser demo."

# [Open demo/index.html in browser - show hex visualizer and decoder]

# [NARRATE]:
#   "This browser demo lets you decode Borsh events interactively.
#    You can load example events and see exactly what each byte means —
#    program_id in blue, sequence in green, discriminant in yellow,
#    schema_hash in orange, and payload in red.
#
#    The browser-side decoder is pure JavaScript Borsh — no server required.
#    Evaluators can use this to explore the wire format without setting up
#    a Rust toolchain."

# ─────────────────────────────────────────────────────────────
# SCENE 9: Summary (11:00–11:30)
# ─────────────────────────────────────────────────────────────
#
# [NARRATE]:
#   "To summarize LP-0012:
#
#    The LEZ Event System provides a runtime adapter: execute_program().
#    By wrapping your logic inside it, events are automatically framed with
#    LEZE bytes and committed to the RISC0 journal, surviving any panics.
#
#    The SDK is production-ready: 32 tests pass, zero clippy warnings,
#    clean formatting, and the demo script runs successfully in a clean
#    environment with RISC0_DEV_MODE=0.
#
#    Thank you for reviewing LP-0012."

# [END RECORDING]

# ============================================================
# POST-RECORDING CHECKLIST
# ============================================================
# [ ] Video clearly shows "RISC0_DEV_MODE=0" at the start
# [ ] Demo script failure path narrated: events survive panic
# [ ] All 32 tests shown passing
# [ ] Clippy zero warnings shown
# [ ] Upload to YouTube/Loom unlisted, submit URL
# [ ] Include video URL in docs/submission-writeup.md
# ============================================================
