#!/bin/sh

root=$(git rev-parse --show-toplevel)
cd $root

mkdir -p ./coverage

llvm_cmd="llvm-cov --lcov --output-path ./coverage/lcov.info"
cargo_cmd="cargo ${llvm_cmd}"

[ "$1" = "watch" ] && exec cargo watch -x "${llvm_cmd}" || cargo ${llvm_cmd}
