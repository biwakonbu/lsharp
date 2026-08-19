# AGENTS.md

This file provides guidance to Codex (Codex.ai/code) when working with code in this repository.

## 言語規則

- **自然言語**: 日本語を使用
- **コメント**: 日本語で記述
- **変数・関数名**: 英語（国際標準）
- **コード**: 英語（国際標準）

## 作業ディレクトリと worktree の隔離

- Codex が新しく作る worktree、`target`、生成物、一時 checkout は `/Users/biwakonbu/github/tmp/<task>/` 配下に限定する。`/Users/biwakonbu/github` 直下には新規作成しない。
- root checkout は別セッションとの共有差分を含み得るため、通常の実装作業では直接編集せず、専用 worktree で作業する。
- 作業完了後は、自分が作った専用 worktree、`target`、VM の一時領域を検証後に削除する。他セッションが所有する worktree や `tmp` 内の作業は移動・削除しない。

## プロジェクト概要

L# (lsharp) は S 式構文 + Hindley-Milner 型推論の言語。WebAssembly (WASI) をターゲットに、wasmtime で直接実行可能。

## ビルド・テスト・リント

```bash
cargo build                        # ビルド
cargo test                         # 全テスト実行
cargo test test_e2e_fibonacci      # 個別テスト実行
cargo test -p lsharp-wasm          # クレート単位でテスト
cargo clippy                       # リント
```

## CLI コマンド

```bash
cargo run -- compile examples/fib.ls -o fib.wasm  # 公開 CLI の基本動線
cargo run -- test examples/fib.ls                 # メタデータテスト (:example, :invariant)
cargo run -- lsp                                  # IDE 向けバックエンド
cargo run -- mcp-server                           # AI 向けバックエンド
```

公開 CLI は `compile` 中心で案内する。`parse` / `check` / `fmt` は LSP / MCP が利用する内部 API として扱い、
ユーザー向けの手順や smoke test には載せない。

## ワークスペース構成

7 クレートの Cargo ワークスペース。コンパイラパイプライン順:

| クレート | 役割 |
|---------|------|
| `lsharp-syntax` | Lexer + Parser → AST 生成 |
| `lsharp-types` | Hindley-Milner 型推論・制約解決・メタデータ検証 |
| `lsharp-ir` | AST → IR への変換 (lowering)、モジュールリンク |
| `lsharp-wasm` | IR → WebAssembly バイナリ生成 (WASI) |
| `lsharp-driver` | CLI エントリポイント、プロジェクト管理 |
| `lsharp-lsp` | LSP サーバー (tower-lsp 統合) |
| `lsharp-docs` | ドキュメント追跡・レビュー管理 |

## コンパイラパイプライン

```
Source (.ls)
  → Lexer (lsharp-syntax/lexer.rs) → Token列
  → Parser (lsharp-syntax/parser.rs) → AST (Program)
  → Type Inference (lsharp-types/infer.rs) → 型チェック済み AST
  → Lowering (lsharp-ir/lower.rs) → IR (Module)
  → Codegen (lsharp-wasm/wasi.rs) → .wasm バイナリ
```

## 主要な型

- **AST**: `Program`, `Expr`, `Decl`, `Pattern`, `Literal`, `Metadata` (lsharp-syntax/ast.rs)
- **型システム**: `Type` (Con/Var/Fun/App/Record), `TypeScheme`, `Substitution`, `TypeEnv` (lsharp-types/types.rs)
- **IR**: `Module`, `Function`, `Instruction`, `IrType` (lsharp-ir/lib.rs)
- **制約**: `TraitConstraint`, `ConstrainedTypeInfo`, `ConstraintDef` (lsharp-types/constraints.rs)

## テスト構成

- **E2E テスト**: `crates/lsharp-wasm/tests/e2e.rs` — フルパイプライン (parse → infer → lower → codegen → WASI 実行)
- **スナップショットテスト**: `insta` クレートによる IR/型出力の回帰テスト
- **メタデータテスト**: `:example` / `:invariant` アノテーションからの自動テスト生成

## TDD ワークフロー (必須)

