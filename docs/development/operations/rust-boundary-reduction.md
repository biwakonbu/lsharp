# Rust 依存境界の縮小

## 目的と対象

L# の通常開発を Rust toolchain や `cargo` の実行待ちから切り離す。対象の product/release target は Mac Apple Silicon (`aarch64-apple-darwin`) と Linux x86_64 (`x86_64-unknown-linux-gnu`) のみである。

ここでいう「Rust 不要」は、あらかじめ取得した native stage0 package を使う日常の編集・検査・テスト・Wasm 出力の経路に `cargo`、`rustc`、host の `lsharp` を置かないという意味である。Rust workspace の物理削除や、MCP/LSP を含む全 host integration の native 化は含まない。

この経路の成立は、自己ホスト実装が L# の全ての型・宣言意味論と parity を持つことを意味しない。現在自己ホストで検証済みの型注釈は `Int` / `Bool` / `String` / `Float` / `Unit` の named primitive、closed named head の再帰的な `TypeApp`、複数引数の関数型、lower-case `TypeExpr::Var` の raw representation と `defn` 注釈における nominal resolution である。`Ref (Vector Int)` と `(-> Int String Bool)` は parser から annotation unification まで確認済みであり、`Ref` / `Vector` の source 名は internal type constructor へ解決される。closed non-parametric `type-alias Name Target` は raw target を保存し、source order の prepass で `defn` の param / return signature と式内 `(: expr Alias)` に透過展開する。`Text -> String`、`RefText -> (Ref Text)`、`TextFn -> (-> Text Text)`、`(: "world" Str)` を parser-to-inference bundle で確認した。parametric `type-alias (Name a ...) Target` は parameter と raw target を保存し、source order の prepass で parameter ごとに fresh 型変数を割り当てる。arity が一致する `(Name Arg ...)` は target へ置換展開され、`Id Int -> Int`、`Callback Int String -> (-> Int String)`、`Box String -> (Ref String)` と式内 `(: "text" (Id String))` を確認した。通常の `defn` 注釈における `TypeExpr::Var` は source 名を nominal type として扱い続けるため、parametric alias target の内部以外で scoped polymorphic variable を提供するものではない。forward / recursive alias、immutable record update、record pattern、GADT の variant return type / refinement は未完了である。これらを変更・検証する開発では、現時点では Rust implementation を source of truth / oracle として必要とする。

### 型・宣言意味論の更新 (2026-07-14)

直前の概要にある record 宣言未実装という記述は更新済みである。自己ホスト parser は field 名、`Type.field` accessor 名、raw TypeExpr を保持し、推論 prepass は record schema、constructor、accessor scheme を値環境へ登録して既知 record literal の field 型不一致を診断する。parametric record は `TypeInferRecordDecl.ls` が parameter ごとの bound variable を持つ structural record scheme を登録し、constructor、literal、accessor の使用ごとに scheme を instantiate する。Int field を持つ `Box` と Bool field を持つ `Box` の別使用箇所は独立であり、同じ `Pair a` literal 内の field は同じ具体化を共有する。`(. record field)` は let 束縛後も具体化済み schema の field 型を返し、field 型不一致と未定義 field を診断する。`{record | field value}` update も同じ schema 型へ単一化し、型不一致と未定義 field を診断する。`Type.field` は structural record 型との単一化を経て field 型を返し、不一致を診断する。record pattern はこの型推論 slice に含まない。static accessor の実行時 lowering は下記で実証済みである。

### record runtime 更新 (2026-07-14)

自己ホスト Wasm compiler は `CompilerMode` の file-compile 経路と legacy `compile-program-functions` / `compile-program-functions-with-base` で、`RecordLit`、direct `FieldAccess`、nonparametric record の `Point ...` constructor、`Point.field` static accessor を既存の `Map` runtime に lower する。record 本体を field 式の allocation 中も root に保持し、field hash を key に `map-insert` / `map-get` を使う。record constructor と static accessor は user `defn` より前に prelude として function table / Wasm body へ登録し、Wasm entrypoint が最後の user function のままになる順序を保つ。actual compiler-mode E2E は `{Point label "record" x 42}` から `(. point label)` を `string-length` へ渡して `6`、`(. point x)` から `42` を出力し、`Point (inc 40) 2` の `Point.x` / `Point.y` が `41` / `2` を出力することを確認した。import された別 module の `Point` でも同じ `41` / `2` を generated Wasm で確認した。legacy 10-import base wrapper でも同じ `41` / `2` を generated Wasm で確認した。immutable update、record pattern、parametric record runtime の専用 E2E は未完である。さらに `EmbeddedCli` / `SmokeCli` / no-arg pipeline smoke が使う legacy `lower` 経路は full program ではなく先頭 IR だけを返すため、今回の証跡はこの legacy public surface の Rust-free compile を意味しない。generated Wasm の opcode 87 (`print-string`) は通常の `CompilerMode` 出力で 11 番目の `env` runtime import への `call 10` として出力するところまで修正済みだが、外部 runtime の文字列 ABI 接続と standalone WASI Preview1 実行は別の output parity gap として残る。

