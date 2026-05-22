# LP-0012 Agent Handoff & Autonomous Deployment Guide

**To the next AI Agent:** You are continuing a project that is **95% complete**. Your goal is to deploy the completed work to the LEZ testnet, gather the real benchmarks, and prepare the final artifacts for submission. **DO NOT re-research or rewrite the core SDK/CLI.** Read this document carefully to understand the context.

---

## Context: What is Completed

We have built a structured event system for the Logos Execution Zone (LEZ) named `lez-event-system` (LP-0012). 
- **The Core Problem Solved**: LEZ uses a RISC0 zkVM. When a program panics, state changes are reverted. We built a pattern (`drain_events()`) where events are committed to the RISC0 journal *before* the panic, ensuring events survive transaction failures.
- **Alignment with LEZ**: The toolchain is pinned to `1.94.0` (matching LEZ exactly). The encoding uses `borsh = "1.5.0"` (matching LEZ exactly). `RISC0_DEV_MODE=0` is strictly enforced everywhere.
- **Completed Components**: 
  1. `lez-events` (SDK with `emit_event` and `drain_events`)
  2. `lez-event-decoder` (Borsh decoder + CLI)
  3. Example programs (`token-transfer`, `withdraw`, `indexer`)
  4. 21/21 passing tests and zero clippy warnings.
  5. `docs/` and `scripts/` fully populated.

---

## To the USER: Regarding "Smart Contracts"
*Menjawab pertanyaan Anda: "apakah itu butuh smart contract atau gimana?"*
Tidak, Anda tidak perlu membuat smart contract baru. Dalam ekosistem LEZ, smart contract disebut sebagai "program" (program berbasis Rust yang berjalan di dalam RISC0 zkVM). Kita **sudah membuat** dua program tersebut di folder `examples/token-transfer` dan `examples/withdraw`. Agent selanjutnya hanya perlu men-deploy program yang sudah kita buat ini menggunakan CLI wallet dari LEZ.

---

## Agent Prompt: Autonomous Next Steps

**Prompt for the next agent:** 
"Execute Phase 9 & 10 deployment and benchmarking autonomously. Follow the steps below without asking the user for permission, except for the final video recording which requires human action."

### Step 1: Wallet Setup & Local Sequencer
You need the LEZ wallet to deploy programs. The wallet CLI is located inside the cloned `logos-execution-zone` repository.
1. Run the local LEZ sequencer in standalone mode:
   ```bash
   cd logos-execution-zone
   RUST_LOG=info cargo run --features standalone -p sequencer_service sequencer/service/configs/debug &
   ```
2. Build the wallet CLI if necessary, and use `just run-wallet check-health` to ensure it can communicate with the local sequencer.
3. If the wallet requires key generation or funding on the testnet/devnet, execute those commands via `just run-wallet ...`.

### Step 2: Build & Deploy Example Programs
We need to deploy the two example programs to the running sequencer to prove they work in a live environment.
1. Build the release binaries for the examples:
   ```bash
   cd lez-event-system
   cargo build --workspace --release
   ```
2. Deploy the programs using the LEZ wallet:
   ```bash
   cd logos-execution-zone
   just run-wallet deploy-program ../lez-event-system/target/release/token-transfer-example
   just run-wallet deploy-program ../lez-event-system/target/release/withdraw-example
   ```
3. Extract the `program_id` from the deployment output for both programs.

### Step 3: Update `docs/deployments.md`
Edit `docs/deployments.md` and replace `<PENDING DEPLOYMENT>` with the actual `program_id`s you received from Step 2.

### Step 4: Gather Real CU Benchmarks
The prompt requires real Compute Unit (CU) costs.
1. Submit transactions to the deployed programs via the wallet:
   ```bash
   just run-wallet submit --program <TOKEN_TRANSFER_ID> --instruction transfer --amount 100
   ```
2. Inspect the transaction receipts to find the CU consumption.
3. Update `docs/benchmarks.md` to replace the "Estimated" wall-time numbers with the actual RISC0/LEZ compute unit costs observed from the receipt.

### Step 5: Verify the Demo Script
Run `./scripts/demo.sh` from the `lez-event-system` directory to ensure the end-to-end flow is completely unbroken.

### Step 6: Final Commit
Commit the updated `docs/deployments.md` and `docs/benchmarks.md` and push to the `main` branch.

---

## Final Manual Step (For the User)
Once the agent completes the above, **the only remaining task is recording the video**. 
The video **must have voice narration** and show:
1. Walkthrough of `docs/event-format.md`.
2. Showing the code for `drain_events()` before a panic in `examples/withdraw/src/main.rs`.
3. Running `./scripts/demo.sh` while explicitly showing that `RISC0_DEV_MODE=0`.
4. Showing `cargo test` passing.
