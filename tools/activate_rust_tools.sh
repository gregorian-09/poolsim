#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TOOLS_BIN="$ROOT_DIR/.tools/bin"

export PATH="$TOOLS_BIN:$PATH"
export LLVM_COV="/usr/lib/llvm-18/bin/llvm-cov"
export LLVM_PROFDATA="/usr/lib/llvm-18/bin/llvm-profdata"

echo "Activated Rust toolchain helpers for poolsim:"
echo "  PATH prepended with: $TOOLS_BIN"
echo "  LLVM_COV=$LLVM_COV"
echo "  LLVM_PROFDATA=$LLVM_PROFDATA"
