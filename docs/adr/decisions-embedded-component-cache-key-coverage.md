# ADR: embedded component cache key の入力被覆

- Status: Accepted (verified slice)
- Date: 2026-08-18
- Scope: `EMBEDCACHE-01` / `I-16` / `crates/lsharp-driver/build.rs` /
  `crates/lsharp-wasm/src/embedded_component_cache.rs`
- Related: [`ISSUES.md` I-16](../../ISSUES.md#i-16)、
  [`decisions-default-path-smoke-determinism.md`](decisions-default-path-smoke-determinism.md)

## Context

embedded component cache は selfhost tree の再コンパイル (miss 1m46s) を hit 4.2s に落とす
最適化で、key は `selfhost/src` 全ファイルの fingerprint と build script 実行ファイルの
fingerprint から導かれる。

しかし埋め込む component の生成入力はそれだけではない。詳細と実測は `I-16` が正本だが、
本 ADR の判断に効く事実は 3 つある。

1. `wit/` は**実入力**である。`emit_wasm_wasi_p2` が `wit/lsharp-compiler.wit` を読み、
   `resolve_world` が `wit/deps` を含む workspace を解決する。
2. `stdlib/` は**潜在入力**である。module 解決の探索パスに載っているが、現 tree では
   56 モジュールすべてが `selfhost/src` 配下で解決するため 1 ファイルも読まれていない。
3. **「build script を再実行させる集合」と「key に入る集合」が別々に書かれている。**
   `wit/` は前者だけ、`selfhost/src` は両方、`stdlib/` はどちらにも無い。
   この食い違いを検出する仕組みが無く、現に食い違っている。

3 が本質である。1 と 2 は 3 の症状にすぎず、片方だけを塞いでも同じ形の欠落が再発する。

## Decision

**入力 root の一覧を単一の定数にし、key 導出と `rerun-if-changed` の両方をそこから導く。**

`crates/lsharp-wasm/src/embedded_component_cache.rs` に次の 2 つを置く。

- `EMBEDDED_COMPONENT_KEY_ROOTS` — project root からの相対 path の配列。
  `["selfhost/src", "stdlib", "wit"]`。
- `embedded_component_key_sources(project_root)` — 上記 root を label 付きで走査し、
  連結した `(label/相対 path, fingerprint)` の列を返す純関数。

`build.rs` は次の 2 箇所でこの定数を使う。key 導出は `embedded_component_key_sources` を
呼ぶだけにし、`rerun-if-changed` は `EMBEDDED_COMPONENT_KEY_ROOTS` を回して出す。
これにより両者は**構造的に**一致し、root を足し忘れる余地が消える。

副次的に `cargo:rerun-if-changed=<root>/stdlib` が新たに出るようになる (現在は欠落)。

### 導出を `lsharp-wasm` 側に置く理由

`embedded_component_cache.rs` の module doc が既に
「key 導出と cache root の逆算は `lsharp-wasm` 側に置いてある (build.rs は直接テストできない)」
と書いている。root の選択も key 導出の一部なので、同じ場所に置くのが既存の設計意図に沿う。
build.rs 側に置くと、今回追加する判断がテストの届かない場所に残る。

### ディレクトリを拡張子で絞らない理由

`stdlib/api.json` や `wit/README.md` は生成物 / ドキュメントであり、component の中身には
効かない。絞れば false invalidation を減らせるが、絞り込みルールは resolver の実装と
二重管理になる。**over-invalidate は遅いだけ、under-invalidate は誤った bytes を埋め込む。**
損失が非対称なので保守側に倒す。実コストも小さい (`selfhost/src` 3.5M に対し
`stdlib` 96K + `wit` 248K で約 10% 増)。

## 却下した選択肢

**案 2 — `build.rs` に `collect_source_entries` を 3 回並べる。却下。**
`EMBEDCACHE-01` が当初想定していた形。動きはするが、root の一覧という**判断**が
テストできない build.rs 側に残る。しかも `rerun-if-changed` との一致は相変わらず
人の注意に依存したままで、今回の欠落を生んだ構造がそのまま残る。

**案 3 — 実際に読んだファイルだけを key にする (compile 後に依存集合を回収)。却下。**
最も正確だが、cache は compile の**前**に引くので鶏卵になる。
2 段 cache (依存集合の cache + bytes の cache) にすれば理屈は通るが、
最適化のための機構としては不釣り合いに重い。

**案 4 — `*.ls` / `*.wit` に拡張子で絞る。却下。**
理由は上記「ディレクトリを拡張子で絞らない理由」のとおり。絞り込みが resolver と
drift したとき、**静かに under-invalidate する**方向に壊れる。

**案 5 — 何もしない (実害未観測)。却下。**
`wit/` には `rerun-if-changed` が張られている以上、wit を編集する作業では確実に踏む。
「実害が観測されていない」は「踏んでいない」ではなく「踏んでも気付けない」の可能性がある
(埋め込まれた component が古いことは、それ自体では何も出力しない)。

## 含めない範囲

- `packages/` (`ModuleSearchPaths::package_sources`)。project ごとの探索パスで、
  embedded component の build には既定値経由でしか入らない。別件として扱う。
- cache entry 上限 (`EMBEDDED_COMPONENT_CACHE_ENTRIES`) と trim 方針。
- `LSHARP_EMBED_COMPONENT_PATH` 経路。この経路は cache を通らない。
- `stdlib` が実際に読まれるようにする / 読まれないようにする、といった module 解決側の変更。
  本 ADR は「入力になり得るものを key が覆う」ことだけを扱う。

## 実装順序 (doc-RED 時点の計画)

1. RED: `embedded_component_key_sources` が 3 root すべてを label 付きで覆うことを要求する test。
2. RED: `stdlib` 相当の root だけが変わったとき key が変わることを要求する test。
3. RED: `wit` 相当の root だけが変わったとき key が変わることを要求する test。
4. GREEN: 定数と関数を追加し、`build.rs` を差し替える。`rerun-if-changed` も定数から出す。
5. 挙動確認: 無変更 rebuild が cache hit のままであること、`stdlib/` の 1 ファイルを
   touch すると build script が再実行され cache miss になること。

## 受入条件

- (a) 上記 3 本の test が GREEN で、`cargo test -p lsharp-wasm embedded_component_cache` が緑。
- (b) 無変更の `cargo build -p lsharp-driver` が cache hit のままであること。
- (c) `stdlib/` または `wit/` の 1 ファイルを触ると cache miss になること。
- (d) `build.rs` に root の一覧が二重に書かれていないこと。

## Evidence

すべて 2026-08-18、worktree `codex/gate-fixes-root-lifetime` (base `e9227f3c`)、
Mac M-series / dev profile での実測。

### 前提の実測 (doc-RED 時点)

- `wit/` が実入力であることの根拠は `crates/lsharp-wasm/src/wasi.rs:239-243` と
  `component_adapter.rs:50-70` の読解。`emit_wasm_wasi_p2` が
  `wit/lsharp-compiler.wit` を渡し、`resolve_world` が WIT workspace を解決する。
- `stdlib/` が現 tree で読まれていないことは静的全数走査で確認した。
  `selfhost/src` 配下の `(import ...)` / `(open ...)` は **254 箇所・異なるモジュール 56 件**で、
  行頭形以外の出現は 0 件 (254 = 254)。**56 件すべてが `selfhost/src/` 配下で解決**し、
  `stdlib/` にしか無いものは 0 件、どちらにも無いものも 0 件だった。
- 走査コスト: `selfhost/src` 3.5M / 77 file に対し `stdlib` 96K / 12 file、
  `wit` 248K / 36 file。約 10% 増。

### RED

`embedded_component_key_sources` と `EMBEDDED_COMPONENT_KEY_ROOTS` は新設 API なので、
RED は assertion 失敗ではなく**コンパイルエラー**として現れた
(`error[E0425]: cannot find function ... not found in this scope` ×5、
`error: could not compile 'lsharp-wasm' (lib test) due to 7 previous errors`)。
旧 key 導出が wit / stdlib を覆っていなかったことは、旧 `build.rs:49-53` が
`collect_source_entries("selfhost/src", ...)` 1 本だけを `from_parts` へ渡していた
という**構造から自明**であり、これ以上の対照実験は置いていない。

### GREEN (unit)

`cargo test -p lsharp-wasm --lib embedded_component_cache` → **22 passed; 0 failed** (0.19s)。
新規は 6 本。

| test | 何を pin するか |
|---|---|
| `..._key_roots_cover_selfhost_stdlib_and_wit` | root 一覧そのもの |
| `..._key_sources_collect_every_root_with_labels` | 3 root が label 付きで載ること |
| `..._key_changes_when_only_stdlib_changes` | stdlib 単独変更で key が動く |
| `..._key_changes_when_only_wit_changes` | wit workspace 変更で key が動く |
| `..._key_sources_tolerate_a_missing_root` | root 不在の checkout で error にしない |
| `..._key_sources_cover_the_real_project_tree` | fixture ではなく**実 tree** の root 名 |

最後の 1 本は doc-RED の計画に無かった追加である。fixture だけだと `wit/` の rename 等で
実 tree の被覆が静かに 0 件になっても test が通り続けるため、実 tree に対する pin を足した。

### GREEN (挙動)

| 操作 | build script | cache | 所要 |
|---|---|---|---|
| `build.rs` 差し替え後の初回 build | 再実行 | miss (key が変わったため) | 1m37s |
| `touch stdlib/List.ls` (mtime のみ) | **再実行** | **hit** `b3c50215...` | 3.3s |
| `wit/lsharp-compiler.wit` に 1 行追記 | 再実行 | miss → 別 key `e2326cd3...` | -- |
| 同ファイルを復元 | 再実行 | hit `b3c50215...` | 2.2s |
| `stdlib/List.ls` に 1 行追記 | 再実行 | **miss** (hit 行が出ない) | 1m27s |
| 同ファイルを復元 | 再実行 | hit `b3c50215...` | -- |

2 行目が**新しい `rerun-if-changed=<root>/stdlib` が効いていることの直接の証拠**である
(従来この張り紙は無く、build script はそもそも再実行されなかった)。
3-4 行目は wit の内容差で key が実際に 2 つに分岐することを示す。

### 受入条件の判定

- (a) key 導出 test が GREEN — **満たした** (22 passed)
- (b) 無変更 rebuild は cache hit のまま — **満たした** (2.2〜3.3s の hit)
- (c) `stdlib/` / `wit/` を触ると miss — **満たした** (両 root で確認)
- (d) `build.rs` に root 一覧が二重に書かれていない — **満たした**
  (`EMBEDDED_COMPONENT_KEY_ROOTS` を key 導出と `rerun-if-changed` の両方が回す)

### lint / 回帰

- `cargo clippy -p lsharp-wasm -p lsharp-driver --all-targets` → 本変更が触った
  `embedded_component_cache.rs` / `build.rs` に対する警告は **0 件**
  (workspace 既存の警告は残るが、いずれも本変更の外)。
- `cargo test -p lsharp-wasm --lib` → `136 passed; 1 failed`。唯一の FAIL は
  `wasi::tests::test_root_set_invalid_slot_records_failure_ledger_before_trap` で、
  baseline 登録済みの既知 FAIL
  ([main exit 免除 ADR](decisions-root-lifetime-main-exit-exemption.md) の
  「満たせなかった受入条件」に記載)。本変更とは無関係。

### 満たせなかった受入条件

- **`stdlib/` が実入力へ変わる状況そのものは作っていない。** 現 tree では 56 モジュール
  すべてが `selfhost/src` で解決するので、stdlib を触っても生成 component の中身は
  変わらない。したがって上記の miss は「key が動いたこと」の証拠であって
  「古い bytes を埋め込む事故を実際に防いだこと」の証拠ではない。
  wit 側は実入力なので、そちらが本命の証拠になる。
- **stale hit が実際に起きる様子は再現していない。** 再現には旧 build.rs で
  wit だけを編集する対照 build が要るが、旧 key は emitter fingerprint に
  build script binary を含むため、build.rs を戻した時点で key が変わり対照にならない。
  構造からの推論に留めた。
- **`cargo build --workspace` / `check-workspace-baseline.sh` は再実行していない。**
  前者は本変更が `lsharp-wasm` lib と `lsharp-driver` build script にしか触れず、
  両者のビルドとテストを直接確認したことで代替した。後者は 5 時間級の実測を入力に取る。

## Consequences

cache key の入力 root が 1 箇所に集まり、`rerun-if-changed` と key の食い違いが
構造的に起きなくなった。root を足す作業は定数へ 1 行足すだけになり、
`test_embedded_component_key_roots_cover_selfhost_stdlib_and_wit` がその変更を必ず可視化する。

代償として、`stdlib/api.json` の再生成や `wit/README.md` の編集でも cache が miss する。
1m30s 程度の再コンパイルを踏むが、これは「絞り込みルールの drift で静かに
under-invalidate する」リスクと引き換えに意図して受け入れたものである。

`packages/` 探索パスは依然として key に入っていない。embedded component の build では
既定値経由でしか効かないため本 ADR の範囲外に置いたが、project ごとの package を
build に巻き込む変更が入ったときは同じ検討が要る。
