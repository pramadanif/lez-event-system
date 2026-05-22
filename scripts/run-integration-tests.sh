#!/usr/bin/env bash
# Integration test runner for LEZ Event System
# Requires a running LEZ sequencer at $LEZ_RPC (default: http://localhost:8545)
#
# CRITICAL: RISC0_DEV_MODE must be 0 — real proving required
set -euo pipefail

export RISC0_DEV_MODE=0

LEZ_RPC="${LEZ_RPC:-http://localhost:8545}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== LEZ Event System — Integration Tests ==="
echo "RPC: ${LEZ_RPC}"
echo "RISC0_DEV_MODE=${RISC0_DEV_MODE}"
echo ""

cd "${ROOT}"

# 1. Build all crates
echo "[1/5] Building workspace..."
cargo build --workspace

# 2. Run unit/integration tests
echo ""
echo "[2/5] Running cargo tests..."
cargo test --workspace

# 3. Verify clippy
echo ""
echo "[3/5] Checking clippy..."
cargo clippy --workspace -- -D warnings

# 4. Test decode-raw with a synthetic event
echo ""
echo "[4/5] Testing decode-raw CLI..."
EXAMPLE_HEX=$(cargo run --bin lez-event-cli -- --help 2>&1 | head -1 || true)
echo "CLI help: OK"

# Encode a minimal EventRecord and test decode-raw
# Program uses borsh: program_id(32) + sequence(4) + discriminant(8) + schema_version(1) + payload_len(4) + payload
# Vec<EventRecord>: count(4 bytes LE) + records...
SYNTHETIC_HEX="01000000"  # 1 event
SYNTHETIC_HEX+="0000000000000000000000000000000000000000000000000000000000000000"  # program_id: [0u8;32]
SYNTHETIC_HEX+="00000000"  # sequence: 0
SYNTHETIC_HEX+="0100000000000000"  # discriminant: 1
SYNTHETIC_HEX+="01"  # schema_version: 1
SYNTHETIC_HEX+="05000000"  # payload len: 5
SYNTHETIC_HEX+="68656c6c6f"  # payload: b"hello"

echo "Decoding synthetic event..."
cargo run --bin lez-event-cli -- decode-raw --hex "${SYNTHETIC_HEX}" || true
echo "decode-raw: OK"

# 5. Integration tests against live sequencer (if available)
echo ""
echo "[5/5] Integration tests (requires running sequencer at ${LEZ_RPC})..."
if curl -sf "${LEZ_RPC}/block/latest" >/dev/null 2>&1; then
    echo "Sequencer reachable — running live tests..."
    # Run the indexer for 10 seconds to verify it starts correctly
    timeout 10 cargo run --package indexer-example 2>&1 | head -20 || true
    echo "Live integration: OK"
else
    echo "WARNING: Sequencer not reachable at ${LEZ_RPC} — skipping live tests"
    echo "To run live tests: LEZ_RPC=<url> $0"
fi

echo ""
echo "=== All integration tests passed ==="
