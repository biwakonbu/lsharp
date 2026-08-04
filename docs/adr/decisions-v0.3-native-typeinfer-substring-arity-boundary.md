# ADR: v0.3 native type inference substring arity boundary

## 状態

Verified partial slice（2026-08-04）。`V2-16b` / `LEGACY-IO-01` は `[~]` のまま維持する。

## 背景

native builtin type matrixには `substring` の valid `String -> Int -> Int -> String` fixtureがあったが、3引数 applyの
途中引数が不正な場合の failure valueが明示されていなかった。generic applyの引数収集・curried function型・最終 unifyのどこで
失敗するかを、I/Oやruntime ABIと混ぜずに固定する必要がある。

## 決定

- `substring` の最小 invalid fixtureは `(substring "abc" true 1)` とする。
- Rust selfhost type-inference oracleでは valid resultの `String` type nameと、middle argument mismatchの `result-failed` を同時に検証する。
- native CLI matrixには同じ sourceを追加する。current-source native evidenceがない場合、保存済み artifactの replayは sanityに限定し、
  Rust-free gateの passへ拡大解釈しない。

## 証跡

- `cargo test -p lsharp-wasm --test e2e e2e::selfhost_typeinfer_builtin_parity:: -- --nocapture` — 6 tests passed。
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/ci/test-native-selfhost-type-builtins.py --program ci-artifacts/native-release/aarch64-apple-darwin/d2dcea7e-standalone-command-line/program.native` — 5 tests passed（replay-only、source commit不一致）。
- `33d5483f` の変更は Rust oracleと `scripts/ci/test-native-selfhost-type-builtins.py` の同一 invalid fixtureを保持する。

## 残る境界

現行 checkout source commitでの Mac Apple Silicon / Linux x86_64 native check、全 builtinの型診断、診断 code/span parity、runtime/codegen、
全公開 command、component、packaged/release provenanceは未検証である。current-source stage0の strict provenanceを迂回する旧 artifact再利用は
採用しない。