実装タスクは必ず TDD (テスト駆動開発) で進める。テストなしの実装は完了と見なさない。

### フロー

1. **RED**: テストを先に書く。Rust oracle/bootstrap の契約は `cargo test` で失敗を確認し、Rust-free slice は同じ fixture を `scripts/native-selfhost-dev.sh` の native stage0 経路でも失敗させる。`cargo test` だけでは native evidence としない。
2. **GREEN**: 実装を書く。Rust oracle の focused test と native stage0 の対応 command を順に成功させ、Rust host fallback の成功を native GREEN として扱わない。
3. **REFACTOR**: リファクタリング → テスト成功を維持
4. **UPDATE**: 検証済み evidence を ADR/docs と TODO.md に反映する。verified slice や partial parity は `[~]` のまま残し、項目全体の完了条件を満たしたら判断と evidence を ADR へ移して TODO.md から項目を削除する。`[x]` は使わない。

### ルール

- 実装ファイルを編集する前に、必ず対応するテストを書く
- テストが 0 個の項目は完了扱いにせず (`[~]` で留める)、完了項目は ADR へ移して TODO.md から削除する。
- テストが失敗したら実装を修正する (テストの期待値を変更しない)
- `/tdd <タスク>` コマンドで TDD ワークフローを起動できる (例: `/tdd P6-3 Computation Expression の脱糖実装`)

## Rust-free selfhost の進め方

L# の最終目標は、Rust 実装を正本として残したまま一部のコマンドだけを動かすことではなく、L# の全言語機能と公開コマンドを自己ホスト実装へ段階的に移し、通常の開発・テスト・Wasm 出力を Rust なしで完走できる状態にすることである。作業中は Rust を bootstrap、oracle/differential 検証、障害時の rollback、未移行 host integration のために保持するが、それを理由に未対応機能を完了扱いにしない。

- 対応 target は Mac Apple Silicon (`aarch64-apple-darwin`) と Linux x86_64 (`x86_64-unknown-linux-gnu`) に限定する。日常の core CLI は、producer/source commit と source fingerprint が current checkout に一致し、manifest と検証証跡がある native stage0 と `scripts/native-selfhost-dev.sh` を入口にする。stale または fingerprint mismatch の stage0 は採用せず、成功経路で `cargo`、`rustc`、host `lsharp`、Rust fallback を呼ばない。
- stage0 manifest の `selfhost_src_fingerprint` は必須 field である。欠落した stage0 は strict lane / dev lane のどちらでも採用しない (旧 package の暗黙採用を防ぐ)。値の算出は `scripts/lib/source-fingerprint.sh` の `lsharp_source_fingerprint` に一本化し、producer (`scripts/ci/package-native-stage0.sh`)、consumer (`scripts/native-selfhost-dev.sh`)、`scripts/dev-loop.sh` は必ずこれを source する。独自に再実装しない。
- `scripts/native-selfhost-dev.sh` は 2 lane を持つ。**strict lane** (既定) は `source_commit` と `selfhost_src_fingerprint` の両方一致を要求する。**dev lane** (`--dev-reuse` または `NATIVE_ALLOW_FINGERPRINT_REUSE=1`) は `source_commit` 不一致だけを許し、fingerprint 不一致は依然として `die` する。どちらも fail-closed で、成功経路に `cargo` / `rustc` / host `lsharp` を入れない。
- dev lane の結果は証跡に採用しない。dev lane で起動した stage directory には `.lane` に `dev-reuse` が記録され、strict lane は同 file を削除する。gate script は `.lane` の非存在をもって「この stage は evidence-eligible」と判定する。
- 日常の運用はこうなる。docs / scripts / Rust 側だけを commit した場合は `selfhost/src` が不変なので dev lane で stage0 を再利用できる。`selfhost/src` を編集した場合は fingerprint が変わるため **両 lane とも die** し、`scripts/ci/native-macos-aarch64-stage0-release.sh` による stage0 再生成が必要になる。この再生成コストを消すのが `LEGACY-MODULE-01` (selfhost module cache) であり、それが入るまでは source 編集ループの待ち時間は残る。
- strict lane を使う smoke (`scripts/ci/native-selfhost-dev-source-file-smoke.sh` など) は、stage0 を生成した commit から HEAD が進むと設計どおり失敗する。これは regression ではない。証跡を取り直すときは、その HEAD で stage0 を再生成してから実行する。
- 言語機能を Rust-free 完了とするには、parser、型推論、lowering、codegen、runtime、source/ftable/import の必要経路を同じ仕様で閉じ、対応 target の native program から実際に実行する E2E テストを追加する。単一レイヤーの unit test、Rust driver 経由の成功、summary/header の生成だけでは完了としない。
- Rust oracle は parity を確認するために使う。新しい L# 実装は RED テスト、Rust との診断/出力差分確認、native stage0 の実行確認、regression test の順で進め、未対応機能は誤った Wasm を出さず明示的な診断または明示的な外部境界を返す。fallback が一度でも実行された結果は native gate の pass/evidence に数えず、fallback 発生を stderr または evidence に残す。
- `compile` / `build` の全 target、EmbeddedCli/Component の実成果物、LSP/MCP/REPL/install/doc などの公開 surface を個別に検証する。明示拒否や Rust host integration は境界を正しく扱った証拠ではあるが、Rust-free 実装完了の証拠ではない。guest-success、artifact bytes、standalone runtime、外部 helper の境界を分けてテストする。
- 長時間の stage regeneration や Linux VM gate の実行中は、対象を共有しない parser/type/runtime の focused test、docs、診断、fixture、契約テストを並行して進める。VM の待ち時間を理由に実装を止めず、完了後に native gate と fixed-point evidence を統合する。
- `TODO.md` と `docs/development/operations/rust-boundary-reduction.md` は current truth として更新する。TODO.md は active-only の正本とし、`[x]` は使わず、partial parity、既知の Rust-only surface、未検証の ABI は `[~]` と残リスクに記録する。要件全体が完了した項目は ADR へ移して TODO.md から削除する。
- stage0 の生成・配布・source commit provenance・rollback は運用上の bootstrap boundary であり、通常開発から Rust を外せても、公開 release の再現性と緊急復旧を検証するまで削除しない。

