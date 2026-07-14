# Rust 依存境界の縮小

## 目的と対象

L# の通常開発を Rust toolchain や `cargo` の実行待ちから切り離す。対象の product/release target は Mac Apple Silicon (`aarch64-apple-darwin`) と Linux x86_64 (`x86_64-unknown-linux-gnu`) のみである。

ここでいう「Rust 不要」は、あらかじめ取得した native stage0 package を使う日常の編集・検査・テスト・Wasm 出力の経路に `cargo`、`rustc`、host の `lsharp` を置かないという意味である。公開 Rust driver の embedded guest 成功時も host `compile_file` を重ねず、失敗時だけ明示的な fallback を使う。Rust workspace の物理削除や、MCP/LSP を含む全 host integration の native 化は含まない。

この経路の成立は、自己ホスト実装が L# の全ての型・宣言意味論と parity を持つことを意味しない。現在自己ホストで検証済みの型注釈は `Int` / `Bool` / `String` / `Float` / `Unit` の named primitive、closed named head の再帰的な `TypeApp`、複数引数の関数型、lower-case `TypeExpr::Var` の raw representation である。`Ref (Vector Int)` と `(-> Int String Bool)` は parser から annotation unification まで確認済みであり、`Ref` / `Vector` の source 名は internal type constructor へ解決される。closed non-parametric `type-alias Name Target` は raw target を保存し、source order の prepass で `defn` の param / return signature と式内 `(: expr Alias)` に透過展開する。`Text -> String`、`RefText -> (Ref Text)`、`TextFn -> (-> Text Text)`、`(: "world" Str)` を parser-to-inference bundle で確認した。parametric `type-alias (Name a ...) Target` は parameter と raw target を保存し、source order の prepass で parameter ごとに fresh 型変数を割り当てる。arity が一致する `(Name Arg ...)` は target へ置換展開され、`Id Int -> Int`、`Callback Int String -> (-> Int String)`、`Box String -> (Ref String)` と式内 `(: "text" (Id String))` を確認した。forward closed alias chain も `Later -> LaterTarget -> String` の source-order 非依存な再評価により signature の受理と不一致診断まで確認済みであり、recursive alias は Rust と同じく `E0006` で拒否する parity を確認済みである（`test_e2e_selfhost_parser_forward_type_alias_unifies_signature`、`test_e2e_selfhost_parser_recursive_type_alias_is_rejected`、closed / parametric alias regression、`TypeInfer.ls` parse/check）。通常の `defn` 注釈では、同じ lower-case `TypeExpr::Var` 名を signature 内で共有し、異なる scoped 変数名も独立した fresh 型変数として扱う polymorphic slice を提供する。`id` の Int / Bool 別 call site と、`choose-first` の `a` / `b` を別々に具体化する call site を確認した。GADT variant の raw return type と match arm-local refinement は parser/type inference slice として検証済みだが、GADT exhaustiveness と full runtime parity は未完了である。record pattern runtime は source / ftable の direct field、binder、literal、fallback、nominal mismatch、patch/base chain marker propagation まで検証済みだが、nested record/constructor pattern と一般 Map API parity は未完了である。immutable record update は `CompilerMode` と ftable runtime slice で patch Map の recursive fallback と元 record の不変性を検証済みだが、内部表現の `map-size` / 反復まで含む完全な record API parity は未主張である。未完了の意味論を変更・検証する開発では、現時点では Rust implementation を source of truth / oracle として必要とする。

### scoped polymorphic `defn` signature (2026-07-15)

`TypeInferFunctions.ls` は `defn` の parameter / return annotation に現れる scoped 名ごとに共有 fresh 型変数を割り当て、通常の型環境で関数を一般化する。これにより `id` を Int と Bool の別 call site で使え、`choose-first [(: x a) (: y b)] : a x` では `a` と `b` を独立に具体化できる。GADT refinement と exhaustiveness は別タスクである。Evidence: `test_e2e_selfhost_scoped_type_var_defn_signature_is_polymorphic`、`test_e2e_selfhost_scoped_multiple_type_vars_defn_signature_is_polymorphic`、`TypeInfer.ls` check。

### 型・宣言意味論の更新 (2026-07-14)

直前の概要にある record 宣言未実装という記述は更新済みである。自己ホスト parser は field 名、`Type.field` accessor 名、raw TypeExpr を保持し、推論 prepass は record schema、constructor、accessor scheme を値環境へ登録して既知 record literal の field 型不一致を診断する。parametric record は `TypeInferRecordDecl.ls` が parameter ごとの bound variable を持つ structural record scheme を登録し、constructor、literal、accessor の使用ごとに scheme を instantiate する。Int field を持つ `Box` と Bool field を持つ `Box` の別使用箇所は独立であり、同じ `Pair a` literal 内の field は同じ具体化を共有する。`(. record field)` は let 束縛後も具体化済み schema の field 型を返し、field 型不一致と未定義 field を診断する。`{record | field value}` update も同じ schema 型へ単一化し、型不一致と未定義 field を診断する。`Type.field` は structural record 型との単一化を経て field 型を返し、不一致を診断する。record pattern はこの型推論 slice に含まない。static accessor の実行時 lowering は下記で実証済みである。

