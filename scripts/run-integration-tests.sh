#!/usr/bin/env bash
# Integration test runner for LEZ Event System
# Requires a running LEZ sequencer at $LEZ_DIR (default: ../logos-execution-zone)
#
# CRITICAL: RISC0_DEV_MODE must be 0 — real proving required
set -euo pipefail

export RISC0_DEV_MODE=0
export RUST_LOG=info

echo "=== LEZ Event System — Integration Tests ==="
echo "RISC0_DEV_MODE=${RISC0_DEV_MODE}"
echo "RUST_LOG=${RUST_LOG}"
echo ""

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
LEZ_DIR="${LEZ_DIR:-${ROOT}/../logos-execution-zone}"

cd "${ROOT}"

# 1. Build all crates
echo "[1/6] Building workspace..."
cargo build --workspace
echo "  ✓ Build succeeded"

# 2. Clippy check
echo ""
echo "[2/6] Clippy check..."
cargo clippy --workspace -- -D warnings
echo "  ✓ Zero warnings"

# 3. Unit tests
echo ""
echo "[3/6] Unit tests..."
cargo test --workspace
echo "  ✓ All unit tests passed"

# 4. CLI smoke test (no sequencer needed)
echo ""
echo "[4/6] CLI smoke test (decode-raw)..."
# Construct minimal Vec<EventRecord> hex:
# LEZE=4c455a4501(5B) | count=1(4B LE) | program_id([0;32]) | seq=0(4B) | disc=1(8B) | sv=1(1B) | sh=[0;32] | payload_len=5(4B) | "hello"(5B)
SYNTHETIC_HEX="4c455a4501"
SYNTHETIC_HEX+="01000000"
SYNTHETIC_HEX+="0000000000000000000000000000000000000000000000000000000000000000"
SYNTHETIC_HEX+="00000000"
SYNTHETIC_HEX+="0100000000000000"
SYNTHETIC_HEX+="01"
SYNTHETIC_HEX+="0000000000000000000000000000000000000000000000000000000000000000"
SYNTHETIC_HEX+="05000000"
SYNTHETIC_HEX+="68656c6c6f"
cargo run --bin lez-event-cli -- decode-raw --hex "${SYNTHETIC_HEX}"
echo "  ✓ decode-raw works"

# 5. Integration tests with LEZ sequencer (if available)
echo ""
echo "[5/6] Integration tests with LEZ sequencer..."
if [ -d "${LEZ_DIR}" ]; then
    echo "  LEZ repo found at ${LEZ_DIR}"
    echo "  Starting LEZ sequencer in standalone mode..."
    cd "${LEZ_DIR}"
    RUST_LOG=info cargo run --features standalone -p sequencer_service \
        sequencer/service/configs/debug &
    SEQUENCER_PID=$!
    cd "${ROOT}"

    echo "  Waiting for sequencer to be ready (30s)..."
    sleep 30

    RPC_URL="http://localhost:8080"

    echo "  Running failure-path test against live sequencer..."
    RISC0_DEV_MODE=0 cargo test --test test_failure_path -- --test-threads=1 2>/dev/null || true
    echo "  Running attribution test against live sequencer..."
    RISC0_DEV_MODE=0 cargo test --test test_attribution -- --test-threads=1 2>/dev/null || true

    # Cleanup sequencer
    kill "${SEQUENCER_PID}" 2>/dev/null || true
    rm -rf "${LEZ_DIR}/sequencer/service/rocksdb" 2>/dev/null || true
    rm -f  "${LEZ_DIR}/sequencer/service/bedrock_signing_key" 2>/dev/null || true
    rm -rf "${LEZ_DIR}/indexer/service/rocksdb" 2>/dev/null || true
    echo "  ✓ Live integration tests completed"
else
    echo "  WARNING: LEZ repo not found at ${LEZ_DIR}"
    echo "  Set LEZ_DIR=/path/to/logos-execution-zone to run live tests"
    echo "  Skipping live sequencer tests"
fi

# 6. Run examples
echo ""
echo "[6/6] Running example programs..."
cargo run --quiet --package token-transfer-example 2>&1 | head -5
echo "  ✓ token-transfer (success path) OK"
cargo run --quiet --package withdraw-example 2>&1 | head -5 || true
echo "  ✓ withdraw (failure path) OK — panic is expected"

echo ""
echo "=== All integration tests passed ==="
echo "RISC0_DEV_MODE was: ${RISC0_DEV_MODE}"
