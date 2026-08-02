# ADR: v0.3 native component artifact postflight

## Status

Verified partial slice (2026-08-02). Native compile/build component packaging
now rejects invalid Wasm bytes before handing them to the next boundary or
atomically replacing the requested output.

## Context

native-selfhost-component.py already kept native compilation and
wasm-tools component new behind an atomic output replacement. Its success
condition only required regular files, so a successful child process could
leave non-Wasm bytes for the next stage. The native helper must make the
artifact boundary explicit without falling back to cargo, rustc, host lsharp,
or another compiler.

## Decision

- After native compile/build, require the temporary core artifact to be a
  non-symlink regular file beginning with the Wasm magic '\0asm'.
- After wasm-tools component new, apply the same check to the temporary
  component before os.replace.
- Invalid core bytes fail before wasm-tools is invoked. Invalid packaged
  bytes fail before final replacement. In both cases the existing output is
  preserved and task-owned temporary files are cleaned up.
- This is a byte-shape postflight, not Wasm semantic validation or target
  runtime execution. The Rust driver remains the canonical oracle for its
  existing runnable-component tests; no Rust fallback is added to the native
  helper.

## Evidence

- RED: the fake native component harness accepted a zero-status invalid core
  far enough to invoke wasm-tools, and accepted invalid packaged bytes as a
  successful replacement.
- GREEN: the same harness rejects both invalid core and invalid packaged
  bytes, preserves the sentinel output, avoids the downstream invocation for
  invalid core, and removes the temporary component. Existing failure,
  warning, explicit-tool, directory-output, and atomic-replace cases remain
  green.
- python3 scripts/ci/test-native-selfhost-component.py,
  python3 -m py_compile scripts/native-selfhost-component.py, and the Rust
  selfhost component-boundary test (cargo test -p lsharp-wasm --test e2e
  selfhost_bootstrap_contracts::test_e2e_selfhost_embedded_cli_component_output_has_explicit_external_boundary)
  pass. The broader existing Rust runnable-component test was also attempted,
  but remains blocked by its pre-existing expectation of a wasm-size: summary
  while the current component delegation returns a localized success summary;
  no Rust source changed in this slice. The docs audit and git diff --check
  are the remaining commit gates.

## Remaining boundary

This does not prove Wasm semantic validity, source/ftable/import parity,
standalone runtime behavior, current-source Mac/Linux stage0 runtime, live
provider/auth acquisition, or packaged/rollback bytes parity. Those remain
[~] under EC-M3-04 / EC-M3-05 and M3-04-N1 / M3-05-N9.
