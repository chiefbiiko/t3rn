#!/bin/bash
set -xeu

echo "[run-circuit.sh] Building circuit with 'cargo build'..."
cargo build || exit

echo "[run-circuit.sh] Cleaning the circuit dev database with './target/debug/circuit purge-chain --dev'..."
yes | ./target/debug/circuit purge-chain --dev

cd ./src/archive
cargo build || exit
cd ../..

base_path=$HOME/.local/share/io.t3rn.circuit

echo "[run-circuit.sh] Running circuit with './target/debug/circuit --dev -lruntime=debug'..."
./target/debug/circuit \
  --dev \
  -lruntime=debug \
  --pruning=archive \
  --base-path=$base_path

DATABASE_URL=postgres://user:pass@localhost:5432/archive \
CHAIN_DATA_DB=$base_path/chains/dev/db \
./src/archive/debug/archive \
  TODO