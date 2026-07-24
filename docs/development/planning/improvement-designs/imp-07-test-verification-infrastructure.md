# imp-07: テスト・検証基盤の強化 (fuzz / 限界値 / rooting 規約 / カバレッジ再配分)

> 対象 issue: [I-06](../../../../ISSUES.md#i-06) (fuzz・リーク・限界テスト欠落)、
> [I-07](../../../../ISSUES.md#i-07) (rooting 修正の頻発)、[I-08](../../../../ISSUES.md#i-08) (テストカバレッジの偏り)
> ロードマップ: [improvement-roadmap.md](../improvement-roadmap.md) Phase B-4 / D-3 / D-4

## 現状の正確な把握 (2026-06-12 コード検証済み)

- **ランダム化テスト**: ワークスペースに proptest / quickcheck / cargo-fuzz / arbitrary の
  依存・ターゲット定義は存在しない
- **既存の検証資産**: insta スナップショット (IR/型出力)、criterion ベンチ
  (`crates/lsharp-wasm/benches/compiler_pipeline.rs`)、GC メトリクス契約テスト +
  CI artifact (`ci-artifacts/gc-metrics/`)、incremental ベンチ
  (`crates/lsharp-wasm/tests/e2e/incremental_benchmark.rs`)
- **rooting**: selfhost コードの GC rooting は `root_push` / `root_pop` / `root_set`
  (生成コード内ランタイム関数) を手動で呼ぶ規律で支えられている。
  root 漏れの retainer として CP-05 G3 の 4 guard test (G3-a〜d、
  `runtime-stability-spec.md` の G3 節) が存在するが、**規約の明文化はない**。
  直近履歴では defn body rooting 修正が反復している (dbdd448, 93074c8, 9c40998)
- **偏り**: テストの大半が lsharp-wasm の E2E (selfhost stage chain 系で数万行規模)。
  syntax/types はインラインテスト主体 (infer.rs の 30%、ir/lib.rs の 39% がテスト)

## 設計

### 1. Property-based テストの導入 (I-06、Phase D-3)

cargo-fuzz (libFuzzer) は CI 常設に向かないため、**proptest を dev-dependency で導入**する:

1. lsharp-syntax: 「任意のトークン列を parse してもパニックしない」
   「pretty-print → re-parse の往復で AST が一致する (roundtrip)」
2. lsharp-types: 「生成した小さな式の型推論がパニック・無限ループしない」
   「unify(a,b) と unify(b,a) の成否一致 (対称性)」
3. 入力生成は AST ジェネレータ (proptest の `Strategy` で深さ制限付き) を
   `crates/lsharp-syntax/src/test_gen.rs` (cfg(test) 限定) に置き、types からも再利用する
4. CI: 通常 PR では proptest のケース数を小さく (例: 64)、
   nightly / 手動ジョブで大きく (例: 4096) 回す 2 段構成

### 2. 限界値・リークテスト (I-06、Phase D-3)

imp-03 のテスト戦略と共有する:

- GC スロット上限 (4096 / 32768) 到達 E2E (imp-03 §4 の RED テストがそのまま限界値テスト)
- 再帰深度: 自己再帰関数の深さを段階増加させ、wasmtime の stack 制限での
  失敗挙動 (trap メッセージ) を E2E で固定し、限界値を `docs/development/validation/` に記録
- リーク検出: 「alloc → 不要化 → `__lsharp_gc_collect` 強制実行」のループを N 回回し、
  `gc_live_alloc_count` が定常に戻ることを契約テスト化 (既存メトリクス export を利用)
- occur check 性能: 深いネスト型 (`List (List (... Int)))` 32/64/128 段) と
  巨大レコード (128/256 フィールド) の推論時間を criterion ベンチへ追加し、
  超線形な悪化があれば別 issue 化する

### 3. GC rooting 規約の明文化と guard 拡張 (I-07、Phase B-4)

1. **規約文書**: `docs/development/planning/memory-management-roadmap.md` (GC 正本) に
   「selfhost rooting 規約」節を追加する:
   - heap 値 (Vector / String / レコード) を**割り当てを起こしうる呼び出し**を跨いで
     保持する場合、跨ぐ前に `root_push`、使用後に `root_pop` する
   - ループ内で更新される heap 値は `root_set` でスロットを更新する
   - 「割り当てを起こしうる呼び出し」の判定が困難な場合は保守的に root する
   - 違反の典型例として直近の defn body rooting 修正 (dbdd448 等) を事例として記録
2. **guard 拡張**: 既存 CP-05 G3 の 4 guard test (root 漏れ retainer) に、
   修正が反復した箇所 (selfhost parser の defn body、x86 の register state /
   string literal data ref) を対象とする回帰テストを追加する。
   新しい rooting バグを修正するたびに guard を 1 本足す運用 (回帰の蓄積) を規約に含める
3. **診断補助** (任意): GC に「collect 時に root 経由で到達できなくなった直後の
   オブジェクトへアクセスしたら fail する debug モード」(alloc 毎に collect を強制する
   stress モード) を追加できると、rooting バグの再現が決定的になる。
   wasi.rs の `__alloc` 入口に「テスト時のみ毎回 `__gc_collect` を呼ぶ」フラグを
   emit するのが最小実装

### 4. テストカバレッジ再配分 (I-08、Phase D-4)

- 方針: E2E を減らすのではなく、**E2E が落ちたときに原因レイヤを単体で再現できる**
  ユニットテストを syntax / types / ir に増やす
- E2E の失敗事例 (過去の rooting バグ、parser バグ) から逆引きして、
  各レイヤの最小再現ユニットテストを起こす (失敗駆動でテストを増やす)
- インラインテストの分離は imp-06 §1 の規則 (tests.rs 分離) に従う
- 機械検査: `scripts/test-distribution.py` でクレートごとの Rust test attribute/function、proptest macro、ignore 数を
  deterministic な TSV/JSON として出力し、`scripts/ci/test-test-distribution.sh` で schema・crate 集合・出力安定性を契約検査する。
  目標値は設けず、偏りの推移を可視化する。

## 影響範囲

- proptest は dev-dependency のみで配布物に影響しない
- stress モード (3-3) は wasi.rs の emit 変更を伴うため imp-03 と調整して実施する

## 検証済み部分実装 (2026-07-24)

`lsharp-syntax` と `lsharp-types` の dev-dependency に `proptest` を追加し、次の bounded property test を 64 cases で固定した。

- `lsharp-syntax`: arbitrary bytes（0〜127 bytes）を入力して parser が panic せず `Ok` または parse/lex error を返す。
- `lsharp-types`: 深さ・要素数を制限した `Type` を生成し、`unify(a, b)` と `unify(b, a)` の成否が一致する。
- `lsharp-types`: 深さ・要素数を制限した expression source を `defn main` に包み、parser → type inference を 64 cases 実行して panic しない。
- `lsharp-types` integration contract: self-application `(defn omega [f] (f f))` は occurs check により `InfiniteType` / `LS1003` を返す。
- `lsharp-types` integration limit contract: `Box` を 32 / 64 / 128 段ネストした型注釈を parse → inference しても panic せず成功する。
- `lsharp-types` integration limit contract: `Int` フィールド 128 / 256 個の `Wide` レコード型注釈を parse → inference しても panic せず成功する。
- `lsharp-wasm` actual runtime contract: 4097 個の unrooted allocation の回収、free-list 再利用、10 回 repeated-start の heap plateau を telemetry で確認する。

さらに `lsharp-syntax` に深さ 3・要素数 bounded の式 generator を追加し、literal/variable/if/let/lambda/application/do/annotation/record/quote の
pretty-print → re-parse 安定性を 64 cases で固定した。full crate gate 中に `"\\` と不正 UTF-8 置換文字の組み合わせが lexer の char boundary panic を検出したため、
未知 escape の非 ASCII code point を一文字分消費する修正と regression seed を追加した。

これらは arbitrary input/type の panic regression、unification の成否対称性、深い型注釈・巨大レコードの限界値、GC leak / free-list の actual runtime、限定した AST roundtrip に対する verified slice であり、
小さな式全体の型推論、nightly 4096 cases、GC slot 32768、runtime memory.grow 上限、rooting stress、native stage0 の GC gate は未着手のまま残る。property test は dev-dependency と `cfg(test)` に閉じ、
配布 artifact の依存関係や公開 CLI の挙動を変更しない。

## ステータス

設計 + parser panic-safety / type-unify symmetry / bounded inference / occurs-check / syntax roundtrip / test-distribution verified slice (2026-07-24)。着手時は TODO.md に Phase B-4 / D-3 / D-4 として項目を作成する。
`LEGACY-TEST-01` aggregate の完了条件は満たしていない。