### 型・宣言意味論の更新: ordinary ADT (2026-07-14)

ordinary ADT は parser が variant 名と raw field TypeExpr を保持し、`TypeInferAdt.ls` の prepass が type parameter を束縛した constructor scheme を値環境へ登録する。`(type (Maybe a) (Just a) Nothing)` の constructor application と match pattern は同じ polymorphic scheme を使い、`Int` と `Bool` の別使用箇所で独立に instantiate される。これは通常 ADT の constructor/pattern 型検査を Rust oracle の必須範囲から外す進捗であり、GADT の variant return type、pattern refinement、exhaustiveness は含まない。

## 現在の事実

- `lsharp-native-selfhost-stage0` package は `compiler`、`transport_driver`、`materializer` を持つ manifest で native bootstrap を開始する。release 用の `App.Cli` archive は stage0 package ではない。
- Mac Apple Silicon では、current fixed-point stage3 compiler を stage0 package 化し、`scripts/native-selfhost-dev.sh` を通す source-file smoke が成功している。smoke は `cargo`、`rustc`、host `lsharp` を PATH 上で失敗させた状態で `parse`、`check`、`fmt`、通常と metadata の `test`、`compile -o`、`build -o` を実行する。
- Linux x86_64 は current import ABI 修正後の stage1 -> stage3 fixed-point を Lima VM で再生成中である。以前の cross artifact による App.Cli smoke は current stage0 からの再生成を証明しないため、Linux の同じ source-file smoke が通るまで完了扱いにしない。
- native bootstrap の初回だけは source tree を再生成する。fingerprint が不変なら `scripts/native-selfhost-dev.sh` は生成済み `program.native` を再利用する。
- `LSHARP_NATIVE_MACOS_AARCH64_CODESIGN_IDENTITY` は macOS host policy 上、生成済み Mach-O の実行に署名が必要な環境でだけ指定する。成功時の codesign 出力は command stderr に漏らさず、失敗時だけ診断として返す。
- GitHub Actions の自動 build は使わない。検証と release は Mac と Lima VM の手動 local gate で行う。

## Native 開発経路

`fetch-stage0.sh` が配置した `./stage0` package があれば、通常のコア開発は次の runner を使う。

```bash
./scripts/native-selfhost-dev.sh check examples/fib.ls

./scripts/native-selfhost-dev.sh --bootstrap compile examples/fib.ls -o fib.wasm
```

`NATIVE_STAGE0_DIR=/path/to/lsharp-native-selfhost-stage0` または `--stage0-dir` は、別の stage0 package を比較・検証する場合だけ指定する。`--bootstrap` は stage0 compiler で current `selfhost/` を native program に再生成する。通常コマンドだけであれば、同じ source fingerprint で bootstrap を繰り返さない。

Linux x86_64 の final gate は macOS host から Lima へ package と必要最小限の source/scripts をコピーして実行する。

```bash
LSHARP_NATIVE_LINUX_X86_STAGE0_DIR=/path/to/linux-stage0 \
  ./scripts/ci/native-linux-x86-native-stage0-source-file-smoke.sh
```

この wrapper は VM の `/tmp` 空き容量を 4 GiB 以上で確認し、VM 内で `scripts/ci/native-selfhost-dev-source-file-smoke.sh` を実行する。source-file smoke は `cargo`、`rustc`、host `lsharp` を blocklist に入れるため、Rust host fallback は成功条件にならない。

## Command Boundary