### record runtime 更新 (2026-07-14)

自己ホスト Wasm compiler は `CompilerMode` の file-compile 経路と legacy `compile-program-functions` / `compile-program-functions-with-base` で、`RecordLit`、`RecordUpdate`、direct `FieldAccess`、nonparametric record の `Point ...` constructor、`Point.field` static accessor を既存の `Map` runtime に lower する。record 本体を field 式の allocation 中も root に保持し、field hash を key に `map-insert` / `map-get` を使う。record update は更新 field だけを持つ patch Map に base Map を sentinel key `-1` で保持し、field lookup が patch chain を再帰的に辿るため、元の record は変更されない。record constructor と static accessor は user `defn` より前に prelude として function table / Wasm body へ登録し、Wasm entrypoint が最後の user function のままになる順序を保つ。actual compiler-mode E2E は `{Point label "record" x 42}` から `(. point label)` を `string-length` へ渡して `6`、`(. point x)` から `42` を出力し、`Point (inc 40) 2` の `Point.x` / `Point.y` が `41` / `2` を出力することを確認した。import された別 module の `Point` でも同じ `41` / `2` を generated Wasm で確認し、parametric `Box Int` / `Box Bool` の別具体化も `41` / `1` を出力する専用 E2E で確認した。さらに `p -> q -> r` の nested update を static / dynamic access で読み、`p` の値が保持されることを `test_e2e_selfhost_compiler_mode_record_update_runs` と ftable 経路の `test_e2e_selfhost_ftable_compiler_record_update_and_static_accessor_run` で確認した。normal compiler-mode の 11-import ABI に function table base `11` を揃え、direct source compile、imported file compile、`Cli` / `EmbeddedCli` の source-to-Wasm 入口で constructor/accessor call が runtime import と衝突しないことを確認した。record pattern は未完である。`App.Cli` と `EmbeddedCli` では `compile-source-wasm-bytes` が full functions/data payload を `build-wasm-bytes-wasi` へ渡す source-string slice を追加し、`EmbeddedCli` の component target は summary text を生成せず外部 packaging 境界を返す。一方、`SmokeCli` と no-arg `PipelineSmoke` は legacy `lower` を残し、component sidecar の生成は外部ツール境界であるため、legacy public surface 全体の Rust-free compile は未完了である。generated Wasm の opcode 87 (`print-string`) は通常の `CompilerMode` 出力で 11 番目の `env` runtime import への `call 10` として出力するところまで修正済みだが、外部 runtime の文字列 ABI 接続と standalone WASI Preview1 実行は別の output parity gap として残る。

source / ftable compiler-mode では record literal / static constructor が nominal marker `-3` を Map に保存し、canonical record pattern の type hash と照合する。同じ field layout を持つ別 record type の arm fallback、ftable nominal pattern の独立 E2E、`p -> q -> r` patch/base Map chain への marker 伝播を確認済みである。nested record/constructor pattern と一般 Map API parity は残課題である。

### legacy source compile boundary 更新 (2026-07-14)

`App.Cli`、`EmbeddedCli`、`SmokeCli` の source-string helper は、`parse-program` の結果を `compile-program-functions-with-source` に渡し、先頭 IR だけを返す `lower` ではなく全 functions/data を `build-wasm-bytes-wasi` へ渡す。これにより helper 自体は複数 top-level function を落とさない。`EmbeddedCli` の component target は summary text を出力せず、外部 component packaging が必要な境界を明示的に返す。一方 `App.Cli` / `EmbeddedCli` の component sidecar と no-arg `PipelineSmoke` は別の legacy / external surface であり、今回の変更はそれらを置き換えない。

### 型・宣言意味論の更新: ordinary ADT (2026-07-14)

ordinary ADT は parser が variant 名と raw field TypeExpr を保持し、`TypeInferAdt.ls` の prepass が type parameter を束縛した constructor scheme を値環境へ登録する。`(type (Maybe a) (Just a) Nothing)` の constructor application と match pattern は同じ polymorphic scheme を使い、`Int` と `Bool` の別使用箇所で独立に instantiate される。さらに selfhost Wasm compiler の source / ftable 経路で Map-based constructor、variant tag、direct field binder、nested constructor pattern、constructor mismatch fallback を actual Wasm 実行で確認した（`test_e2e_selfhost_compiler_mode_adt_constructor_pattern_binds_and_falls_back`、`test_e2e_selfhost_compiler_mode_adt_nested_constructor_pattern_runs`、`test_e2e_selfhost_ftable_compiler_adt_constructor_pattern_runs`）。これは ordinary ADT の parser / 型検査と source / ftable runtime の Rust-free slice を示すが、full ftable/import target parity、Rust linear-memory ABI parity、nominal/exhaustiveness closure は未完了である。GADT の variant return type、pattern refinement、exhaustiveness も含まれない。

