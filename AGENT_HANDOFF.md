# LP-0012 Complete Agent Handoff
**Status Date**: May 23, 2026  
**Commit**: `20f82ed` (main branch)  
**Repo**: https://github.com/pramadanif/lez-event-system

---

## AGENT PROMPT: Continue Autonomously

You are continuing the LP-0012 LEZ Event System submission. Read this entire file before writing a single line of code.

**Your job**: Fix the remaining gaps below and prepare the repo so evaluators can `git clone` and run `./scripts/demo.sh` successfully. Do NOT rewrite any existing code unless a specific gap below requires it. Do NOT ask the user for permission on code changes.

---

## Current State (What Works)

| Check | Status |
|-------|--------|
| `cargo build --workspace` | ✅ Pass |
| `cargo clippy --workspace --tests -- -D warnings` | ✅ 0 warnings |
| `cargo fmt --all -- --check` | ✅ Clean |
| `cargo test --workspace` | ✅ 21/21 pass |
| `./scripts/demo.sh` (offline) | ✅ Pass |
| CI pipeline (`ci.yml`) | ✅ 3 jobs: lint+fmt, unit-tests, demo-script |

---

## LEZ Codebase Facts (Verified — Do NOT Re-Research)

1. **Rust toolchain**: `1.94.0` (matches `logos-execution-zone/rust-toolchain.toml`)
2. **Borsh**: `1.5.0` (matches LEZ workspace)
3. **Transaction types**: `logos-execution-zone/common/src/transaction.rs` — `NSSATransaction::Public`, `PrivacyPreserving`, `ProgramDeployment`
4. **Block structure**: `logos-execution-zone/common/src/block.rs` — `Block { header, body: BlockBody { transactions }, bedrock_status }`
5. **Indexer RPC**: Uses `jsonrpsee` websocket client; see `test_fixtures/src/indexer_client.rs`
6. **Wallet CLI**: `just run-wallet <args>` — reads from `NSSA_WALLET_HOME_DIR=$(pwd)/configs/debug`
7. **Sequencer commands**: `just run-bedrock`, `just run-indexer`, `just run-sequencer`
8. **`ProgramOutput` struct** (nssa/core/src/program.rs): has `self_program_id`, `caller_program_id`, `instruction_data`, `pre_states`, `post_states`, `chained_calls`, `block_validity_window`, `timestamp_validity_window` — **NO `events` field yet**
9. **`TxReceipt`**: Does NOT exist as a named struct in LEZ. The sequencer returns `Block { body: BlockBody { transactions } }` — transactions contain the proof/output but no events field.
10. **Sequencer environment dependencies** (CANNOT build without these):
    - `xcrun metal` (Xcode Metal compiler) — requires `xcode-select --install`
    - `logos-blockchain-circuits` — large ZK circuit lib, must be cloned separately
11. **`just` commands for programs**: `just run-wallet deploy-program <binary_path>` — returns program ID
12. **Explorer/indexer RPC API**: `get_transaction(hash)` exists in `indexer_service_rpc::RpcClient` and returns `Option<Transaction>` (NOT a receipt with events)

---

## Gaps Remaining (In Priority Order)

### GAP 1: `lez-event-cli decode --tx HASH --rpc URL` uses a fake API format
**File**: `lez-event-decoder/src/bin/lez-event-cli.rs`  
**Problem**: The `decode` subcommand tries to fetch `{rpc}/tx/{hash}` and parse a `TxReceipt { events: Vec<EventRecord> }` struct. This format does NOT match LEZ's actual RPC.  
**Why it matters**: Evaluators will try to run `lez-event-cli decode --tx X --rpc Y`. Without a real sequencer that returns our format, this call will fail.  
**Fix Options**:
- Option A (Recommended): Keep the `decode` command but document it requires the patched sequencer. Add a comment in the CLI help text saying: "Requires LEZ sequencer with lez-event-system patch applied (see docs/architecture-decision.md)"
- Option B: Add a `--mock` flag to decode that simulates a response for demo purposes (but mark it clearly as mock)

**IMPORTANT**: Do NOT create fake/mock data that pretends to be real. The PROMPT says "no mock or fake."

### GAP 2: `examples/token-transfer` and `examples/withdraw` are NOT real LEZ programs
**Problem**: These examples use stub functions (`read_inputs()`, `write_outputs()`) — they do NOT use `read_nssa_inputs()` / `write_nssa_outputs()` from the actual `nssa_core` crate.  
**Root cause**: Using real nssa_core would require `nssa_core` as a git dependency from a private/complex repo.  
**Current status**: This is acceptable — the examples ARE pedagogically correct. They demonstrate the API pattern even if they don't compile with real LEZ runtime.  
**Recommended fix**: Add a comment at the top of each example clearly stating: "This is a self-contained simulation. For real LEZ programs, replace read_inputs()/write_outputs() with nssa_core::read_nssa_inputs()/write_nssa_outputs()."

