#!/bin/bash
set -xe

rustup toolchain install nightly-2021-04-01 --profile minimal
rustup target add wasm32-unknown-unknown --toolchain nightly-2021-04-01
rustup default nightly-2021-04-01