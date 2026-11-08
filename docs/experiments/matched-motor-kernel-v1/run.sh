#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
OUT="${1:?pass a new caller-owned output directory}"
mkdir "$OUT"
OUT="$(cd "$OUT" && pwd)"
python3 -B lower_plan.py > "$OUT/lowering-verification.json"
rustfmt --edition 2021 --check lib.rs probe.rs
rustc --version --verbose > "$OUT/rustc-version.txt"
rustc --edition=2021 --test lib.rs -D warnings -C debuginfo=0 -o "$OUT/kernel-tests"
"$OUT/kernel-tests" --test-threads=1 > "$OUT/tests.log"
rustc --edition=2021 --crate-type=lib --crate-name matched_motor_kernel lib.rs -D warnings -C debuginfo=0 -o "$OUT/libmatched_motor_kernel.rlib"
rustc --edition=2021 probe.rs --extern matched_motor_kernel="$OUT/libmatched_motor_kernel.rlib" -D warnings -C debuginfo=0 -o "$OUT/kernel-probe"
"$OUT/kernel-probe" > "$OUT/probe.csv"
sha256sum "$OUT/kernel-tests" "$OUT/libmatched_motor_kernel.rlib" "$OUT/kernel-probe" "$OUT/probe.csv" > "$OUT/SHA256SUMS"
