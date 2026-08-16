# ADR: Rust-free daily loop の立ち上げ (T1-1 / T1-2)

- Status: Accepted (verified slice)
- Date: 2026-08-16
- Scope: `scripts/lib/source-fingerprint.sh` (新規)、`scripts/ci/package-native-stage0.sh`、
  `scripts/native-selfhost-dev.sh`、`scripts/dev-loop.sh`、`AGENTS.md`、
  および fixture 側の追随 (`scripts/ci/test-native-selfhost-dev.sh`、
  `scripts/ci/test-native-stage0-package.sh`、`scripts/ci/test-native-selfhost-install-runner.sh`、
  `scripts/ci/test-dev-loop.sh`、`scripts/ci/test-package-native-linux-x86-actual-stage1-vm.sh`)
- Related: `AGENTS.md` の「Rust-free selfhost の進め方」、`LEGACY-MODULE-01`、`I-12`、
  [`decisions-v0.3-native-macos-stage0-producer.md`](decisions-v0.3-native-macos-stage0-producer.md)、
  [`decisions-dev-loop-rust-lane-speedup.md`](decisions-dev-loop-rust-lane-speedup.md)、
  [`rust-boundary-reduction.md`](../development/operations/rust-boundary-reduction.md)

## Context

Track 0 で待ち時間の原因 A (不要な Rust 再コンパイル) を潰した。本 ADR は、Rust-free daily loop が
そもそも日常運用に乗っていなかった原因を扱う。

`scripts/native-selfhost-dev.sh` の再利用ゲートは `manifest.source_commit == git rev-parse HEAD` の
一致のみを見ていた。これは同時に**厳しすぎ、かつ緩すぎた**。

実測 (本作業時点):

```
selfhost/src の最終変更: d6e0eab3 (2026-08-02)
git rev-list --count d6e0eab3..HEAD  ->  117
git diff --stat d6e0eab3..HEAD -- selfhost/src  ->  (空)
```

117 commit 積まれているのに `selfhost/src` はバイト単位で不変である。旧ゲートはこの 117 件すべて
(docs 修正、ADR、テスト追加…) が意味的に完全有効な stage0 を無効化していた。commit のたびに
stage0 再生成が必要になり、これが Rust-free lane が日常運用に乗らない直接原因だった。

同時に、`selfhost/src` を編集して未 commit の状態では HEAD が変わらないため、**dirty worktree が
素通り**していた。

`AGENTS.md` が要求していたのは元々 2 条件 —「producer/source commit **と** source fingerprint が
current checkout に一致し」— である。fingerprint 検証は契約が要求済みで未実装だった、というのが
正確な現状だった。したがって本変更は**契約を緩めるのではなく満たす変更**である。

## Decision

### T1-1 stage0 の一度きりのローカル生成

`scripts/fetch-stage0.sh` では入手できない。唯一の Release `v0.1.0-native-rc1` は 2026-05-11 で
`selfhost/src` が大きく乖離している。Mac Apple Silicon の正規 producer である
`scripts/ci/native-macos-aarch64-stage0-release.sh` を 1 度実行して stage0 を作る。

この producer は **clean worktree を要求する**。実行前に作業差分を commit しておくこと。

### T1-2 fingerprint による 2 lane 分離

| | strict lane (既定 / 証跡) | dev lane (明示 opt-in) |
|---|---|---|
| 起動 | 既定 | `--dev-reuse` / `NATIVE_ALLOW_FINGERPRINT_REUSE=1` |
| `source_commit` 一致 | **必須** | 不問 |
| `selfhost_src_fingerprint` 一致 | **必須 (新規)** | **必須** |
| field 欠落 | `die` | `die` |
| 不一致時 | `die` (fail-closed) | `die` (fail-closed) |
| 表示 | 通常 | stderr に `native-selfhost-dev: dev-reuse lane (source_commit mismatch tolerated: ...)` |
| 記録 | `.lane` を削除 | `$STAGE_DIR/.lane` に `dev-reuse` |
| 証跡採用 | 可 | **不可** |

strict lane は fingerprint 検証が加わることで**旧実装より強くなる** (dirty worktree の穴が塞がる)。
dev lane は「commit は違うが `selfhost/src` は同一」という、`AGENTS.md` の言う stale に実質当たらない
ケースだけを通す。

### fingerprint 実装の一本化