### 今後の標準進行（L# dogfooding、正本）

この節を、L# を L# で開発しながら Rust 依存を段階的に置換するための運用契約とする。単一の成功テスト、Rust driver の成功、生成 summary、または stale な stage0 artifact だけで Rust-free 完了を宣言しない。

- 通常の L# の実装・テスト・Wasm 出力は、検証済み native stage0 と `scripts/native-selfhost-dev.sh` を入口に L# 自身で進める。Rust の `cargo test` は oracle/bootstrap/differential lane、native stage0 の `test` は Rust-free dogfooding lane として別々に記録し、成功経路に `cargo`、`rustc`、host `lsharp`、暗黙の Rust fallback を入れない。
- Rust は削除対象ではなく、stage0 の取得・再生成・provenance、Rust oracle/differential、障害解析、emergency rollback、未移行 host integration のための明示的な境界として残す。未対応機能を Rust fallback で成功したように見せない。
- 対応 target は Mac Apple Silicon (`aarch64-apple-darwin`) と Linux x86_64 (`x86_64-unknown-linux-gnu`) に限定する。別 target の対応を進捗や完了条件へ混ぜない。
- 次の作業は正本 TODO から一つの未対応機能を選び、失敗値・failure boundary・target・再現 command を固定する RED を先に追加する。GREEN 後に native stage0、Rust oracle、runtime/artifact、両対応 target の必要な証跡を揃える。
- 未対応機能は明示診断または明示 external boundary で止める。partial parity、Rust-only、bootstrap/oracle、verified slice を区別して TODO/docs に記録し、verified slice は `[~]` とする。完了項目は ADR へ移して TODO.md から削除し、`[x]` は使わない。
- Linux VM や stage regeneration の待機中は、同じ heavy replay を重複起動せず、artifact reuse と VM-side lock を使う。共有しない parser/type/runtime、診断、fixture、contract test、docs を並行して進める。
- 変更は task-relevant files に限定し、focused gate と docs audit の後に `main` へ commit/push する。push 後に `HEAD`、`origin/main`、worktree、TODO の残件を再監査し、未完なら次の具体的な RED と blocker を残す。
- 「Rust なしで日常開発可能」と「L# 全機能・全公開 surface が Rust-free 完了」は別の判定とする。後者は parser から公開 command、runtime、配布 provenance までの要件別 evidence が揃うまで宣言しない。

