# ADR: Rust lane dev loop の待ち時間短縮 (T0-1 〜 T0-5)

- Status: Accepted (verified slice)
- Date: 2026-08-16
- Scope: `Cargo.toml` の `[profile.dev]` / `[profile.test]`、`.cargo/config.toml` (新規)、
  `scripts/dev-loop.sh` (新規)、`scripts/ci/test-dev-loop.sh` (新規)、
  `crates/lsharp-driver/build.rs`、`crates/lsharp-wasm/src/embedded_component_cache.rs` (新規)
- Related: `I-05` (Rust host cache)、`LEGACY-MODULE-01`、`d8cd6376`、
  [`rust-boundary-reduction.md`](../development/operations/rust-boundary-reduction.md)

## Context

「開発の待ち時間が長い。早く Rust を脱却して L# だけで開発したい」という要求に対し、待ち時間を
3 つの独立な原因へ分解した。

| 原因 | Rust 脱却で解決するか |
|---|---|
| A. 不要な Rust 再コンパイル | する |
| B. L# コンパイラ自身のスループット | しない (むしろ悪化する) |
| C. selfhost lane の差分ビルド欠如 (`LEGACY-MODULE-01`) | しない (Rust 脱却の前提条件) |

キャッシュは現時点で Rust lane のほうが進んでいる。`compile_multi_file_with_cache` と
`ArtifactCache` は Rust 側に実装・検証済みだが selfhost 側に対応物が無い。**いま Rust を捨てると
待ち時間は増える。** 本 ADR は A のみを対象とする。C は `LEGACY-MODULE-01` の残件として TODO に残す。

また [`adr-rust-removal.md`](../development/operations/adr-rust-removal.md) が Rust workspace の
物理削除を withdrawn としており、`crates/` は bootstrap / oracle / rollback 境界として恒久保持する
方針である。したがってこの投資は「いずれ捨てる場所への投資」ではない。

## Decision

### T0-1 `incremental = false` の撤回

`[profile.dev]` / `[profile.test]` の `incremental = false` を `true` へ戻し、`debug` を
`"line-tables-only"` へ落とし、`[profile.dev.package."*"] debug = false` を追加する。

撤回して良い根拠: 導入コミット `d8cd6376` ("chore: cap local build and VM replay storage",
2026-07-17) の動機はディスク圧迫のみで、検証項目は `cargo metadata` / `limactl validate` /
`bash -n` / `git diff --check` の 4 つ。determinism や snapshot に関する根拠は無い。insta snapshot
は実行時出力のハッシュなので incremental の有無に影響されない。ディスク削減の意図は依存 crate の
debuginfo 除去で肩代わりする (`cargo clean` で消えた 77.1GiB の主因は wasmtime/cranelift の
debuginfo)。

同コミットが VM replay script へ入れた `CARGO_INCREMENTAL=0` は**触らない**。使い捨て VM の
ディスク上限対策であり意図が異なる。

### T0-2 sidecar による L# 編集ループ

`selfhost/src` の編集を確認するのに `cargo build` を待たない。driver は実行ファイルの隣の
`<stem>.component.wasm` を embedded component より優先して読む
(`resolve_default_component_bytes` / `adjacent_component_sidecar_path_for_executable`)。この sidecar を
差し替える `scripts/dev-loop.sh` を追加する。契約テストは `scripts/ci/test-dev-loop.sh`。

- 生成先は `.lsharp-dev/bin/` とし、`target/debug/` には**置かない**。`target/debug/lsharp` の隣に
  sidecar を置くと、その binary を exec する driver 系 integration test の挙動が黙って変わる。
- `.cargo/config.toml` へ `[env] LSHARP_EMBED_COMPONENT_PATH` を置く案は採らない。repo 全体に
  stale component が静かに効き、build.rs の `rerun-if-changed=selfhost/src` と競合する。
- **これは Rust lane の効率化であって Rust-free lane ではない。** 成功経路に cargo-built driver を
  使うため、この loop の結果を native gate の evidence に数えない。

### T0-3 build.rs の content-addressed cache 化

`build_default_embedded_component()` が呼んでいた非キャッシュ版を、`lsharp-embed-component-v1`
envelope による content-addressed cache 経由に変える。key は (sorted source fingerprints,
emitter fingerprint)。ロジックは build.rs から
`crates/lsharp-wasm/src/embedded_component_cache.rs` へ抽出してテスト可能にする。

### T0-4 `.cargo/config.toml` は alias のみ

linker 設定は入れない。mold は Mach-O 非対応、zld は開発終了 (作者自身が ld-prime を使えと表明)、
lld の Mach-O backend は Apple ld-prime に対し明確な優位が無い。`[build] rustflags` も CI と
食い違うと全再コンパイルを誘発するため入れない。