producer と consumer で算出が 1 バイトでもずれると全 stage0 が使えなくなる。実装を
`scripts/lib/source-fingerprint.sh` の `lsharp_source_fingerprint` に一本化し、
`package-native-stage0.sh` / `native-selfhost-dev.sh` / `dev-loop.sh` の 3 者が source する形にした
(`dev-loop.sh` は Track 0 で 3 つ目の複製を持っていたので、これも同時に畳んだ)。

アルゴリズム: `cd <src>` して `find . -type f -print | LC_ALL=C sort`、各 path について
`printf '%s  %s\n' <sha256> <path>`、その stream 全体の sha256。ソート順の locale 依存を避けるため
`LC_ALL=C` は固定である。

### manifest schema

`package-native-stage0.sh` が `selfhost_src_fingerprint` を出力する。既存 field は変更していない。

```json
{
  "kind": "lsharp-native-selfhost-stage0",
  "target": "aarch64-apple-darwin",
  "source_commit": "d87cd5d148bcdb6aa5005ec082357e87d7c1e746",
  "selfhost_src_fingerprint": "c3da0653841242b431bfb123a61332d36f966a542692f6b2c08a81f7703ccdc1",
  "compiler": "bin/compiler",
  "transport_driver": "bin/transport-driver",
  "materializer": "bin/materializer"
}
```

`scripts/fetch-stage0.sh` は `kind` / `source_commit` / path のみを見るため、field 追加の影響を受けない。

## Evidence

### T1-1 stage0 生成

`scripts/ci/native-macos-aarch64-stage0-release.sh` を `d87cd5d1` の clean worktree で 1 回実行。

| 指標 | 実測 |
|---|---|
| stage0 e2e 全体 | **927.92s** |
| 既存 ADR の記録値 | 484.89〜542.31s |

記録値の約 1.7〜1.9 倍かかった。producer の記録レンジは Mac Apple Silicon でも再現しないことがある、
という新しいデータ点として残す。原因は未特定 (並行負荷の可能性)。

生成物: `ci-artifacts/native-stage0/aarch64-apple-darwin/current`。

### T1-2 RED -> GREEN

`scripts/ci/test-native-selfhost-dev.sh` に RED-1〜8 を追加。実装前は

```
FAIL: dev lane rejected a stage0 whose source fingerprint matches the checkout
```

で RED-1 が落ちることを確認した。実装後は全 lane が GREEN。

| 検証 | 結果 |
|---|---|
| `bash scripts/ci/test-native-selfhost-dev.sh` | `native selfhost dev runner tests: OK` |
| `bash scripts/ci/test-native-stage0-package.sh` | `native stage0 package tests: OK` |
| `bash scripts/ci/test-native-selfhost-install-runner.sh` | `native selfhost install runner tests: OK` |
| `bash scripts/ci/test-dev-loop.sh` | `PASS` |
| `bash scripts/ci/test-package-native-linux-x86-actual-stage1-vm.sh` | `Linux x86 actual-stage1 stage0 package tests: OK` |
| `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` | `Linux native stage0 source-file provenance tests: OK` |
| `NATIVE_STAGE0_DIR=<abs>/current bash scripts/ci/native-selfhost-dev-source-file-smoke.sh` | exit 0 (**実 strict lane で新 fingerprint 検証を通過**) |
| `test-native-macos-aarch64-stage0-release` / `test-native-stage0-release-package` / `test-native-selfhost-source-file-smoke-evidence` / `test-decode-native-selfhost-transport` / `test-fetch-stage0-atomic-install` / `test-fetch-stage0-archive-provenance` / `test-native-official-release-snapshots` / `test-native-linux-x86-source-smoke-replay-lock` / `test-native-linux-x86-source-smoke-evidence-copy` | 全て exit 0 |

producer と consumer が同一値を出すことを実測で確認した (`selfhost/src` に対して両者とも
`c3da0653841242b431bfb123a61332d36f966a542692f6b2c08a81f7703ccdc1`)。

### 実 stage0 での lane 検証 (harness ではなく実物)

commit `ccfe4efc` (stage0 の `source_commit` は `d87cd5d1`、`selfhost/src` は不変) の状態で、
実 stage0 に対して両 lane を通した。入力は小さい fixture
`tests/fixtures/validation/ec-m3-canonical-source.ls` を使う (`selfhost/src/App/Cli.ls` は `I-12` の
segfault を踏むため結果が混ざる)。

strict lane — 設計どおり `die` する:

