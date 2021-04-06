#!/bin/bash
set -xe

rustup update stable-2021-03-25
rustup toolchain install nightly-2021-03-25 --profile minimal
rustup target add wasm32-unknown-unknown --toolchain nightly-2021-03-25