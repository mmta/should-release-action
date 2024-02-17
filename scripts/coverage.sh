#!/bin/sh

root=$(git rev-parse --show-toplevel)
cd $root

mkdir -p ./coverage

exec cargo llvm-cov --lcov --output-path ./coverage/lcov.info