```
$ bash scripts/native-selfhost-dev.sh --stage0-dir <current> \
    check tests/fixtures/validation/ec-m3-canonical-source.ls
error: stage0 manifest source_commit does not match current checkout: manifest=d87cd5d1... checkout=ccfe4efc...
[exit 1]
```

dev lane — 成功し、marker と `.lane` が記録される:

```
$ bash scripts/native-selfhost-dev.sh --dev-reuse --stage0-dir <current> \
    check tests/fixtures/validation/ec-m3-canonical-source.ls
native-selfhost-dev: dev-reuse lane (source_commit mismatch tolerated: manifest=d87cd5d1... checkout=ccfe4efc...)
Bool
diagnostics:0
[exit 0]   6.62s

$ cat <stage-dir>/.lane
dev-reuse
```

**927.92s の stage0 再生成が 6.62s の stage bootstrap に置き換わった**のが、本 slice が実際に短縮した
待ち時間である。ただし短縮の対象は「`selfhost/src` を触らない commit のあと」に限られる (下記 Consequences)。

### fixture 側の追随が必要だった箇所

manifest field を必須にしたため、stage0 manifest を偽造して runner を起動する harness が壊れた。
実測で検出し、いずれも「契約に合わせる」方向で修正した (期待値を緩めていない)。

- `test-native-stage0-package.sh` — manifest の完全一致 assert。fixture source tree を
  `NATIVE_STAGE0_SELFHOST_SRC` で固定し、期待 fingerprint を共有実装で算出する形にした。
  併せて「source tree 不在なら `die`」の reject case を追加した。
- `test-native-selfhost-install-runner.sh` — fixture に `scripts/lib/` が無く runner が起動不能だった。
  lib の copy と manifest への fingerprint 追加。
- `test-dev-loop.sh` — 同上 (`dev-loop.sh` の一本化に伴う)。
- `test-package-native-linux-x86-actual-stage1-vm.sh` — package 時の fingerprint 対象と runner の
  `--source-root` が別 tree だったので、packaging 側を runner fixture に合わせた。

## Consequences

### 得られたもの

`selfhost/src` を触らない commit (docs / scripts / Rust / ADR) では、stage0 を再生成せずに
dev lane で日常作業を続けられる。旧ゲート下で無効化されていた 117 commit 相当のケースがこれに当たる。

strict lane は dirty worktree の穴が塞がり、旧実装より強くなった。

### 得られなかったもの — source 編集ループは速くなっていない

fingerprint を必須にした結果、**`selfhost/src` を編集すると strict lane / dev lane の両方が `die` する**。
dev lane が救うのは「commit は進んだが `selfhost/src` は同一」のケースだけである。

したがって「L# のソースを編集して即座に試す」ループの待ち時間は本 slice では変わっていない。
これを消すのは `LEGACY-MODULE-01` (selfhost module cache、Track 1 の T1-3) であり、数週間規模の作業になる。
計画上もこの順序 (A -> C -> Rust 脱却 -> B) を採っており、本 slice はその途中段階である。

### strict lane smoke の扱い

`scripts/ci/native-selfhost-dev-source-file-smoke.sh` は strict lane を使う。stage0 を生成した commit から
HEAD が進むと**設計どおり失敗する**。これは regression ではない。証跡を取り直すときは、その HEAD で
stage0 を再生成してから実行する。`AGENTS.md` にも明記した。

### 運用上の落とし穴 (実際に踏んだもの)

- **producer の既定出力が自身の gate を壊す。** `ci-artifacts/native-stage0/` と
  `.native-selfhost-dev/` は untracked かつ未 ignore だったため、1 度実行すると次回の
  clean worktree gate に引っかかって producer を二度と実行できなくなる。両方を `.gitignore` に追加した。
- **stage0 の再 package は full 再生成なしで済む。** manifest field 追加だけなら、既存 binary に対して
  `package-native-stage0.sh` を `--output-dir current-repack` で再実行し、`mv` で差し替えればよい
  (約 900s の再生成が不要になる)。旧 package は `current.pre-fingerprint` として rollback 用に残した。
- **`check selfhost/src/App/Cli.ls` が segfault する** (exit 139)。stage0 の bootstrap 自体は成功し、
  小さい fixture を使う documented smoke も通るため、stage0 は有効である。入力サイズ/内容に依存する
  別問題として `I-12` に登録した。本 slice のスコープ外。
