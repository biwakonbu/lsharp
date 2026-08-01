# V4 M1 semantic fixture evidence runbook

この runbook は、V4-M1-01 の Rust oracle / native stage0 / Wasm runtime
evidence を同じ source commit で再現するための operator 手順である。ここに
書かれたコマンドを実行しただけでは evidence は成立しない。各 target の
artifact、runtime、negative gate、cleanup の結果を確認し、
`semantic_fixture_evidence_audit.py` の結果を正本にする。

## Scope と所有権

- supported target は `aarch64-apple-darwin` と `x86_64-unknown-linux-gnu` の2つだけ。
- root checkout は編集せず、専用 worktree を
  `/Users/biwakonbu/github/tmp/<task>/` に作る。
- Rust compiler、native runner、Wasmtime、stage0 manifest は全て caller が
  絶対パスで指定する。host `lsharp`、暗黙の Rust fallback、provider/network は使わない。
- Linux x86_64 の Lima/QEMU replay は同じ仮説につき一つだけ実行する。既存の VM、lock、
  artifact が別 task 所有なら待機し、停止・削除・再起動しない。
- `ci-artifacts/` 以下の report は task-owned worktree 内で生成し、完了後に保存要否を確認して
  から削除する。root checkout や他 task の worktree の生成物は触らない。

## 0. Preflight

```bash
set -euo pipefail

ROOT="$(pwd)" # dedicated worktree root
SOURCE_COMMIT="$(git rev-parse HEAD)"
TASK="v4-m1-01-evidence-${SOURCE_COMMIT:0:12}"
EVIDENCE_ROOT="$ROOT/ci-artifacts/v4-m1-01/$SOURCE_COMMIT"
mkdir -p "$EVIDENCE_ROOT"

git status --short --branch
git rev-parse --verify HEAD
python3 scripts/ci/semantic_fixture_matrix.py \
  --manifest scripts/ci/semantic-fixture-matrix.json \
  --root "$ROOT" >/dev/null
```

`SOURCE_COMMIT` は report、stage0 manifest、comparison、evidence index の全てで同一でなければ
ならない。作業中に source が変わった場合は report を再利用せず、preflight からやり直す。

## 1. Fixture set の固定

一度の batch は同じ target / source commit に対して実行する。順序は producer が ID 順に正規化するが、
operator 側でも選択範囲を記録する。

```bash
FIXTURES=(
  --fixture-id invalid/lexer-unexpected-character
  --fixture-id invalid/module-not-found
  --fixture-id invalid/parser-unexpected-eof
  --fixture-id invalid/record-field-pattern-literal
  --fixture-id invalid/type-undefined-value
  --fixture-id valid/adt-pattern
  --fixture-id valid/argv-program-only
  --fixture-id valid/closure-allocation
  --fixture-id valid/free-list-growth
  --fixture-id valid/io-read-file
  --fixture-id valid/map-collections
  --fixture-id valid/module-import
  --fixture-id valid/nested-record-pattern
  --fixture-id valid/record-accessor
  --fixture-id valid/recursive-runtime
  --fixture-id valid/syntax-basic
)
TARGET="aarch64-apple-darwin" # or x86_64-unknown-linux-gnu
```

subset を検証する場合も `--fixture-id` を省略せず、evidence index の fixture list と完全一致させる。
重複 ID、未定義 ID、manifest にない command は audit で失敗する。

## 2. Rust oracle report

`COMPILER` と `WASMTIME` は `ROOT` 外の任意の host path でもよいが、symlink ではない実行可能ファイルを
明示する。work directory はこの task 専用にする。

```bash
COMPILER="/absolute/path/to/rust-oracle-compiler"
WASMTIME="/absolute/path/to/wasmtime"
ORACLE_WORK="$EVIDENCE_ROOT/oracle-work"
mkdir -p "$ORACLE_WORK"

python3 scripts/ci/semantic_fixture_rust_report.py \
  --manifest scripts/ci/semantic-fixture-matrix.json \
  --root "$ROOT" \
  "${FIXTURES[@]}" \
  --target "$TARGET" \
  --source-commit "$SOURCE_COMMIT" \
  --compiler "$COMPILER" \
  --wasmtime "$WASMTIME" \
  --work-dir "$ORACLE_WORK" \
  --output "$EVIDENCE_ROOT/oracle.json"
```

valid fixture は regular Wasm artifact と Wasmtime の stdout/stderr を記録する。invalid fixture は
compiler が non-zero で終了し、`LS####` code と source byte span の両方が得られた場合だけ記録する。
code/span が欠けたときに推測で補完してはならない。