| Command surface | Native の責務 | Rust の要否 | 外部条件・制約 |
| --- | --- | --- | --- |
| `parse` / `check` / `fmt` / `test` | native `program.native` が直接実行する core CLI | 検証済み core slice では不要 | Bash、Python 3、hash tool。stage materialize は Mac で `clang`、Linux で `cc` を使う。Mac は必要な host でのみ codesign identity を指定する。型・宣言の未実装 P0 は Rust oracle が必要。 |
| `compile -o` / `build -o` (WASI Preview1) | native CLI が actual core Wasm bytes を出力する | 通常開発では不要 | 上と同じ。Mac は検証済み、Linux は current stage0 gate 待ち。 |
| component `compile` / `build` | native core Wasm を component 化する | 不要 | Python helper と外部 `wasm-tools` が必要。これは Rust host fallback ではない。 |
| `install` | package install / module index helper | 不要 | Python 3。git dependency は `git` が必要。 |
| `repl` | expression ごとの native compile + run | 不要 | Python helper と外部 `wasmtime`。stateful evaluator ではない。 |
| `doc` | native `doc --json` を document helper が整形する | 不要 | Python helper。 |
| `lsp --stdio` | native program に stdio replay shim を接続する | 不要 | Python shim。bare `lsp` は native runner が明示的に拒否する。 |
| `mcp-server` | native runner は提供しない | 必要 | Rust host integration の責務として明示的に失敗する。 |
| `compile --emit-ir` | native runner は提供しない | 必要 | Rust host integration の責務として明示的に失敗する。 |
| `--target web-wasm` / `--target native` | native runner は提供しない | 必要 | native selfhost の supported output target 外として明示的に失敗する。 |

## Rust に残る責務

Rust が完全に不要になったわけではない。次の作業は native base development loop の外側に残る。

1. stage0 の生成・配布・取得。fresh clone が自動で stage0 を取得する public release contract は別途閉じる必要がある。通常開発は供給済み stage0 package を前提にする。
2. native selfhost と Rust implementation の oracle/differential 比較、障害解析、emergency rollback。
3. `mcp-server`、bare LSP、`--emit-ir`、native target など、上表で明示した Rust host integration surface。

したがって、Linux gate 完了後に「検証済み core CLI の日常ループは Rust なしで開発可能」と言える。一方で、自己ホストの型・宣言意味論 P0 が未完了の間は「base language development 全体」や「L# の全機能」が Rust なしとは言えない。closed / parametric alias の signature・式内 annotation、ordinary ADT の constructor/pattern、parametric record の constructor/literal/field access/update、`Type.field` accessor の型検査 slice はその境界を少し狭めた。nonparametric record の constructor/static accessor runtime も `CompilerMode` では検証済みだが、legacy `lower` / embedded compiler surface、上表の Rust-only surface、external tool dependency、forward / recursive alias、scoped polymorphic variable、immutable record update、record pattern、parametric record runtime の専用 E2E、GADT return type/refinement などの未実装 P0 は残る。

### 残る Base Language Gap

Rust を base implementation から外すため、legacy `lower` / embedded compiler の full-program 化、forward / recursive alias、scoped polymorphic variable、immutable record update、record pattern、GADT return type/refinement を自己ホスト側で実装・差分検証する必要がある。`CompilerMode` と legacy function wrapper における nonparametric record の constructor/literal/direct/static accessor runtime と、nonparametric / parametric record の schema / `Type.field` accessor 型検査、ordinary ADT constructor/pattern はこの一覧から除外する。parametric record runtime は専用 E2E を通すまで保留する。出力側では `print-string` の 11-import env ABI を標準 WASI Preview1 と接続し、standalone 実行まで確認する作業が残る。

## 検証と残タスク

- `bash scripts/ci/test-native-selfhost-dev.sh` は runner の source refresh、native direct command routing、external helper routing、Rust-only command の明示拒否を検証する。
- `test_native_selfhost_dev_source_file_smoke_script_contract` は smoke が host fallback を発見・利用しないことを固定する。
- `test_e2e_native_macos_aarch64_materializer_executes_tiny_stage_code` は macOS materializer の再署名成功時に stderr が空であることを固定する。
- Mac Apple Silicon の actual stage0 source-file smoke は 2026-07-13 に成功した。
- Linux x86_64 は current fixed-point artifact を stage0 package 化し、`native-linux-x86-native-stage0-source-file-smoke.sh` を成功させることが残っている。
- selfhost Wasm の `print-string` は 11-import layout と `call 10` の emitter contract まで検証済みだが、標準 WASI Preview1 host だけで実行できる standalone ABI までは検証していない。
- Linux 成功後、両 target の evidence と command boundary を再確認して V2-16d / V2-16e の完了可否を判断する。stage0 の public acquisition はそれとは別の release/distribution task として追跡する。