### T0-5 cargo-nextest

導入するが、**nextest はコンパイルを速くしない**。実行のみが並列化される。doctest は扱わないので
`cargo test --doc` を別途残す。`[[test]]` harness の統合は行わない — リポジトリは
`decisions-legacy-*-test-split.md` 系 ADR と 500〜800 行ルールに従って意図的にテストを分割してきた
経緯があり、統合は規約に逆行する。

### 却下したもの

| 施策 | 却下理由 |
|---|---|
| wasm toolchain 三重化の解消 | 0.221 系は wasmtime 29 自身が引いている。`wit-parser` を 0.245 へ上げることは wasmtime-wit-bindgen 29 の API 制約でできない。将来の wasmtime メジャーアップグレードにぶら下げる |
| sccache | rustc 呼び出しはキャッシュするが build.rs の実行はキャッシュしない (本件の重い部分に効かない)。かつ incremental compilation と併用できず T0-1 と競合する |

## Evidence

計測環境は Mac Apple Silicon、dev profile。詳細は
[`rust-boundary-reduction.md`](../development/operations/rust-boundary-reduction.md) の
「Rust lane dev loop の待ち時間短縮 (2026-08-16)」節。

| 項目 | before | after |
|---|---|---|
| T0-1 warm rebuild (`wasi/mod.rs` に空行 1 行) | 42s | 7s (cold 161s / noop 0s) |
| T0-3 build.rs component 生成 | MISS 1m46.5s | HIT 4.2s (約 25 倍) |
| `-p lsharp-driver --test default_path_delegation` | 803.99s | 304.92〜389.00s |
| `cargo nextest run -p lsharp-ir --lib` | — | 291 passed / 107.871s |

- `scripts/ci/test-dev-loop.sh` は RED → GREEN。fingerprint 一致時の no-op、1 ファイル touch 時の
  1 回だけの再生成、生成先が `target/debug/` 配下でないことを固定する。
- `crates/lsharp-wasm/src/embedded_component_cache_tests.rs` は RED → GREEN。store→lookup hit、
  source 1 バイト変更での key 変化、envelope digest 破損時が `Err` でも stale hit でもなく miss で
  あることを固定する。
- 実測で判明した副作用: `lsharp compile` は entry file を canonical 整形して**書き戻す**
  (`prepare_source_for_compile`、契約テストで固定された仕様)。素通しすると毎回 `selfhost/src` が
  dirty になり `rerun-if-changed=selfhost/src` が発火する。`scripts/dev-loop.sh` は compile 前に
  entry を退避し、復元後の tree fingerprint が一致しなければ fingerprint を記録せず `die` する。
- `cargo nextest run -p lsharp-ir --lib` の壁時計 107.8s はほぼ全量が単一 test
  (`incremental_analysis_tests::test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds`)
  である。

### workspace 検証の結果と、満たせなかった受入条件

計画の受入条件 3「`cargo test --workspace` が全 GREEN であること」は**文言どおりには満たしていない**。
97 件が FAIL する。ただしこの 97 件は pristine な `a3ae4551` で同一に再現する pre-existing failure
であり、test 名の集合 `diff` は空、pass/fail 件数と snapshot の byte diff も一致する。したがって
受入条件の意図 (T0 の変更が挙動を変えないこと) は満たしている。T0-1 / T0-2 / T0-3 は免責される。

この 97 件は台帳未記載だったため、本 slice で `I-10` として ISSUES.md へ登録した。

なお `cargo test --workspace --doc` が 8 crate ではなく 7 crate を報告するのは、`lsharp-driver` が
`[[bin]]` のみで lib target を持たず cargo が Doc-tests を生成しないためであり、正常挙動である。

## Consequences

Rust lane の編集→確認ループが実測で短縮され、`selfhost/src` の編集は cargo を起動せずに確認できる
ようになった。一方で本 slice は原因 A のみを解消したものであり、B (L# コンパイラのスループット) と
C (selfhost の差分ビルド) は未着手のままである。**C が閉じるまで「Rust 脱却で待ち時間が減る」は
成立しない。** 正しい順序は A を潰す → C を実装する → Rust 脱却 → B を継続改善であり、Track 1
(T1-1 stage0 一度きり生成 / T1-2 fingerprint dev lane / T1-3 selfhost module cache) を TODO に残す。

incremental 有効化により `target/` のディスク使用量は増える。debuginfo 縮小で相殺する設計だが、
実測での定点観測はしていない。`cargo sweep` (= `cargo clean --profile dev`) を alias として用意した。