機能を置換する単位は、`RED → selfhost 実装 → focused GREEN → Rust differential lane → native stage0 dogfooding lane → artifact/runtime → 対応2 target → docs/ADR/TODO → commit/push` とする。未対応機能はまず明示診断または明示 external boundary で止め、その後に同じ observable contract を保った native 実装へ置換する。

### semantic batch と build cadence

長時間の selfhost build が進捗を隠さないよう、作業は細かな関数単位ではなく、同じ意味論と failure boundary を持つ一つの semantic family 単位でまとめる。

- 一つの batch の開始時に、対象 feature、失敗値、診断/span、exit code、artifact/runtime boundary、対応 target、再現 command を一つの RED contract に固定する。無関係な feature を同じ batch に混ぜない。
- RED は static contract と最小 runtime/E2E fixtureを同じ focused test lane にまとめる。実装後は同じ fixtureを使って GREEN、Rust oracle/differential、native stage0 dogfoodingを順に確認し、期待値を実装に合わせて変更しない。
- Cargo target と生成 artifact は batch 専用の場所を一度だけ作り、focused test の filter を変える場合も同じ target を再利用する。各関数修正のたびに `cargo build` や全 workspace test を繰り返さず、意味論 family の RED/GREEN が揃ってから必要な gate をまとめて実行する。
- Linux x86_64 の stage regeneration / VM replay は同じ仮説につき一回だけ実行する。既存の current-source artifact、VM-side lock、保守的な chunk/timeout を再利用し、重複 replayを進捗として数えない。Mac Apple Silicon の native gateも同じ batch の最終 evidence としてまとめる。
- heavy gate の待機中は、対象 artifact・VM・lockを共有しない parser/type/runtime test、diagnostic、fixture、docs audit、次 batch の候補調査を進める。完了後に結果を統合し、仮説が棄却されたら実装を残さず次の REDへ移る。
- batch の検証済み境界に到達したら、task-relevant codeとtestを commit/pushし、native evidence取得後に docs/TODOを更新して再度 commit/pushする。各 push 後に `HEAD == origin/main`、worktree、VM、artifact、TODO残件を確認する。
- 停止・中断時は、次の RED、failure value、対象 target、再現 command、blocker、残る evidence、artifact/VMの所有とcleanup状態を current docs または TODO に残す。未検証のまま「完了」と書かず、再開時は必ず status refresh から始める。

### selfhost/src 編集の local dev loop (Rust lane、evidence 非対象)

`selfhost/src` を編集して挙動を見るだけなら `cargo build` を待つ必要はない。lsharp driver は
実行ファイルの隣に `<stem>.component.wasm` があればそれを `include_bytes!` した embedded component
より優先して読む (`crates/lsharp-driver/src/main.rs` の `resolve_default_component_bytes` /
`adjacent_component_sidecar_path_for_executable`)。この sidecar を差し替える loop を
`scripts/dev-loop.sh` として用意してある。

```bash
cargo build                          # driver binary が無い初回だけ
scripts/dev-loop.sh                  # selfhost/src に変更があれば component を再生成
.lsharp-dev/bin/lsharp check foo.ls  # 以後はこの binary を使う
```

- `scripts/dev-loop.sh` は `selfhost/src` 配下の source fingerprint (算出は共有 helper
  `scripts/lib/source-fingerprint.sh` の `lsharp_source_fingerprint`。producer / consumer と同じ実体を
  source しており再実装ではない) を `.lsharp-dev/.component-fingerprint.sha256` と比較し、
  一致していれば何もしない。不一致のときだけ `LSHARP_DISABLE_EMBEDDED_COMPONENT=1` を付けて
  Rust パイプラインで component を再生成する。この env を外すと古い component 自身が新しい source を
  コンパイルすることになるので必須である。
