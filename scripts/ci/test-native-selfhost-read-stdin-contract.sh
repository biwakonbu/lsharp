#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
COMPILER_BASE="$ROOT/selfhost/src/Backend/Wasm/CompilerBase.ls"
COMPILER="$ROOT/selfhost/src/Backend/Wasm/Compiler.ls"
IR="$ROOT/selfhost/src/IR/IR.ls"
NATIVE="$ROOT/selfhost/src/Backend/Native/NativeCodegen.ls"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

assert_contains() {
  local path="$1"
  local text="$2"
  grep -F -- "$text" "$path" >/dev/null || fail "$path does not contain: $text"
}

assert_contains "$COMPILER_BASE" '(defn op-read-stdin [] 91)'
assert_contains "$COMPILER_BASE" '(defn builtin-read-stdin [] 3103017793106833)'
assert_contains "$COMPILER_BASE" '3103017793106833'
assert_contains "$COMPILER" '(defn nullary-builtin-op [bop] (if (= bop 75) true (if (= bop 86) true (= bop 91))))'
assert_contains "$IR" '(defn ir-read-stdin [] 91)'
assert_contains "$NATIVE" '(defn emit-x86-selfhost-read-stdin-helper []'
assert_contains "$NATIVE" '(defn emit-aarch64-selfhost-read-stdin-helper []'
assert_contains "$NATIVE" '(defn x86-selfhost-read-stdin-helper-offset [import-stub-offset import-count]'
assert_contains "$NATIVE" '(defn aarch64-selfhost-read-stdin-helper-offset [import-stub-offset import-count]'
assert_contains "$NATIVE" '(if (= opcode 91)'
assert_contains "$NATIVE" '(append-native-bytes-rooted result (emit-x86-selfhost-read-stdin-helper)'
assert_contains "$NATIVE" '(append-native-bytes-rooted result (emit-aarch64-selfhost-read-stdin-helper)'

echo "native read-stdin contract test passed"
