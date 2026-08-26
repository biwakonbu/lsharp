# ADR: Actual selfhost CLI check file boundary

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `selfhost/src/App/Cli.ls`,
  `crates/lsharp-wasm/tests/e2e/selfhost_cli_actual_main_args.rs`,
  `crates/lsharp-wasm/tests/e2e/selfhost_cli_core.rs`,
  `crates/lsharp-wasm/tests/e2e/support.rs`
- Related: `V2-16c`, `LEGACY-TOOL-01`

## Context

The public `check <file>` test was ignored even though the source bundle was intended to be
the actual `App.Cli` entrypoint. The bundle also lagged behind a new `ManifestInput` import.
Separately, two recently added `Cli.ls` branches left the enclosing `defn` open, so the bundle
parser stopped at the following definition.

## Decision

Keep the existing CLI behavior and repair only the source syntax and bundle inventory. Include
`ManifestInput.ls` in the support path resolver, embedded source map, and CLI runtime module list.
Make the single-file `check` regression a normal test once its output contract is green. Keep the
native stage0, external tool, release, and target matrix boundaries separate from this Rust-host
oracle lane.

## Evidence

- `test_e2e_selfhost_cli_main_with_args_check_file` executes `check input.ls` and returns
  `Int` / `diagnostics:0` with exit success (**下の訂正節を参照**)。
- `test_support_selfhost_cli_runtime_bundle_cached` verifies the cached bundle identity and
  `Tools.Validation.ManifestInput` module marker.
- `test_e2e_selfhost_cli_main_no_args_shows_help` executes the same actual bundle without argv and
  returns the `Usage: lsharp <command>` and `Commands:` help markers with exit success.
- `test_e2e_selfhost_cli_main_batched_version_and_parse_argv` compiles the bundle once, then
  verifies `--version` and `-v` return `lsharp 0.1.0`, and `parse input.ls` returns the expected
  `decls:1` / `diagnostics:0` summary.

### 訂正 (2026-08-24、`--ignored` lane 全量 sweep)

本節 1 件目の Evidence (`..._check_file`) は**現在の実測と食い違う**。当時の観測なので原文は残し、差分をここに書く。

| 当時の主張 | 2026-08-24 実測 |
|---|---|
| `Int` / `diagnostics:0` / exit success | `Fn` / `diagnostics:0` / exit success |

`diagnostics:0` と exit success は現在も成り立つ。**食い違うのは型名だけ**である
(`crates/lsharp-wasm/tests/e2e/selfhost_cli_actual_main_args.rs:28`、
`left: ["Fn", "diagnostics:0"]` / `right: ["Int", "diagnostics:0"]`)。

原因は本 ADR の観測ミスではなく、**後続の契約変更を本 ADR へ反映しなかったこと**である。
`914bd9f1` (`I-45`、[`decisions-selfhost-zero-arity-defn-type.md`](decisions-selfhost-zero-arity-defn-type.md)、
2026-08-22 accepted) が 0-arity `defn` を `Unit -> body` として登録する契約を採り、
`render-type-text` はこれを `"Fn"` と表示する。本 Evidence の fixture は 0-arity なので、
`Int` を返していた当時の観測はこの契約変更によって無効になった。

**pin は `"Fn"` へ追随させた (2026-08-27)。** 経緯は
[`decisions-selfhost-zero-arity-defn-type.md`](decisions-selfhost-zero-arity-defn-type.md) の「7〜11 本目」節。
判別の結果、これは `render-type-text` のバグではなく `I-45` 契約そのものだと確定した
(詳細は `ISSUES.md` の `I-76`)。

**方法論として残す点**: 本 ADR は `#[ignore]` 下の test の**自称期待値**を Evidence にしていた。
lane から外れた test は赤に転じても誰も気付かないので、Evidence として引くなら
**どの lane が実際にそれを回しているか**を併記しなければならない。この失敗モードは
`ISSUES.md` の `I-70` が 43 件を突き合わせて確認したもので、本 ADR はその 3 件のうちの 1 件である。

## Boundary

This is a Rust-host actual Wasm source-bundle slice. It does not prove native stage0 `check`,
no-arg/version/parse parity, the remaining public commands, external helper parity, release
provenance, or both supported target artifacts. `V2-16c` and `LEGACY-TOOL-01` remain `[~]` in
`TODO.md`.