- 生成先は `.lsharp-dev/bin/` であり `target/debug/` ではない。`target/debug/lsharp` の隣に sidecar を
  置くと、その binary を exec する driver 系 integration test の挙動が黙って変わる。
- `target/debug/lsharp` のほうが新しい場合は driver binary も再コピーする。fingerprint とは独立に
  判定するので、Rust 側だけ変更したときも古い binary を使い続けない。
- `.cargo/config.toml` に `[env] LSHARP_EMBED_COMPONENT_PATH` は置かない。repo 全体へ stale component が
  静かに効き、build.rs の `rerun-if-changed=selfhost/src` と競合する。
- **`lsharp compile` は entry file を canonical 整形して書き戻す** (`prepare_source_for_compile`、
  `crates/lsharp-tooling/src/compile.rs:221-235`。契約テストで固定された仕様)。素通しすると毎回
  `selfhost/src` が dirty になり、build.rs の `rerun-if-changed=selfhost/src` が発火して次の
  `cargo build` がフル再コンパイルになる。`scripts/dev-loop.sh` は compile 前に entry を退避して
  復元する。復元後の tree fingerprint が compile 前と一致しなければ fingerprint を記録せず `die` する
  (entry 以外まで書き換えられた場合の fail-closed)。書き戻しは compile の**前**に起きるので、
  compile が失敗しても復元は必ず走る (「整形差分あり + 型エラー」が編集中の最頻ケース)。
- **これは Rust lane の作業効率化であって Rust-free lane ではない。** 成功経路に cargo-built driver を
  使うため、この loop の結果は native gate の pass/evidence に数えない。evidence は従来どおり
  検証済み native stage0 + `scripts/native-selfhost-dev.sh` から取る。
- 契約テストは `scripts/ci/test-dev-loop.sh`。

### native emitter のバイト列を cargo 無しで読む

`selfhost/src/Backend/Native/NativeCodegen.ls` は native の機械語をバイト列リテラルで持つ。
これを読むのに cargo は要らないが、**grep で出現順に並べると誤る** — emitter は
`(concat-three-byte-vectors-rooted (byte-vector-2 ...) heap-base (byte-vector-3 ...))` のように
`let` 束縛を引数順で並べ替えるし、`read-stdin` / `int-to-string` / `string-concat` の chunk 群は
`(ref-new (vector-new N))` へ `append-encoded-u32-rooted` を積む形式でリテラルを 1 つも持たない。

`scripts/native_codegen_bytes.py` が S 式を評価して正しい並びのバイト列を返す。

```bash
python3 scripts/native_codegen_bytes.py --selftest              # 評価器の自己検証
python3 scripts/native_codegen_bytes.py --list                  # frontier を進める helper を両 lane で列挙
python3 scripts/native_codegen_bytes.py --dump <helper-name>     # 指定 helper のバイト列を hex で
```

`--list` は heap frontier の bump (aarch64 `add x22, x22, xN` / x86 `mov [r14], rN`) と
limit 参照を数えるので、`NATIVE-HEAP-01` の棚卸しに使える。helper が新しい構築形式を
使い始めたら `--list` の「評価不能」に名前が出る。**そこを空欄のまま読み飛ばすと undercount する。**

### Git worktree の配置と片付け

- 新しい worktree は `/Users/biwakonbu/github/tmp/` の直下に作成する。`/Users/biwakonbu/github/` 直下へ `lsharp-*` の作業ディレクトリを増やさない。
- feature worktree は一つの task に限定し、commit/push と統合が完了したら、clean・upstream 一致・統合先の祖先であることを確認して `git worktree remove` する。
- 継続利用する integration worktree だけを `github/tmp` に残す。aborted merge や検証専用 worktree、不要になった local branch も確認後に片付ける。
- dirty、実行中、または別セッション所有の可能性がある worktree は移動・削除しない。所有と完了を確認できない既存 worktree は、利用者と調整してから整理する。

### 現在の完遂ループ（v0.2 Evidence-driven Contracts）

現在の対象は `TODO.md` の current milestone と scheduling rules が示す最優先の未完項目から一つだけ選ぶ。特定の milestone 名を恒久的な開始点として固定せず、push 後の再監査で次の対象を更新する。

