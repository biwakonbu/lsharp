# ADR: metadata directive allowlist の parity test

- Status: Accepted (verified slice)
- Date: 2026-08-18
- Scope: `PARSER-PARITY-01` / `I-18` / `crates/lsharp-syntax/tests/metadata_directive_parity.rs`
- Related: [`ISSUES.md` I-18](../../ISSUES.md#i-18)、
  [`decisions-root-lifetime-intentional-imbalance-annotation.md`](decisions-root-lifetime-intentional-imbalance-annotation.md)

## Context

`:` で始まる metadata directive を受理するかの判定表が手書きで複数箇所に存在し、
一致を検査するものが無い。片方だけに directive を足すと、同じソースが front end によって
通ったり落ちたりする。しかも directive でない `:` は戻り値型注釈として読まれるため、
食い違いは「未知の directive」ではなく**型注釈の parse error**として現れ、原因が読み取りにくい。

`I-18` は sync point を 2 つ (Rust の `is_colon_directive` と selfhost の
`directive-symbol-v3`) と記録していたが、本スライスの調査で **3 系統**あることが分かった。
`crates/lsharp-syntax/src/parser/metadata.rs` の `try_parse_metadata` は「受理するか」を
判定する `is_colon_directive` とは別に「どう読むか」の分岐を持ち、これも独立した一覧である。
実測値は Evidence 節に置く。

## Decision

**`crates/lsharp-syntax/tests/` に integration test を置き、3 系統の allowlist を
すべて text 抽出して比較する。ただし比較するのは「集合の完全一致」ではなく
「ペアごとの差分が既知の集合と一致すること」とする。**

### 完全一致を assert しない理由

3 つの一覧は**正しく運用していても一致しない**。

- `where` / `constraints` は lexer が専用トークン (`TokenKind::Where` /
  `TokenKind::Constraints`) へ落とすため、`is_colon_directive` の生きた判定経路は
  `Some(TokenKind::Where)` 側の腕であり、`matches!` の中の文字列腕は**到達しない死んだ枝**である。
  `try_parse_metadata` は `Some(TokenKind::Symbol(_))` しか見ないので、この 2 つが
  そちらに無いのは**正しい**。
- したがって集合の完全一致を要求する test は、実装が正しいまま赤くなる。

完全一致でなく差分を pin する形にすると、以下が同時に成り立つ:

- **新しい divergence は検出される** — どちらか片方にだけ directive を足せば、
  期待差分集合に無い名前が現れて落ちる。
- **正しく両側に足したときは落ちない** — 一覧そのものを test に転記しないので、
  directive 追加のたびに test を書き換える必要が無い。

### 抽出は両側とも text で行う

Rust 側だけを behavioral (実際に parse させる) にする案は採らない。behavioral 比較は
「Rust だけに存在する名前」を列挙できず、片翼の検査にしかならないためである。
selfhost 側は Rust から呼べない以上どのみち text 抽出になるので、両側を text 抽出へ揃える。

`selfhost/src/Syntax/Parser.ls` の読み出しには `include_str!` を**使わない**。
`env!("CARGO_MANIFEST_DIR")` + `fs::read_to_string` を使う。理由は下記「配置」。

### 配置

`crates/lsharp-syntax/tests/` に置く。同 crate には `selfhost/` のソースを実行時に読む
先例 (`selfhost_cli_validation_contract.rs`) が既にある。

`crates/lsharp-wasm/tests/e2e/` に置く案は却下する。同 test 群は `support.rs` が
selfhost の全モジュールを `include_str!` しており、`.ls` を 1 文字触るだけで e2e binary 群が
まるごと再コンパイルされる。`include_str!` を避けるのも同じ理由で、
compile-time 依存を作ると `.ls` の編集が Rust の再ビルドを誘発する。

## 却下した選択肢

**案 A — `:roots-unbalanced` を selfhost parser へ即座に port する。却下。**
divergence 1 件は消えるが、**再発を防ぐものが何も残らない**。加えて `selfhost/src` の編集は
`lsharp-driver/build.rs` の `rerun-if-changed` (cache key roots `["selfhost/src", "stdlib", "wit"]`)
を発火させ、embedded component の再ビルドを巻き込む。`I-18` が指摘している実害そのものを
検査を置く前に自分で踏むことになる。**parity test が先**である。

**案 B — 一覧を単一正本 (data file か片方からの生成) へ寄せる。却下 (このスライスでは)。**
筋としては正しく、`I-18` の「直し方の方向」もこれを挙げている。ただし正本化は
lexer の予約語経路 (`where` / `constraints`) と selfhost の payload 処理の差異を
先に整理しないと設計できない。**parity test が無いまま正本化すると、寄せ損ねを検出する手段が無い。**
`TODO.md` の `PARSER-PARITY-01` も正本化を「含めない範囲」に明記している。

**案 C — 3 系統の完全一致を assert する。却下。**
上記のとおり `where` / `constraints` で偽陽性になる。偽陽性を避けるために
死んだ文字列腕を削除する案もあるが、それは本スライスの範囲を越えたリファクタで、
「検査を置く」ことと「実装を整理する」ことを混ぜる。**死んだ腕は記録だけして残す。**

**案 D — 一覧を test 側に転記して assert する。却下。**
directive を足すたびに test の転記も直す必要があり、転記漏れが偽の赤を生む。
検査したいのは「一覧の中身」ではなく「3 系統がずれていないこと」である。

## 受入条件との差

`TODO.md` の `PARSER-PARITY-01` は受入条件を「両者の一覧の一致を検査する test を置くこと」と
書いている。本 ADR が置く test は**一致ではなく差分を検査する**ので、文言どおりには満たさない。

意図 (新しい divergence を検出できる状態にする) は満たしていると判断する。根拠は
「完全一致を assert しない理由」節のとおり、完全一致は実装が正しいままでも成立せず、
受入条件の文言が前提にしていた「3 系統は一致するはず」という認識自体が実測で否定されたためである。
**この判断ごと記録して受入条件を書き換える。**

## Evidence

すべて 2026-08-18、worktree `codex/spec-and-parser-parity` (base `2c2a50e4`) での実測。

**3 系統の実測件数** — `directive_allowlist_sizes_are_pinned` が pin している。

| 系統 | 件数 |
|---|---|
| `decl.rs` の `is_colon_directive` | 29 |
| `metadata.rs` の `try_parse_metadata` | 27 |
| selfhost の `directive-symbol-v3` (+ `source-` 版) | 28 |

**ペアごとの差分** — 実測は期待どおり 2 組 3 件。

| 差分 | 実測 | 判定 |
|---|---|---|
| `decl` − `selfhost` | `{roots-unbalanced}` | `I-18` の唯一の意味論的 divergence |
| `selfhost` − `decl` | ∅ | -- |
| `decl` − `metadata` | `{where, constraints}` | 予約語トークン経路の構造差。divergence ではない |
| `metadata` − `decl` | ∅ | -- |

- **RED**: 3 test とも**当初から緑**。これは意図どおりで、本 test は現状の差分を固定するもの
  だからである (先例: [main exit 免除 ADR](decisions-root-lifetime-main-exit-exemption.md) の
  「拒否側の現状を固定するテスト」2 本)。**緑が空虚でないことは変異で実証した**。

  | 変異 | 結果 |
  |---|---|
  | `decl.rs` の `matches!` にだけ `"fake-directive-parity-probe"` を追加 | **3 test すべて FAILED**。`decl` − `selfhost` が `{fake-directive-parity-probe, roots-unbalanced}`、`decl` − `metadata` が `{constraints, fake-directive-parity-probe, where}`、件数が 29 → 30 |
  | `ARM_INDENT` を 24 → 25 桁へ変更 (抽出を破壊) | `assert_extraction_alive` が発火し「抽出結果が 0 件しかない (最低 25 件を期待)。これは allowlist の divergence ではなく、抽出パターンが…」で FAILED |

  変異はいずれも実測後に復元し、`git diff --stat -- crates/lsharp-syntax/src/parser/decl.rs` が
  空であることを確認した。

- **GREEN**: `cargo test -p lsharp-syntax --test metadata_directive_parity` → **`3 passed; 0 failed`**。
  crate 全体 `cargo test -p lsharp-syntax` → 各 target 緑で、唯一の FAIL は
  `selfhost_cli_validation_contract::selfhost_cli_validation_surface_is_registered`。
  これは `workspace-expected-failures.txt:135` に登録済みの既知 FAIL (upstream 由来) で、
  本変更とは無関係である (working tree の差分は新規 2 ファイルのみ)。
- **lint**: `cargo clippy -p lsharp-syntax --all-targets` → 警告 0 件。
- **副産物**: selfhost が受理する 28 件のうち `parse-defn-metadata-step-v3` が実際に読むのは
  22 件で、6 件 (`where` / `rationale` / `since` / `see-also` / `transitions` / `constraints`) は
  `skip-directive-payload-v3` へ落ちて payload が捨てられていることが分かった。
  本 test の範囲外なので `ISSUES.md` の `I-20` として起票した。

### 満たせなかった受入条件

- **`TODO.md` の文言「両者の一覧の一致を検査する test」は満たしていない。** 上記
  「受入条件との差」節の判断による。一致ではなく差分を検査する。
- **二重管理そのものは解消していない。** 一覧は 3 系統 (`I-20` の
  `source-metadata-form-kind-v3` を数えれば 4 系統) のまま残る。案 B の正本化は別スライス。
- **`:roots-unbalanced` の selfhost への port は行っていない。** 案 A の却下理由のとおりで、
  `I-18` の divergence 1 件は開いたまま、test がそれを既知として固定している状態である。

## Consequences

directive の片側追加が test で落ちるようになる。一方これは**二重管理そのものの解消ではない**。
一覧は 3 系統のまま残り、正本化 (案 B) は別スライスに残る。
また本 test は allowlist だけを見るので、payload の読み方の差異 (selfhost 側で
payload が捨てられている等) は検出しない。そちらは `ISSUES.md` の別項目が持つ。
