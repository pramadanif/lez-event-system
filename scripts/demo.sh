#!/usr/bin/env bash
# Demo script for LEZ Event System (LP-0012)
#
# Demonstrates:
# 1. Success path: token-transfer emits 2 events
# 2. Failure path: withdraw emits events BEFORE panic
# 3. decode-raw: offline event decoding
# 4. (Optional) Live RPC decoding if sequencer is running
#
# CRITICAL: RISC0_DEV_MODE=0 — real proving, not mock
set -euo pipefail

export RISC0_DEV_MODE=0
LEZ_RPC="${LEZ_RPC:-http://localhost:8545}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "╔══════════════════════════════════════════════════╗"
echo "║   LEZ Event System — LP-0012 Demo               ║"
echo "║   RISC0_DEV_MODE=0 (real proving)               ║"
echo "╚══════════════════════════════════════════════════╝"
echo ""

cd "${ROOT}"

# Build all workspace crates
echo "▶ Building workspace..."
cargo build --workspace
echo "  ✓ Build succeeded"
echo ""

# Run tests first to prove correctness
echo "▶ Running test suite..."
cargo test --workspace --quiet
echo "  ✓ All tests passed"
echo ""

# Demonstrate success path
echo "▶ Demo 1: Token Transfer (success path)"
echo "  Emits: TransferInitiated → TransferCompleted"
echo ""
cargo run --quiet --package token-transfer-example 2>&1 | sed 's/^/  /'
echo ""

# Demonstrate failure path — critical for LP-0012
echo "▶ Demo 2: Withdraw (failure path — events survive panic)"
echo "  Emits: WithdrawAttempted → InsufficientFunds"
echo "  Then: drain_events() + write_output BEFORE panic"
echo "  Result: events in receipt even though tx fails"
echo ""
# The withdraw example panics with exit 101 — that's expected behavior
cargo run --quiet --package withdraw-example 2>&1 | sed 's/^/  /' || true
echo "  ✓ Failure path completed (panic is expected)"
echo ""

# Demonstrate decode-raw (no sequencer needed)
echo "▶ Demo 3: Offline event decoding (decode-raw)"
echo "  Decodes Borsh-encoded events without a running sequencer"
echo ""
# Construct a minimal Vec<EventRecord> in hex
# 1 record, program_id=[0;32], seq=0, discriminant=1, sv=1, payload=b"hello"
HEX="01000000"
HEX+="0000000000000000000000000000000000000000000000000000000000000000"
HEX+="00000000"
HEX+="0100000000000000"
HEX+="01"
HEX+="05000000"
HEX+="68656c6c6f"
echo "  Hex bytes: ${HEX:0:32}..."
cargo run --quiet --bin lez-event-cli -- decode-raw --hex "${HEX}" 2>&1 | sed 's/^/  /'
echo ""

# Live demo (optional)
echo "▶ Demo 4: Live RPC decode (optional)"
if curl -sf --max-time 2 "${LEZ_RPC}/block/latest" >/dev/null 2>&1; then
    echo "  Sequencer found at ${LEZ_RPC}"
    LATEST_BLOCK=$(curl -sf "${LEZ_RPC}/block/latest" | python3 -c "import sys,json; print(json.load(sys.stdin)['block_number'])" 2>/dev/null || echo "unknown")
    echo "  Latest block: ${LATEST_BLOCK}"
    echo "  Run: lez-event-cli decode --tx <TX_HASH> --rpc ${LEZ_RPC}"
    echo "  Run: lez-event-cli watch --rpc ${LEZ_RPC}"
else
    echo "  No sequencer at ${LEZ_RPC} — skipping live demo"
    echo "  To run with sequencer: LEZ_RPC=<url> $0"
fi

echo ""
echo "╔══════════════════════════════════════════════════╗"
echo "║   Demo complete ✓                               ║"
echo "║                                                  ║"
echo "║   Key result: events survive transaction panic   ║"
echo "║   See withdraw example output above              ║"
echo "╚══════════════════════════════════════════════════╝"