1. `TODO.md`、current worktree、対象 target の artifact/VM 状態を再確認し、対象を一つの observable contract に絞る。
2. Rust oracle と selfhost/native の双方で同じ fixture を使う RED を追加し、failure value、diagnostic/span、exit code、artifact/runtime boundary を固定する。
3. L# 実装を変更し、focused GREEN の後に Rust differential、native stage0 の `check/test/compile/build`、Wasm validate/runtime を必要な範囲で通す。
4. Mac Apple Silicon と Linux x86_64 の必要 gate を揃え、未検証の target/ABI は `[~]` と残リスクに記録する。header、summary、単一 layer test だけでは完了にしない。
5. 検証済み evidence を docs/ADR/TODO へ反映し、task-relevant 差分だけを `main` へ commit/push する。push 後に `HEAD`、`origin/main`、worktree、TODO を再監査する。
6. 長時間 gate 中は共有しない別 slice を進め、同じ仮説の重い replay を増やさない。停止時は次の RED、再現 command、blocker、残タスクを必ず記録し、完了まで再開できる状態にする。

`EC-M1-01`〜`EC-M1-07`、`LEGACY-*`、runtime/public surface の全要件を evidence で閉じるまで、Rust-free daily development 可能を全機能 Rust-free 完了とは呼ばない。

### 実装の進行規則

1. **開始時の事実確認**: 作業対象の `AGENTS.md`、`git status`、現在 branch/upstream、`TODO.md` の正本、直近の artifact/VM 状態を先に確認する。過去の完了報告や stale artifact は current evidence として再利用しない。
2. **一つの狭い仮説を一つの RED にする**: 失敗値、failure boundary、対象 target、再現 command をテストに固定する。実装を先に書かず、期待値を失敗に合わせて変更しない。
3. **待ち時間を分離する**: stage regeneration / Linux VM の heavy job は VM-side lock と artifact reuse を使って一本に制限する。実行中は parser/type/runtime の非共有 focused work、fixture、docs、diagnostic、contract test を進め、同じ仮説の full replay を重複起動しない。VM workdir、lock、巨大な一時 artifact は終了時に回収し、disk 使用量を確認する。
4. **GREEN の証拠を段階化する**: focused test、selfhost source `check`、Wasm validation/runtime、Rust oracle/differential、Mac Apple Silicon native gate、Linux x86_64 native/VM gateの順に、必要な範囲まで検証する。Rust driver の成功、summary、header、単一 layer testだけでは Rust-free 完了にしない。
5. **境界を明示する**: 未対応 feature は Rust fallback で成功したように見せず、明示的な診断または外部 tool boundary を返す。`verified slice`、`partial parity`、`Rust-only`、`bootstrap/oracle` を TODO/docs で混同しない。
6. **独立作業の分担**: サブエージェントは read-only の調査、証跡監査、独立した focused test 候補の整理に活用する。実装・docs・Git 反映は current worktree の責任範囲を一つに保ち、証拠を統合してから採用する。
7. **反映と再監査**: GREEN と docs audit の後、task-relevant files だけを commit/push する。push 後に `HEAD` と `origin/main`、worktree、TODO の残タスクを再確認し、未完のまま停止する場合は次の具体的な RED と blocker を記録する。完了宣言は要件ごとの evidence audit が終わるまで行わない。

### 完璧な L# 実装の判定

「Rust-free」は Rust のソースを早期削除することではなく、L# の対応機能が parser → 型推論 → lowering → codegen → runtime → 公開 command の全境界を通り、Mac Apple Silicon と Linux x86_64 の native program から同じ意味論で実行できることを指す。未対応の言語機能、ABI、公開 surface、component/external helper、bootstrap provenance が残る間は TODO.md の `[~]` を維持し、各項目の parity と実行証跡を閉じてから ADR へ移して TODO.md から削除する。

### 完遂ロードマップ

完璧な実装へは、便利な周辺機能からではなく、意味論と検証境界を先に閉じる次の順序で進める。

