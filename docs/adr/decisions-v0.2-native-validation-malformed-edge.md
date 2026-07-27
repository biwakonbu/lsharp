# ADR: v0.2 native validation malformed edge rejection

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M3-01`、`EC-M3-02`、`docs/adr/decisions-v0.2-native-validation-orphan-edge.md`

## Context

Source graph の edge metadata は endpoint を二つ持つ typed edge として解決する。endpoint が
不足した form を report 生成へ進めると、malformed input が `unknown` や空の graph として
扱われるため、入力エラーと判定結果の境界が崩れる。Rust source adapter は malformed edge を
directive span 付きで拒否し、selfhost `IntentSource` は stable source graph error code `1` を
返すが、native source-file smoke は orphan/duplicate のみを検査していた。

## Decision

- `:motivates "intent:checkout/safe-cancel"` の endpoint 不足 fixture を追加する。
- `validate --source <fixture> --format json --emit-manifest <path>` は exit `1`、stderr の
  `source validation error:1`、空 stdout、manifest fileなしを返す。
- malformed input を report を保持する `fail` / `unknown` と混同せず、orphan/duplicate/write
  failure と同じ diagnostic-only fail-closed 境界として維持する。

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` は、
  `VALIDATION_MALFORMED_SOURCE` と malformed error contract が inner runner にないため失敗した。
- GREEN: 同じ focused command は fake Lima/provenance fixtureで
  `Linux native stage0 source-file provenance tests: OK` を返した。
- `bash -n`、native selfhost runner/docs audit、`git diff --check` は同じ変更で確認する。

## Boundary and follow-up

これは native source-file smoke の malformed-edge contract を拡張した verified sliceであり、
current source-commit に一致する packaged stage0 の実行、Mac/Linux artifact/runtime、manifest
bytes と standalone Wasm runtime の parity を完了扱いにしない。`TODO.md` の `EC-M2-03` と M3
aggregate は `[~]` を維持し、実 stage0 replay 後に同じ fixtureの actual execution を確認する。