## 現在の事実

- `lsharp-native-selfhost-stage0` package は `compiler`、`transport_driver`、`materializer` を持つ manifest で native bootstrap を開始する。release 用の `App.Cli` archive は stage0 package ではない。
- Mac Apple Silicon では、current fixed-point stage3 compiler を stage0 package 化し、`scripts/native-selfhost-dev.sh` を通す source-file smoke が成功している。smoke は `cargo`、`rustc`、host `lsharp` を PATH 上で失敗させた状態で `parse`、`check`、`fmt`、通常と metadata の `test`、`compile -o`、`build -o` を実行する。
- Linux x86_64 は、最後に検証した commit `4bd9ee9` から生成した fresh actual-stage1 を stage0 package 化し、Lima `lsharp-linux-x86` VM 内で `native-linux-x86-native-stage0-source-file-smoke.sh` を成功させた。2,779 functions の transport/materialize を通過し、`cargo`、`rustc`、host `lsharp` を blocklist に入れた状態で `parse`、`check`、`fmt`、通常と metadata の `test`、`compile -o`、`build -o` を完走した。今回の GADT / record pattern selfhost 変更はこの gate の後に加わったため、checkpoint commit 後に同じ Linux gate を再実行する必要がある。2026-07-14 の historical `8dd37ef-static-string-fixedpoint` replay における `parse stdout is missing decls:1` は、fresh stage0 で解消された過去の failure evidence として残す。
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
既定の transport は 64 functions/chunk、chunk timeout は 900 秒である。checkpoint を再利用する診断時だけ `LSHARP_NATIVE_LINUX_X86_TRANSPORT_CHUNK_SIZE` と `LSHARP_NATIVE_LINUX_X86_TRANSPORT_TIMEOUT_SECONDS` を指定して VM 側へ引き渡せる。VM の disk size や空き容量 gate は変更しない。

## Command Boundary

| Command surface | Native の責務 | Rust の要否 | 外部条件・制約 |
| --- | --- | --- | --- |
| `parse` / `check` / `fmt` / `test` | native `program.native` が直接実行する core CLI | 検証済み core slice では不要 | Bash、Python 3、hash tool。stage materialize は Mac で `clang`、Linux で `cc` を使う。Mac は必要な host でのみ codesign identity を指定する。型・宣言の未実装 P0 は Rust oracle が必要。 |
| embedded driver の guest-success compile/build | guest の artifact summary / output をそのまま返す | 不要 | guest exit code 0 では Rust `compile_file` を呼ばず、失敗時だけ host artifact fallback。runtime disable 下の `test` は delegation hint。 |
| `compile -o` / `build -o` (WASI Preview1) | native CLI が actual core Wasm bytes を出力する | 通常開発では不要 | 上と同じ。Mac は検証済み、Linux は current stage0 gate 待ち。 |
| component `compile` / `build` | native core Wasm を component 化する | 不要 | Python helper と外部 `wasm-tools` が必要。これは Rust host fallback ではない。 |
| `install` | package install / module index helper | 不要 | Python 3。git dependency は `git` が必要。 |
| `repl` | expression ごとの native compile + run | 不要 | Python helper と外部 `wasmtime`。stateful evaluator ではない。 |
| `doc` | native `doc --json` を document helper が整形する | 不要 | Python helper。 |
| `lsp --stdio` | native program に stdio replay shim を接続する | 不要 | Python shim。bare `lsp` は native runner が明示的に拒否する。 |
| `mcp-server` | native runner は提供しない | 必要 | Rust host integration の責務として明示的に失敗する。 |
| `compile --emit-ir` | native runner は提供しない | 必要 | Rust host integration の責務として明示的に失敗する。 |
| `--target web-wasm` / `--target native` | native runner は提供しない | 必要 | native selfhost の supported output target 外として明示的に失敗する。 |

## Record pattern の現在地

2026-07-15 時点で、selfhost parser は record pattern の field 配置を維持したまま record type name hash を AST 末尾へ保存し、`TypeInferPattern.ls` は登録済み record schema を instantiate して各 child pattern を field 型と unify する。未登録 record、未定義 field、field child の型不一致は selfhost 側で診断できる。既存の type name なし手組み AST は shallow fallback を維持する。