1. **言語契約**: lexer/parser、型推論、診断 code/span、module/import、metadata の Rust/native parity を閉じる。未完の `LS####` 診断体系、GADT exhaustiveness、HKT、computation expression、trait の動的境界はこの段階の対象とする。
2. **実行意味論**: record/ADT/pattern、Map、closure、GC、linear-memory ABI を lowering → Wasm codegen → runtime の順に閉じ、source/ftable/import と両対応 target の actual E2E を揃える。単なる AST/IR snapshot で終了しない。
3. **自己ホスト compiler**: legacy `lower`、full-program builder、module graph、component sidecar、standalone I/O と dynamic memory layout を実成果物で閉じる。Rust driver fallback が成功を隠していないことを negative test でも確認する。
4. **公開 surface**: `compile` / `build` の全 supported output、`test`、`doc`、`repl`、`lsp --stdio`、`install`、必要な external tool boundary を実 native program で検証する。`mcp-server` や unsupported target は、未実装のまま曖昧に成功させず明示拒否または Rust host integration として分類する。
5. **配布と最終監査**: stage0 の source provenance、取得、再生成、rollback、Mac/Linux release artifact を固定し、TODO の全 `[~]` / `[ ]` を要件単位で再監査する。全項目の evidence が揃うまで「完全対応」「Rust 完全撤去」と宣言しない。

各段階では、最初に次の一つの RED を選び、GREEN の直後に Rust oracle/native target/runtime の必要な証拠を追加する。長時間の VM gate が必要な場合も、待機中に次の段階の非共有 focused test や診断を進め、重い replay を進捗として数えない。

### 完璧な実装へ向けた運用契約

「Rust を外す」は Rust のコードを先に削除することではない。L# で実際に開発を続けられるよう、検証済み native stage0 を日常開発の入口にして、未対応の機能は Rust fallback で隠さず、明示的な診断または明示的な外部境界として残す。完璧な実装とは、対応 target で parser → 型推論 → lowering → codegen → runtime → 公開 command と成果物が一貫して実行でき、配布用 stage0 の provenance・再生成・rollback まで検証済みの状態である。

作業は次の順序で進める。

1. 開始時に `git status`、branch/upstream、`TODO.md`、対象 target の native stage0・artifact・VM lock/job 状態を確認する。過去の報告や stale artifact を現状の証拠にしない。
2. TODO から一つの observable contract だけを選び、失敗値、診断 code/span、exit code、artifact/runtime boundary、対象 target、再現 command を固定した RED を先に追加する。
3. selfhost/native の実装を進め、focused test、Rust oracle/differential、native stage0、Wasm validate/runtime、必要な Mac Apple Silicon と Linux x86_64 gate を順に通す。summary/header/単一 layer test だけでは完了にしない。
4. 検証済み evidence だけを docs/ADR/TODO に反映し、`[~]` は partial parity、Rust-only、bootstrap/oracle、external boundary、未検証 ABI を明示する。要件全体の evidence が揃うまで `[x]` や「完全対応」を使わない。
5. task-relevant files だけを `main` に commit/push し、push 後に `HEAD == origin/main`、worktree、TODO 残件を再監査する。ユーザーの既存差分は保持し、force push や無関係な整理はしない。

長時間の stage regeneration / Linux VM gate は仮説ごとに一つだけ実行し、VM-side lock と既存 artifact を再利用する。待機中は同じ replay を重複起動せず、共有しない parser/type/runtime、診断、fixture、contract test、docs を進める。停止する場合は、次の RED、再現 command、blocker、対象 artifact/target、必要な evidence を current docs に残し、次の run は必ず status refresh から再開する。VM の一時 workdir と巨大 artifact は gate 後に回収し、disk 使用量も確認する。

### CI と手動 release の境界

- 当面の supported target build/release は GitHub Actions の CI で実行せず、Mac Apple Silicon 上の native stage0、必要な Linux x86_64 Lima VM gate、ローカルの focused test を正本とする。CI の green、workflow の存在、または CI が生成した stale artifact だけを Rust-free 完了や release readiness の証拠にしない。
- GitHub Actions は既存の静的検査・docs 契約を壊さない範囲で保持するが、無料枠を消費する native selfhost replay や release build を自動起動しない。release は対象 target、source provenance、artifact/runtime の local evidence を確認してから手動で行う。
- ローカル VM は同じ仮説の job を一つに制限し、既存 artifact を再利用する。job 完了後は tmux/process、lock、temporary workdir、disk 使用量を確認し、アイドル VM は停止する。容量変更は実測値と次回 gate の必要量に基づいて行い、広い resize/recreate を推測で実施しない。