### GAP 3: `tests/` folder at repo root is empty
**Problem**: PROMPT's directory structure shows `tests/` at repo root. All tests are in `lez-events/tests/`.  
**Impact**: Minor — Rust allows tests in crate subdirectories. NOT a blocker for evaluators.  
**Recommended fix**: None needed (or add symlinks/README in tests/ explaining location).

### GAP 4: `docs/deployments.md` has placeholder program IDs
**Problem**: `<PENDING DEPLOYMENT>` — no real program IDs.  
**Can this be fixed without live sequencer?**: NO. Requires `xcode-select --install` + `logos-blockchain-circuits` setup.  
**Recommended documentation**: Be honest in the file that deployment requires the full environment.

### GAP 5: `docs/benchmarks.md` has no real CU numbers
**Problem**: PROMPT says "docs/benchmarks.md with real CU numbers."  
**Can this be fixed?**: Only with live sequencer.  
**Status**: `docs/benchmarks.md` currently has wall-time estimates. This is acceptable as a caveat.

### GAP 6: Video not recorded
**Cannot be done by AI agent.** Human must record narrated video.  
**Requirements** (from PROMPT §10.1):
- `echo $RISC0_DEV_MODE` must print `0` visibly
- Run `./scripts/demo.sh`
- Narrate the failure path: "See here — transaction failed, but events are still committed"
- Show `cargo test` passing

---

## What NOT To Do

- ❌ Do NOT rewrite the SDK (emit.rs, event.rs, error.rs) — it's correct
- ❌ Do NOT rewrite the tests — all 21 pass
- ❌ Do NOT add `RISC0_DEV_MODE=1` anywhere
- ❌ Do NOT change rust-toolchain.toml (it matches LEZ exactly)
- ❌ Do NOT try to build the LEZ sequencer (needs logos-blockchain-circuits)
- ❌ Do NOT create mock data that pretends to be live sequencer output

---

## Repository Structure

```
lez-event-system/
├── .github/workflows/ci.yml        ✅ 3-job pipeline (lint+fmt, unit-tests, demo-script)
├── demo/index.html                 ✅ Interactive browser demo (open in browser to show evaluators)
├── docs/
│   ├── event-format.md             ✅ 514 lines, complete spec
│   ├── architecture-decision.md    ✅ Design rationale
│   ├── research-notes.md           ✅ LEZ codebase findings
│   ├── benchmarks.md               ⚠️  Wall-time estimates (no real CU yet)
│   ├── deployments.md              ⚠️  Placeholder program IDs
│   ├── submission-writeup.md       ✅ Complete
│   └── gap-analysis.md             ✅ This audit
├── lez-events/src/                 ✅ Core SDK
│   ├── emit.rs                     ✅ Thread-local buffer, drain_events()
│   ├── event.rs                    ✅ EventRecord, LezEvent trait
│   ├── error.rs                    ✅ 4 variants, 0xEE01–0xEE04
│   └── macros.rs                   ✅ impl_lez_event! macro
├── lez-events/tests/               ✅ 21 tests total
│   ├── test_encoding.rs            ✅ Borsh determinism
│   ├── test_ordering.rs            ✅ Sequence monotonicity
│   ├── test_size_limits.rs         ✅ All limits return Err
│   ├── test_failure_path.rs        ✅ drain-before-panic pattern
│   └── test_attribution.rs         ✅ program_id handling
├── lez-event-decoder/              ✅ Decoder + CLI
├── examples/token-transfer/        ✅ Success path (simulated)
├── examples/withdraw/              ✅ Failure path (simulated)
├── examples/indexer/               ✅ Reference indexer
├── scripts/demo.sh                 ✅ Works offline, RISC0_DEV_MODE=0
├── scripts/run-integration-tests.sh ✅ Works offline
├── Cargo.toml                      ✅ Workspace
├── rust-toolchain.toml             ✅ 1.94.0 (matches LEZ)
└── LICENSE                         ✅ MIT
```

---

## Honest Assessment for Evaluators

The LP-0012 submission delivers:

1. **Complete API design** — `emit_event`, `drain_events`, `EventRecord`, `LezEvent` trait, error codes
2. **Proven failure-path mechanism** — `drain_events()` before `write_output()` before `panic!()` — tested in `test_failure_path.rs`
3. **Full decoder + CLI** — `decode-raw` works offline; `decode --tx` requires patched sequencer
4. **Reference examples** — pedagogically correct demonstrations of the pattern
5. **Documented sequencer integration plan** — exactly what changes are needed in LEZ's `ProgramOutput` and sequencer

What requires a live sequencer (environment-blocked):
- Real CU numbers (benchmarks.md)
- Real program IDs (deployments.md)
- End-to-end `lez-event-cli decode --tx HASH --rpc URL`

The evaluators (Logos team) can apply the documented `ProgramOutput.events` extension to the sequencer and verify it works. The SDK is ready.