一方、selfhost Wasm compiler の match lowering は direct record Map の field presence/value lookup、field binder local、literal child check、arm fallback、nominal type mismatch fallback を source / ftable の actual Wasm 実行で確認済みである。`p -> q -> r` の patch/base Map chain でも nominal marker を保持し、ftable 経路の独立 record pattern E2E を通過した。ただし nested record/constructor pattern と一般 Map API parity は未完了である。したがって、この進捗は record pattern の一部を Rust oracle の必須範囲から外したものであり、L# の全 record pattern 機能が Rust なしで使えることを意味しない。

上記の nominal type hash は全経路の完了を意味しない。source / ftable compiler-mode の direct record literal / canonical pattern mismatch と patch/base Map chain の marker 伝播は検証済みだが、nested pattern と一般 Map API parity は未検証である。

## Rust に残る責務

Rust が完全に不要になったわけではない。次の作業は native base development loop の外側に残る。

1. stage0 の生成・配布・取得。fresh clone が自動で stage0 を取得する public release contract は別途閉じる必要がある。通常開発は供給済み stage0 package を前提にする。
2. native selfhost と Rust implementation の oracle/differential 比較、障害解析、emergency rollback。
3. `mcp-server`、bare LSP、`--emit-ir`、native target など、上表で明示した Rust host integration surface。

したがって、Linux gate 完了後に「検証済み core CLI の日常ループは Rust なしで開発可能」と言える。一方で、自己ホストの型・宣言意味論 P0 が未完了の間は「base language development 全体」や「L# の全機能」が Rust なしとは言えない。closed / parametric alias の signature・式内 annotation、forward closed alias の signature、recursive alias の E0006 rejection、scoped polymorphic `defn` signature、ordinary ADT の parser / 型検査と direct runtime slice、parametric record の constructor/literal/field access/update/runtime、`Type.field` accessor の型検査、immutable record update の nested runtime slice、record pattern の source / ftable direct runtime slice はその境界を少し狭めた。ordinary ADT runtime の残り、record runtime の full public closure、legacy `lower` / embedded compiler surface、上表の Rust-only surface、external tool dependency、record pattern の残り、GADT exhaustiveness / full runtime parity などの未実装 P0 は残る。

### 残る Base Language Gap

Rust を base implementation から外すため、legacy `lower` / embedded compiler の full-program 化、ordinary ADT runtime の残り、record pattern の nested record/constructor pattern、GADT exhaustiveness / full runtime parity を自己ホスト側で実装・差分検証する必要がある。recursive alias は Rust implementation と同じく拒否するため、未対応の recursive language feature としては数えない。`CompilerMode` と ftable 経路における nonparametric / parametric record の constructor/literal/direct/static accessor/update runtime、record pattern の direct field/binder/literal/fallback/nominal/patch-chain slice、ordinary ADT の parser / 型検査と source / ftable 経路の direct / nested constructor/tag/binder/fallback slice はこの一覧から除外する。ただし ordinary ADT の full ftable/import target parity、Rust linear-memory ABI parity、nominal/exhaustiveness closure、record を一般 Map として扱う全 API の parity は別途確認する。出力側では `print-string` の 11-import env ABI を標準 WASI Preview1 と接続し、standalone 実行まで確認する作業が残る。

ここでいう record pattern の残件は、検証済みの direct / nominal / patch-chain slice 以外の全体 parity、特に nested pattern と一般 Map API を指す。

## 検証と残タスク

- `bash scripts/ci/test-native-selfhost-dev.sh` は runner の source refresh、native direct command routing、external helper routing、Rust-only command の明示拒否を検証する。
- `test_native_selfhost_dev_source_file_smoke_script_contract` は smoke が host fallback を発見・利用しないことを固定する。
- `test_e2e_native_macos_aarch64_materializer_executes_tiny_stage_code` は macOS materializer の再署名成功時に stderr が空であることを固定する。
- `test_guest_compile_success_does_not_request_host_fallback` と `test_test_command_is_selfhost_shadow_command` は driver の guest-success / Rust fallback boundary を固定する。
- Mac Apple Silicon の actual stage0 source-file smoke は 2026-07-13 に成功した。
- Linux x86_64 の last verified source-file gate は 2026-07-15、commit `4bd9ee9` で成功した。今回の GADT / record pattern checkpoint を反映した stage0 package は未再生成のため、commit 後の Linux gate を残タスクとする。historical stage0 replay の `parse stdout is missing decls:1` は再発しなかった。
- selfhost Wasm の `print-string` は 11-import layout と `call 10` の emitter contract まで検証済みだが、標準 WASI Preview1 host だけで実行できる standalone ABI までは検証していない。
- Linux 成功後の両 target の evidence を再確認し、V2-16d は完了とする。V2-16e は、stage0 の public acquisition、Rust oracle/differential、emergency rollback、未完の言語意味論を残すため継続する。stage0 の public acquisition は別の release/distribution task として追跡する。