### 完了監査の必須条件

完了宣言は、実装が存在することや focused test が通ることではなく、要求された境界ごとの evidence が揃ったことを意味する。各タスクの完了前に次を確認する。

- TODO、仕様、Issue、ADR に書かれた各 requirement・target・公開 command・artifact/runtime boundary を列挙し、それぞれの証拠が current checkout の実ファイル、実行結果、生成物、runtime であることを確認する。
- unit test、Rust driver の成功、summary/header、stale artifact、Rust fallback の成功を、native stage0、Wasm validate/runtime、Mac Apple Silicon、Linux x86_64 の証拠へ拡大解釈しない。証拠の scope が要件の scope と一致しない場合は未完了とする。
- 未対応、partial parity、Rust-only、bootstrap/oracle、external boundary、未検証 ABI を分類し、未完了のまま `[x]` や完了移行を行わない。完了項目だけを ADR に移し、TODO から削除する。
- 同じ blocker が解消していない場合でも、停止時は次の RED、再現 command、blocker、残る target/evidence を記録する。再開時はその記録と current state を照合し、重複した heavy replay を起動しない。

### 継続作業と終了条件

- 明示的な一時停止指示がない限り、partial parity や blocker が残った時点で作業を完了扱いにしない。各継続ターンは `git status`、`TODO.md`、target artifact/VM 状態を refresh し、次の具体的な RED または識別実験へ進む。
- heavy gate の待機中は同じ仮説の replay を増やさず、非共有の parser/type/runtime、診断、fixture、contract test、docs、証跡監査を進める。gate 完了後に結果を統合し、仮説が棄却された場合は実装を残さず次の仮説へ移る。
- blocker で止まる場合でも、failure value、failure boundary、対象 target、再現 command、次の識別実験、残る evidence を current docs に記録する。停止は未完了の省略ではなく、再開可能な状態を作るための記録工程とする。
- 「Rust なしで日常開発できる verified slice」は中間状態であり、全機能・全公開 surface・両対応 target・配布 provenance の完了監査を通るまで Rust-free 完了や Rust 削除を宣言しない。
- 完了判定は実装や単一テストの存在ではなく、TODO/仕様/Issue/ADR の各 requirement と target/evidence の current audit が終わり、残タスクが次の未完項目として残っていないことを確認してから行う。

## hooks/スキルのトラブルシューティング

hooks やスキルに問題が発生した場合は `.Codex/rules/hook-troubleshooting.md` を参照。
注意: hook の stderr 出力 ([TDD Guard], [TDD Tracker]) は正常な情報メッセージであり、エラーとして対処する必要はない。

## ファイルサイズ制限

- 1 ファイルあたり **500〜800 行**に収める
- これを超えるとエージェントの解析精度が落ちるため、早めにモジュール分割・リファクタリングを行う
- 新規実装時も既存ファイルが肥大化しないよう注意する

## 主要依存関係

- `miette`: ソーススパン付きリッチエラーレポート
- `wasm-encoder`: WebAssembly バイナリ生成
- `wasmtime` + `wasmtime-wasi`: Wasm 実行ランタイム
- `insta`: スナップショットテスト
- `clap`: CLI 引数パース
- `tower-lsp`: LSP サーバーフレームワーク

## 言語機能

- S 式構文 (Clojure 風)
- ADT + パターンマッチ → リニアメモリ上の struct (タグによる判別)
- レコード型 → リニアメモリ上の struct
- モジュールシステム: `(module Name)`, `(import Module)`, `(open Module)`
- トレイト: 辞書引数による静的ディスパッチ
- 計算式: `let!` によるモナディックバインド
- メタデータ: `:doc`, `:example`, `:invariant`, `:transitions`
