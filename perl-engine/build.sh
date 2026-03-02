#!/usr/bin/env bash
set -euo pipefail

cargo build --release -p yggdrasilffi
mkdir -p lib/Yggdrasil/lib

[ -f ../target/release/yggdrasilffi.dll ] && cp ../target/release/yggdrasilffi.dll ./lib/Yggdrasil/lib
[ -f ../target/release/libyggdrasilffi.dylib ] && cp ../target/release/libyggdrasilffi.dylib ./lib/Yggdrasil/lib
[ -f ../target/release/libyggdrasilffi.so ] && cp ../target/release/libyggdrasilffi.so ./lib/Yggdrasil/lib
