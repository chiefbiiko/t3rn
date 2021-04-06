#!/bin/bash
set -xo pipefail

# BUG: `build`ing with existing ./target fails, thus 2 re`build`..
# cargo +nightly-2021-04-01 clean
cargo +nightly-2021-04-01 build
yes | ./target/debug/circuit purge-chain --dev
./target/debug/circuit --dev -lruntime=debug