## 3. Native stage0 report

`STAGE0_MANIFEST` の `kind`、target、source commit は preflight と一致させる。runner の環境から
`LSHARP_PATH` と embedded-component fallback flag を除去して実行するため、runner は明示した current
stage0 boundary を所有していなければならない。

```bash
RUNNER="/absolute/path/to/native-stage0-runner"
WASMTIME="/absolute/path/to/wasmtime"
STAGE0_MANIFEST="/absolute/path/to/current-source-stage0/manifest.json"
NATIVE_WORK="$EVIDENCE_ROOT/native-work"
mkdir -p "$NATIVE_WORK"

python3 scripts/ci/semantic_fixture_native_report.py \
  --manifest scripts/ci/semantic-fixture-matrix.json \
  --root "$ROOT" \
  "${FIXTURES[@]}" \
  --target "$TARGET" \
  --source-commit "$SOURCE_COMMIT" \
  --runner "$RUNNER" \
  --wasmtime "$WASMTIME" \
  --stage0-manifest "$STAGE0_MANIFEST" \
  --work-dir "$NATIVE_WORK" \
  --output "$EVIDENCE_ROOT/native.json"
```

stale stage0、target mismatch、source commit mismatch、unexpected invalid artifact は report を生成せず
停止する。Linux gate は Mac 側の古い artifact をコピーして済ませない。

## 4. Rust/native differential

```bash
python3 scripts/ci/semantic_fixture_diff.py \
  --manifest scripts/ci/semantic-fixture-matrix.json \
  --root "$ROOT" \
  --oracle "$EVIDENCE_ROOT/oracle.json" \
  --native "$EVIDENCE_ROOT/native.json" \
  "${FIXTURES[@]}" \
  > "$EVIDENCE_ROOT/comparison.json"
```

終了値の意味は固定する。

| exit | status | operator の扱い |
|---:|---|---|
| 0 | `pass` | report、artifact、runtime、期待値、source/target が一致した場合だけ次へ進む |
| 1 | `mismatch` | 差分を修正または原因分類する。完了 evidence として保存しない |
| 2 | `pending` | artifact/runtime/target gate を未完として残す。成功扱いにしない |

## 5. Evidence index audit

`INDEX` は [`v4-m1-06-evidence-index.schema.json`](../../schemas/v4-m1-06-evidence-index.schema.json)
に従う project-relative path で作る。report と comparison の source commit / target / fixture IDs は
index と一致させる。各 fixture の `command` は matrix に宣言されたものから一つ選び、4つの negative gate
を全て `pass` と明示する。

```bash
INDEX="$EVIDENCE_ROOT/index.json"

python3 scripts/ci/semantic_fixture_evidence_audit.py \
  --manifest scripts/ci/semantic-fixture-matrix.json \
  --root "$ROOT" \
  --index "$INDEX" \
  > "$EVIDENCE_ROOT/evidence-index.json"
```

audit は index の `pass` を信用せず、参照された report と comparison を再検証して再計算する。
`pass=0`、`pending=2`、`mismatch=1` 以外の状態、safe path でない参照、ADR の欠落、scope mismatch、
stale source/target、欠落した gate は fail closed になる。

## 6. Two-target と cleanup gate

`TARGET` を2つの supported target それぞれで変えて 1〜5 を繰り返す。片方だけの結果を aggregate の
完了証拠にしない。両 target の index、report、comparison を source commit ごとに棚卸しする。

```bash
git status --short --branch
git diff --check
ps -axo pid=,etime=,command= | rg '(cargo|rustc|wasmtime|limactl|qemu|native-selfhost)' || true
du -sh "$EVIDENCE_ROOT" || true
```

task-owned process、VM workdir、lock、temporary artifact は gate 後に回収する。shared VM、他 task の
worktree、active product artifact は所有者の確認なしに削除しない。runbook の手順だけでは ADR/TODO の
完了移行を行わず、要件ごとの evidence scope audit が通った項目だけを ADR に移す。

## 次の作業順

1. current-source の Rust/native report と Wasm/runtime evidence を target ごとに取得する。
2. V4-M1-01 の differential と evidence index が `pass` になるまで、未接続 boundary を `[~]` として保持する。
3. V4-M1-02 の module graph closure は、現在 active な parser/type/import worktree の反映後に、
   同じ fixture/evidence contract で着手する。
4. V4-M1-03〜05 は、GC/runtime、public command、release/rollback の所有 task と replay lock を確認してから
   一つずつ選ぶ。
