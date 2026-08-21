# L# Active Backlog

このファイルは、**未完了タスクだけ**を持つ単一正本である。完了した項目は判断・結果・代表 evidence を
[`docs/adr/`](docs/adr/) または対応する仕様・運用記録へ残し、このファイルから削除する。

状態:

- `[ ]`: 未着手。次の RED と observable contract をまだ固定していない
- `[~]`: verified slice はあるが、項目全体の completion boundary を満たしていない
- `[BLOCKED: 理由]`: 外部状態または明示的な依存待ち

`[x]` は使わない。日付別の進捗ログ、個別 test 名、artifact hash、完了済み phase はここへ蓄積せず、
設計、ADR、test、artifact、運用記録を参照する。

## Checkpoint — 2026-08-12

ここは一度区切って再開するための current truth である。完了済みの細かな test 名や hash を backlog の完了項目へ
昇格させず、判断と代表 evidence は ADR へ置く。

### 現在地

- 確認時点の code checkpoint は `47743365`（`4e2d0cf3` の selfhost validation serializer state 分離で default EmbeddedCli build blockerを解消し、`47743365` で Rust CLI の `validate --source` を複数 `.ls` source file の deterministic project aggregateへ拡張）。単一 file と directory の source collection、file 境界を越えた duplicate intent node の code `2`、first/duplicate span、stdout空・manifest未生成の Rust CLI contract は focused test と `validate_cli` 全体で検証済みである。Mac Apple Silicon の同 source checkpoint native App.Cli release は selfhost fixed point と native core runtime matrix `44 cases` を確認済みである。Linux x86_64 replayは stage1完了後に actual stage2/stage3 summaryを回収できず、current-source fixed point と project aggregate の Linux runtime evidence は未検証である。
- 今回の Linux VM はジョブ終了後に停止し、`lsharp-linux-x86` は `Stopped`、replay lock と task-owned VM workdir は残っていない。VM は使用量約 `3.5 GiB` / 空き約 `7.2 GiB` で、次回は同じ仮説の replay を重複起動せず、current-source summary の有無を先に確認する。
- 共有 root checkout は他セッションの競合・未保存差分を含むため編集対象にしない。新規 worktree、`target`、一時 checkout は
  `/Users/biwakonbu/github/tmp/<task>/` に限定し、完了後は自分の所有物だけを片付ける。
- Cloud で narrow contract の実装と task-only commit を作り、local task-owned worktree で結果を適用し、まとめて検証して
  `main` へ push する。Cloud の HTTPS credential がない場合も commit SHA と検証結果を保存して local 適用へ切り替える。
- Rust/native parity、native no-execution、必要な artifact/runtime boundary、対象 target の evidence が揃うまで、
  Rust-free 完了や項目全体の完了を宣言しない。

### 直近の verified partial（完了項目には移さない）

- installed package の既存 `docs/api.json` は regular non-symlink file のみを package-owned metadata として読む。
- `.lsharp/packages/<entry>` 自体は regular non-symlink directory のみを discovery 対象にする。
- regular package directory 内の `lsharp.toml` symlink は discovery から除外し、explicit package API は既存 not-found で fail-closed にする。
- regular package directory 内の root `src/` directory symlink は source-owned tree として扱わず、外部 `.ls` の API projection と native `doc` 実行を止める。

代表 ADR: [`decisions-v0.3-native-mcp-package-api-regular-file-boundary.md`](docs/adr/decisions-v0.3-native-mcp-package-api-regular-file-boundary.md)、
[`decisions-v0.3-native-mcp-installed-package-directory-ownership.md`](docs/adr/decisions-v0.3-native-mcp-installed-package-directory-ownership.md)、
[`decisions-v0.3-native-mcp-package-manifest-symlink-boundary.md`](docs/adr/decisions-v0.3-native-mcp-package-manifest-symlink-boundary.md)、
[`decisions-v0.3-native-mcp-package-src-directory-ownership.md`](docs/adr/decisions-v0.3-native-mcp-package-src-directory-ownership.md)。

### 再開時の次の一件

`47743365` の verified partial は Rust driver の project source aggregation である。directory は regular `.ls` fileを
deterministicに収集し、全 node を先に登録してから evidence/review/edge を解決する。cross-file duplicate node の
fail-closed diagnostic は Rust CLI で固定したが、valid な複数 file graph の report/manifest、cross-file typed edge、
duplicate evidence/review の source-specific diagnostics、selfhost/native App.Cli・EmbeddedCli・MCP parityは未接続である。
次のREDは、異なる `:intent` IDを持つ2 fileとcross-file edgeを一つの projectとして受理し、deterministic report と
`--emit-manifest` の node/edge/source provenance を生成する Rust CLI contractである。再現予定 command は
`cargo test -p lsharp-driver --test validate_cli validate_accepts_project_directory_with_cross_file_edge -- --nocapture` とする。
その後、current-source Linux x86_64 fixed-point summaryを一度だけ取得し、native public directory validationを別の
observable contractとして追加する。別の type diagnostic span、全rule code/message parity、component/packaged parity、
Rust-free aggregateは未完了であり、V2-16b / V2-16c / V2-16eは[~]のまま維持する。

#### これまでの standard projection evidence

これまでの verified partial として、1409e18b / d3e852a6 で review の unused-let lint diagnostic を標準 LSP Diagnostic object へ投影し、
続く test-only batch で empty-do (`L0002`) も同じ wire fields へ固定した。Rust actual bundle の focused E2E は
`test_e2e_selfhost_cli_lsp_stdio_didopen_publishes_standard_empty_do_diagnostic` が 1 passed / 430.65s となり、point range、
severity 2、code L0002、source lsharp、message "do block has no expressions" を確認した。Mac Apple Silicon の
`ci-artifacts/native-release/aarch64-apple-darwin/current-d3e852a6-lsp-lint/program.native` は production source commit
`d3e852a6572cbdf4ea705eee851d67230b20772e` と一致し、後続 `c50b7b3c` は docs-only、selfhost_fixed_point=true、artifact 4,748 KiB、
stderr 0、native core matrix 27 cases 全 pass だった。

さらに `43ef943e` で、ハイフン付き束縛名 `(let [unused-a 42] 0)` の L0001 message 復元を同じ契約へ追加した。
Rust actual bundle の `test_e2e_selfhost_cli_lsp_stdio_didopen_preserves_hyphenated_lint_name` は 1 passed / 378.17s で、
既存の hash 逆変換に基づく message `let binding unusebka is not used` を固定した。保存済みの同じ Mac artifact と Linux x86_64
App.Cli ELF に native runner を再適用し、unused-let、hyphenated unused-let、empty-do を含む 28 cases 全 passだった。
この batch は `selfhost/src` を変更していないため、stage regeneration は実行せず、既存 fixed-point artifact を replay-only で再利用した。

続く `86139edb` で、標準 type Diagnostic の LS1001 (`undefined symbol`) と LS1002 (`if condition must be Bool`) を同じ didOpen wire
contractへ追加した。Rust actual bundle の `test_e2e_selfhost_cli_lsp_stdio_didopen_publishes_standard_type_diagnostics` は
1 passed / 348.51s、Mac Apple Silicon と Linux x86_64 の保存済み App.Cli artifactは 30 cases 全 passだった。

さらに `e2cef471` の test-only batch では、parse source `")"` の LS0101 (`unexpected token )`) を同じ didOpen wire contractへ
追加した。Rust actual bundle の `test_e2e_selfhost_cli_lsp_stdio_didopen_publishes_standard_parse_diagnostic` は 1 passed / 342.45s で、
range `0:0..0:1`、severity 1、code LS0101、source lsharp、message `unexpected token )` を固定した。Mac Apple Silicon と
Linux x86_64 の保存済み App.Cli artifactへ runnerを再適用した native core runtime matrixは 31 cases 全 passだった。
`selfhost/src` は変更していないため stage1→stage3 regenerationは実行せず、既存 fixed-point artifactをreplay-onlyで再利用した。

Linux x86_64 では ci-artifacts/native-linux-x86-hostgen-vm/d3e852a6-lsp-lint-current/actual-selfregen-summary.json に
target x86_64-unknown-linux-gnu、host Linux/x86_64、status=pass、stage2/stage3 code length 各 11,448,943、
stdout SHA-256 各 a66bf8c746a9cf91a6b0cdb0509a9f12b3b7987301f025646d69fdffd1c6677e、stderr 空の一致を記録した。
保存済み stage2 を再利用した target-only App.Cli materialize の
ci-artifacts/native-linux-x86-hostgen-vm/d3e852a6-lsp-lint-cli/manifest.json は source commit
d3e852a6572cbdf4ea705eee851d67230b20772e、selfhost_fixed_point=true、code 13,375,178、
program SHA-256 b155abe13cb16c71f6c34e02152b33b4f819c9a8cceb769386740317f3a6f988、--version smoke stderr 0 を記録した。
同じ Linux ELF を VM 内で native core matrix 31 cases、type builtins 5 tests、MCP 6 requests 全 pass した。
target-only lane は保存済み stage2 と VM-side lock を再利用し、Seed fixed-point の重複 replay を避けた。

これは unused-let (`L0001`)、empty-do (`L0002`)、type (`LS1001`/`LS1002`)、parse (`LS0101`) の標準 wire projectionに限定した verified partial であり、複数 lint 診断の順序/dedup、
他の parse/type/lint rule の正確な span end、全 rule code/message parity、全 diagnostics/type/lint parity、definition/references/rename の全 semantic projection、
component/packaged release parity、Rust-free aggregate は未完了のため V2-16b / V2-16c / V2-16e は [~] のまま維持する。
Evidence commits: 1409e18b, d3e852a6, 6e09ff86, 1b6784db, 43ef943e, 86139edb, e2cef471.

さらに `8850c7d4` では、標準 single-document LSP fixture `(defn helper [x] x)` / `(defn main [] (helper 1))` の definition responseを
URI付き `Location` objectへ投影する contractを追加した。REDでは native App.Cli が参照側 definition rangeを `0:0..0:0` と返したため、
`lsp-find-defn-offset-before-loop` の accumulatorを同名の局所束縛で隠していた failure boundaryを特定し、最小の `next-match` 更新へ修正した。
Rust actual bundleの `test_e2e_selfhost_cli_lsp_stdio_standard_uri_navigation_contract` は修正後 1 passed / 346.13s で、definitionの
URI `file:///tmp/lsharp-uri-contract.ls` と zero-based range `0:6..0:6` を固定した。

Mac Apple Siliconの current-source actual release gateは 1 passed / 919.31s、native core runtime matrixは 32 cases全 passだった。
Linux x86_64の `ci-artifacts/native-linux-x86-hostgen-vm/8850c7d4-lsp-definition/actual-selfregen-summary.json` は target
`x86_64-unknown-linux-gnu`、host `Linux/x86_64`、status `pass`、stage2/stage3 code length各 `11,448,943`、stdout SHA-256各
`a66bf8c746a9cf91a6b0cdb0509a9f12b3b7987301f025646d69fdffd1c6677e`、stderr空の一致を記録した。保存済みstage2をVM-side lock付きで
再利用した `ci-artifacts/native-linux-x86-hostgen-vm/8850c7d4-lsp-definition-cli/manifest.json` は source commit `8850c7d4...`、
target `x86_64-unknown-linux-gnu`、selfhost_fixed_point=true、source tree SHA-256
`52f77188d7a54a9b8d4659853368102f3890a6fc916ca230dbd35ba6ddcf9b58`、code `13,375,205`、program SHA-256
`b4268dceb3f8e2ecf4f254d28a10a86f6be630870b2b1a0857aae79e3dc78081`、stderr 0を記録し、`--version` smokeと同じLinux ELFの native
core runtime matrix 32 cases全 passを確認した。Linuxゲート後は task-owned VM workdir、program/script、replay lockを回収し、12GiB VMは
停止状態で使用量 3.5GiB / 空き 7.2GiBだった。

さらに test-only `5922ad15` では、同じ current-source App.Cli artifactへ standard `references` と `rename` requestを追加し、5 framesの
definition・references・renameを一つのLSP processで実行した。Mac Apple Silicon と Linux x86_64 の native core runtime matrixは各 32 cases
全 passで、selfhost sourceは変更していないため stage regenerationは重複実行していない。

これは標準 single-document definition/references/rename の URI/location/workspace edit projectionに限定した verified partialであり、full
symbol range、cross-document URI provenance、全 diagnostics/type/lint parity、component/packaged release parity、Rust-free aggregateは
未完了のため V2-16b / V2-16c / V2-16e は [~] のまま維持する。Evidence commits: 8850c7d4, 5922ad15.

さらに test-only `5dc93e27` では、標準 type Diagnostic の LS1004 (`function argument type mismatch`) を didOpen wire contractへ追加した。
Rust actual bundleの `test_e2e_selfhost_cli_lsp_stdio_didopen_publishes_standard_type_diagnostics` は 1 passed / 333.40s、Mac Apple Silicon と
Linux x86_64 の current-source App.Cli artifactへ同じ native runnerを適用した native core runtime matrixは各 32 cases全 passだった。
`selfhost/src` は変更していないため stage regenerationは重複実行していない。

これは LS1004 ひとつの標準 Diagnostic projectionに限定した verified partialであり、複数診断の順序/dedup、他の parse/type/lint ruleの正確な
span end、全 rule code/message parity、component/packaged release parity、Rust-free aggregateは未完了のため V2-16b / V2-16c / V2-16e は
[~] のまま維持する。Evidence commit: 5dc93e27.

さらに test-only `468279b0` では、parameterized self-application `(defn main [x] (x x))` の infinite typeを同じ didOpen wire contractへ追加した。
Rust actual bundleの `test_e2e_selfhost_cli_lsp_stdio_didopen_publishes_standard_type_diagnostics` は `1 passed / 341.59s`、Mac Apple Siliconの
保存済み `8850c7d4` App.Cli artifactとLinux x86_64の同じsource checkpointから作成したApp.Cli ELFへ native runnerを適用し、LS1003を含む
native core runtime matrixは各 `33 cases` 全 passだった。zero-based point range、severity 1、code `LS1003`、source `lsharp`、message
`infinite type` をRust/native双方で固定し、selfhost sourceは変更していないためstage regenerationは実行していない。Linux replay後はVM workdirを回収し、
`lsharp-linux-x86` を停止した。

これは parameterized self-applicationのLS1003 standard Diagnostic projectionに限定した verified partialである。zero-argument defn内の
lambda self-application `(defn main [] (fn [x] (x x)))` は保存済みnative artifactで `LS1002` / `type error` へ丸められるため、lambda/defn経路全体の
infinite-type parity、複数診断の順序/dedup、他のparse/type/lint ruleの正確なspan end、全rule code/message parity、component/packaged release parity、
Rust-free aggregateは未完了であり、V2-16b / V2-16c / V2-16eは[~]のまま維持する。Evidence commit: `468279b0`。

続く test-only `e28835bd` では、standard parse Diagnosticの unexpected close `]`（LS0101）と未閉じ vector `[`（LS0102）を同じ didOpen wire contractへ追加した。
Rust actual bundleの `test_e2e_selfhost_cli_lsp_stdio_didopen_publishes_standard_parse_diagnostic` は `1 passed / 329.99s`、保存済み Mac Apple Silicon
App.Cli artifactとLinux x86_64 App.Cli ELFへ native runnerを適用した native core runtime matrixは各 `35 cases` 全 passだった。`]` は range
`0:0..0:1`、`[` は point range `0:1..0:1` とし、severity 1、source `lsharp`、message `unexpected token ]` / `unexpected input end` を固定した。
selfhost sourceは変更していないためstage regenerationは実行せず、Linux replay後はVM workdirを回収して`lsharp-linux-x86`を停止した。

これは parse LS0101 / LS0102 の標準 projectionに限定した verified partialであり、LS0103 unknown-form、LS0104 multiple-parse-errors、複数診断の順序/dedup、
他のparse/type/lint ruleの正確なspan end、全rule code/message parity、component/packaged release parity、Rust-free aggregateは未完了である。
V2-16b / V2-16c / V2-16eは[~]のまま維持する。Evidence commit: `e28835bd`。

続く test-only `c1982e1d` では、`(defn main [`、`(`、`(defn main [] (do` の3つの文脈で unexpected EOF（LS0102）の point spanを追加固定した。
Rust actual bundleの同一focused testは `1 passed / 329.90s`、保存済み Mac Apple Silicon App.Cli artifactとLinux x86_64 App.Cli ELFの
native core runtime matrixは各 `38 cases` 全 passだった。point rangeは順に `0:12`、`0:1`、`0:17`、severity 1、source `lsharp`、message
`unexpected input end` を確認した。selfhost sourceは変更していないためstage regenerationは実行せず、Linux replay後はVM workdirを回収して
`lsharp-linux-x86` を停止した。証跡artifactの使用量はMac側52MiB、Linux target-only ELF側13MiBだった。

この batchも LS0102 の複数構文文脈に対する標準 projectionの verified partialに限定される。LS0103 unknown-form、LS0104 multiple-parse-errors、
複数診断の順序/dedup、他のparse/type/lint ruleの正確なspan end、全rule code/message parity、component/packaged release parity、Rust-free aggregateは
未完了であり、V2-16b / V2-16c / V2-16eは[~]のまま維持する。Evidence commit: `c1982e1d`。

続く `48d48bb1` では、先頭の unknown top-level form `(unknown-form)` を raw parse diagnostic `1003` として生成し、標準 LSP Diagnosticの
range `0:1..0:13`、severity 1、code `LS0103`、source `lsharp`、message `unknown form` へ投影した。Rust actual bundleの
`e2e::selfhost_cli_core::test_e2e_selfhost_cli_lsp_stdio_didopen_publishes_standard_parse_diagnostic` は `1 passed / 327.96s`。
Mac Apple Siliconの current-source App.Cli release gateは `1 passed / 857.41s`、manifest source commitは `48d48bb1...`、
`selfhost_fixed_point=true`、native core runtime matrixは `39 cases` 全 passだった。

Linux x86_64の current-source actual self-regeneration summaryは
`ci-artifacts/native-linux-x86-hostgen-vm/48d48bb1-lsp-unknown/actual-selfregen-summary.json` に target `x86_64-unknown-linux-gnu`、
host `Linux/x86_64`、status `pass`、stage2/stage3 code length各 `11,451,770`、stdout SHA-256各
`30ed6c388b1e69ac19a11630e838685d4f9c8ebd75cb3286db5b9e9c7237ec83`、stderr空の一致を記録した。VM free-space gateは
available `7,658,594,304` / required `4,294,967,296` bytesだった。stage2をVM-side lock付きで再利用した target-only App.Cli artifactの
`ci-artifacts/native-linux-x86-hostgen-vm/48d48bb1-lsp-unknown-cli/manifest.json` は source commit `48d48bb1...`、
source tree SHA-256 `f538dd95ae7e3bfe7074248083b82fcb14db70c980d58369ed5af6d68074ccba`、`selfhost_fixed_point=true`、
code `13,378,933` bytes、program SHA-256 `d4296870f1a95815872ee1e4d2a4ecbe225adc0c2d0f9646a21253a7ddcddd14`、
`--version` smoke `lsharp 0.1.0`、stderr 0を記録した。同じ Linux ELFをVM内で実行した native core runtime matrixは `39 cases` 全 passで、
検証後はrunnerの一時領域を削除し、`lsharp-linux-x86` を停止した。selfhost source変更を含むため stage1 -> stage2 -> stage3 replayは一度だけ実行した。

これは先頭の unknown top-level formに対する LS0103 standard Diagnostic projectionと、同一 current-source App.Cli artifactの両対応target
runtimeに限定した verified partialである。後続 top-level form、LS0104 multiple-parse-errors、複数診断の順序/dedup、他のparse/type/lint ruleの
正確なspan end、全rule code/message parity、component/packaged release parity、Rust-free aggregateは未完了である。V2-16b / V2-16c / V2-16eは
[~]のまま維持する。Evidence commit: `48d48bb1`。

### V2-16b / V2-16c native standard parse LS0104 multiple-parse-errors projection (2026-08-10)

`b6cfb12a` で、複数の malformed top-level `defn` signature `(defn [) (defn [)` をRust parserの`Multiple`相当として検出し、最初の失敗位置
`0:6..0:7`を保持したraw parse diagnostic `1004`から、標準 LSP Diagnosticのseverity 1、code `LS0104`、source `lsharp`、message
`multiple parse errors`へ投影した。検出はこの繰り返し`defn` signatureの形に限定し、既存のdelimiter/unknown-form/recovery経路は広げていない。
Rust actual bundleの`e2e::selfhost_cli_core::test_e2e_selfhost_cli_lsp_stdio_didopen_publishes_standard_parse_diagnostic`は`1 passed / 334.31s`だった。

Mac Apple Siliconのcurrent-source App.Cli release gateは`1 passed / 1009.01s`、artifact
`ci-artifacts/native-release/aarch64-apple-darwin/current-9182bb2b-lsp-multiple/manifest.json`はtarget `aarch64-apple-darwin`、source commit
`9182bb2b...`、`selfhost_fixed_point=true`、program SHA-256
`cf35bb64954e2e8412d59a9d8653a4f5261449b3e0dda706adedc7f063ae504e`、artifact `4,748 KiB`を記録した。同じMac programへnative core runtime
matrixを適用し、LS0104を含む`40 cases`全pass、stderr空を確認した。

Linux x86_64のcurrent-source actual self-regeneration summaryは
`ci-artifacts/native-linux-x86-hostgen-vm/b6cfb12a-lsp-multiple/actual-selfregen-summary.json`にtarget `x86_64-unknown-linux-gnu`、host
`Linux/x86_64`、status `pass`、stage2/stage3 code length各`11,465,699`、stdout SHA-256各
`813f14713001b31e1e2988efeaaf8fbc8486e4499e3de186736cec81bb686fc4`、stderr空の一致を記録した。verified stage2をVM-side lock付きで再利用した
target-only App.Cli artifactの
`ci-artifacts/native-linux-x86-hostgen-vm/b6cfb12a-lsp-multiple-cli/manifest.json`はsource commit `b6cfb12a...`、source tree SHA-256
`9eeb550a3218ab824ef2faa8768e50b35a1bee4e73af1b38a8a0cfedbe12e745`、`selfhost_fixed_point=true`、code `13,393,023` bytes、program SHA-256
`922da0904d40b97ce3967686a57c8892ee4fd935856746be138307173dc186fc`、`--version` smoke、stderr 0を記録した。同じLinux ELFをVM内で実行した
native core runtime matrixはLS0104を含む`40 cases`全passだった。VM free-space gateはavailable約`7.2 GiB` / required `4 GiB`で、runner/workdirと
replay lockを検証後に回収し、`lsharp-linux-x86`を停止した。

これは繰り返しmalformed top-level `defn` signatureのLS0104 standard Diagnostic projectionと、同一current-source App.Cli artifactの両対応target
runtimeに限定したverified partialである。その他の複数parse error形状、複数診断の順序/dedup、他のparse/type/lint ruleの正確なspan end、全rule code/message parity、
component/packaged release parity、Rust-free aggregateは未完了である。V2-16b / V2-16c / V2-16eは[~]を維持する。Evidence commits: `b6cfb12a`, `9182bb2b`。

### V2-16b / V2-16c native standard LS1002 if-branch Diagnostic span projection (2026-08-10)

`368ac2c0` で、`(defn main [] (if true 1 false))` のbranch unification failureにif式の開始・終了offsetを保持させ、selfhostのTypeInfer結果から
標準LSP Diagnostic range `0:14..0:31`へ投影した。既存のLS1002 if-condition、LS1004 argument mismatch、LS1001 undefined symbol、LS1003 infinite typeの
point range契約は変更していない。Rust actual bundleの
`test_e2e_selfhost_cli_lsp_stdio_didopen_publishes_standard_type_diagnostics` は `1 passed / 344.07s` だった。

Mac Apple Siliconのcurrent-source App.Cli release gateは `1 passed / 879.61s`。artifact
`ci-artifacts/native-release/aarch64-apple-darwin/current-368ac2c0-lsp-if-branch/manifest.json` は target `aarch64-apple-darwin`、source commit
`368ac2c053b928f670e13e93a0899a61ff30addd`、`selfhost_fixed_point=true`、program `4,393,216` bytes、program SHA-256
`f9158e72488a971e5a24333ae08163cfb1b41b2e39e111c4e14c56b88cc291fe`、stderr 0を記録した。同じMac programのnative core runtime matrixは
LS1002 branchを含む `41 cases` 全passだった。後続 `7c830835` はrunnerの件数表示だけを41 casesへ補正したcommitで、selfhost sourceの内容は変えていない。

Linux x86_64のcurrent-source actual self-regeneration summaryは
`ci-artifacts/native-linux-x86-hostgen-vm/7c830835-lsp-if-branch/actual-selfregen-summary.json` に target `x86_64-unknown-linux-gnu`、host
`Linux/x86_64`、status `pass`、stage2/stage3 code length各 `11,466,310`、stdout SHA-256各
`f7b2688f8d65ddbeca91ef51ecc62b6d8a9406dd5cb1eb21d54308aee95d1ecc`、両stderr 0を記録した。stage2/stage3 manifestは source commit
`7c83083501b42979567459bfe183a542536279ef`、data `2,757` bytes、entrypoint `11,461,702`、function-start length `3,430`、main function index
`3,439`で一致した。verified stage2をVM-side lock付きで再利用した target-only App.Cli artifactの
`ci-artifacts/native-linux-x86-hostgen-vm/7c830835-lsp-if-branch-cli/manifest.json` は source tree SHA-256
`9182b2b5f25ceb95357bb1e382026cc6966cb6bd0f212e13283897af4278d72b`、`selfhost_fixed_point=true`、code `13,395,487` bytes、program
`13,438,760` bytes、program SHA-256 `b29a5e4cb878c780cb66e37a14fc27416960f5fd5fcedc4d2fc15ff259aaa0e7`、`--version` smoke
`lsharp 0.1.0`、stderr 0を記録した。同じLinux ELFをVM内で実行したnative core runtime matrixは `41 cases` 全passだった。
VM free-space gateは available `7,670,906,880` / required `4,294,967,296` bytesで、target-only後のVM workdir、runner、replay lockを回収して
`lsharp-linux-x86`を停止した。

これはif branch mismatchのLS1002 range projectionと、同一current-source App.Cli artifactの両対応target runtimeに限定したverified partialである。
LS1002 branch以外の正確なspan、複数diagnosticの順序/dedup、全rule code/message parity、parser/type/lint全体のdiagnostic parity、component/packaged
release parity、Rust-free aggregateは未完了である。V2-16b / V2-16c / V2-16eは[~]を維持する。Evidence commits: `368ac2c0`, `7c830835`。

### V2-16b / V2-16c native standard LS1001 undefined-symbol Diagnostic span projection (2026-08-10)

REDでは、`(defn main [] missing)` の undefined-symbol diagnosticがnative App.Cliからpoint range `0:0..0:0`で返る差分を固定し、期待する
exact range `0:14..0:21`をRust/native共通のdidOpen wire contractへ追加した。Rust actual bundleの
`test_e2e_selfhost_cli_lsp_stdio_didopen_publishes_standard_type_diagnostics` は `0 passed / 1 failed / 453.73s` となり、failure valueを確認した。
`a0a3dd55` では、selfhost `App.Cli` のLS1001/LS1002 span projectionだけを拡張し、既存のpoint-range type diagnostic契約を変更せずに修正した。
同じRust focused E2Eは `1 passed / 375.44s` となった。

Mac Apple Siliconのcurrent-source App.Cli release gateは `1 passed / 932.19s`。artifact
`ci-artifacts/native-release/aarch64-apple-darwin/current-a0a3dd55-lsp-undefined/manifest.json` は target `aarch64-apple-darwin`、source commit
`a0a3dd55b25443669ce081cafae03b3d7f59660e`、`selfhost_fixed_point=true`、program `4,393,216` bytes、program SHA-256
`d90da889334ab53fa93686f2ccab768afb909f2557c0674de428db06975ff3e2`を記録した。同じMac programのnative core runtime matrixはLS1001を含む
`41 cases`全pass、stderr空だった。

Linux x86_64のcurrent-source actual self-regeneration summaryは
`ci-artifacts/native-linux-x86-hostgen-vm/a0a3dd55-lsp-undefined/actual-selfregen-summary.json`にtarget `x86_64-unknown-linux-gnu`、host
`Linux/x86_64`、status `pass`、stage2/stage3 code length各`11,466,310`、stdout SHA-256各
`f7b2688f8d65ddbeca91ef51ecc62b6d8a9406dd5cb1eb21d54308aee95d1ecc`、両stderr 0を記録した。stage2/stage3 manifestは source commit
`a0a3dd55b25443669ce081cafae03b3d7f59660e`、data `2,757` bytes、entrypoint `11,461,702`、function-start length `3,430`、main function index
`3,439`で一致した。verified stage2をVM-side lock付きで再利用した target-only App.Cli artifactの
`ci-artifacts/native-linux-x86-hostgen-vm/a0a3dd55-lsp-undefined-cli/manifest.json`は source tree SHA-256
`1923a45103b2293ca32f05d5d339bbca0f4e57d582806f90a247dacd0130f79c`、`selfhost_fixed_point=true`、code `13,395,530` bytes、program
`13,438,760` bytes、program SHA-256 `266d5567f09be6a6c3497d7707f4c756d9a27ab29c4f892cf085f1b526d71411`、`--version` smoke
`lsharp 0.1.0`、stderr 0を記録した。同じLinux ELFをVM内で実行したnative core runtime matrixは`41 cases`全passだった。VM free-space gateは
available約`7.1 GiB` / required `4 GiB`で、target-only後のVM workdir、runner、replay lockを回収して`lsharp-linux-x86`を停止した。

これはundefined symbolのLS1001 exact range projectionと、同一current-source App.Cli artifactの両対応target runtimeに限定したverified partialである。
LS1001以外の正確なspan、複数diagnosticの順序/dedup、全rule code/message parity、parser/type/lint全体のdiagnostic parity、component/packaged
release parity、Rust-free aggregateは未完了である。V2-16b / V2-16c / V2-16eは[~]を維持する。Evidence commit: `a0a3dd55`。

### V2-16b / V2-16c native standard LS1004 argument-mismatch Diagnostic span projection (2026-08-10)

REDでは、`(defn bad [] (+ 1 true))` のLS1004がnative App.Cliからpoint range `0:0..0:0`で返る差分を固定し、期待する
exact range `0:13..0:23`をRust/native共通のdidOpen wire contractへ追加した。既存Mac artifact runnerは actual `0:0..0:0`で失敗し、
Rust focused E2Eは `0 passed / 1 failed / 376.43s`となった。App.Cli投影だけではspanが存在しないため `0 passed / 1 failed / 391.13s`となり、
failure boundaryをselfhost parser/TypeInferApplyへ絞った。

`f937afd3`では、apply AST末尾へ開き括弧から閉じ括弧までのoffsetを保持し、通常macro展開で引き継ぎ、LS1004失敗結果だけへspanを載せた。
既存のapply argc/argument index契約は変更していない。同じRust focused E2Eは `1 passed / 439.89s`となった。

Mac Apple Siliconのcurrent-source App.Cli release gateは `1 passed / 1224.34s`。artifact
`ci-artifacts/native-release/aarch64-apple-darwin/current-f937afd3-lsp-argument/manifest.json`は target `aarch64-apple-darwin`、source commit
`f937afd34a0cb893e5db0177e698668afae8f437`、`selfhost_fixed_point=true`、program `4,393,216` bytes、program SHA-256
`243c5cbf0804516ff0d9219c2b12dd22675608207a92b4dc88cc133a45aff8fa`、stderr 0を記録した。同じMac programのnative core runtime matrixはLS1004を含む
`41 cases`全passだった。

Linux x86_64のcurrent-source actual self-regeneration summaryは
`ci-artifacts/native-linux-x86-hostgen-vm/f937afd3-lsp-argument/actual-selfregen-summary.json`にtarget `x86_64-unknown-linux-gnu`、host
`Linux/x86_64`、status `pass`、stage2/stage3 code length各`11,466,916`、stdout SHA-256各
`a6d4029c9af9ff73f504183932bf7e80679fba9f87bdcb1b119f2f634b8ebb68`、両stderr 0を記録した。stage2/stage3 manifestは source commit
`f937afd34a0cb893e5db0177e698668afae8f437`、data `2,757` bytes、entrypoint `11,462,308`、function-start length `3,430`、main function index
`3,439`で一致した。verified stage2をVM-side lock付きで再利用した target-only App.Cli artifactの
`ci-artifacts/native-linux-x86-hostgen-vm/f937afd3-lsp-argument-cli/manifest.json`は source tree SHA-256
`6c3a6a48c9a9acc0ab04793ca4166ebf4e4e6c0fa63ea56abd9a5ab4e5ebf712`、`selfhost_fixed_point=true`、code `13,404,007` bytes、program
`13,446,952` bytes、program SHA-256 `ef0daffb943910105b8882aede445f97ca622b7a22db6a76fe6321b06e161244`、`--version` smoke
`lsharp 0.1.0`、stderr 0を記録した。同じLinux ELFをVM内で実行したnative core runtime matrixは`41 cases`全passだった。VM free-space gateは
current selfregen available `7,669,870,592` / required `4,294,967,296` bytes、target-only available `7,669,694,464` bytes。target-only後にguest
runner/workdirを回収し、`lsharp-linux-x86`を停止した。

これはLS1004 function argument mismatchのexact range projectionと、同一current-source App.Cli artifactの両対応target runtimeに限定したverified partialである。
LS1004以外の正確なspan、複数diagnosticの順序/dedup、全rule code/message parity、parser/type/lint全体のdiagnostic parity、component/packaged
release parity、Rust-free aggregateは未完了である。V2-16b / V2-16c / V2-16eは[~]を維持する。Evidence commit: `f937afd3`。

### V2-16b / V2-16c native standard LS1003 infinite-type Diagnostic span projection (2026-08-11)

REDでは、既存fixture `(defn main [x] (x x))` のLS1003がnative App.Cliからpoint range `0:0..0:0`で返る差分を固定し、期待する
exact range `0:15..0:20`をRust/native共通のdidOpen wire contractへ追加した。保存済みMac artifact runnerは actual `0:0..0:0`で失敗し、
Rust focused E2Eは `0 passed / 1 failed / 388.57s`となった。`TypeInferApply` の apply error helperへLS1003を追加した段階でも
`0 passed / 1 failed / 458.52s`となり、failure boundaryをselfhost `App.Cli` の span projectionへ絞った。

`bf9868de`では、既存のapply spanをLS1003のfailure resultへ保持し、`App.Cli`の標準 type Diagnostic projectionでLS1003もspanを使用するようにした。
LS1001、LS1002、LS1004の既存 code/message/range契約は変更していない。同じRust focused E2Eは `1 passed / 460.13s`となった。

Mac Apple Siliconのcurrent-source App.Cli release gateは `1 passed / 1028.01s`。artifact
`ci-artifacts/native-release/aarch64-apple-darwin/current-bf9868de-lsp-infinite/manifest.json`は target `aarch64-apple-darwin`、source commit
`bf9868de98649634714c939964be6e283e4f357b`、`selfhost_fixed_point=true`、program `4,393,216` bytes、program SHA-256
`88f662b1d1e11add41101ea46f55a9c892e7762e47ace9c44327f249d3c135be`、stderr 0を記録した。同じMac programのnative core runtime matrixはLS1003を含む
`41 cases`全passだった。

Linux x86_64のcurrent-source actual self-regeneration summaryは
`ci-artifacts/native-linux-x86-hostgen-vm/bf9868de-lsp-infinite/actual-selfregen-summary.json`にtarget `x86_64-unknown-linux-gnu`、host
`Linux/x86_64`、status `pass`、stage2/stage3 code length各`11,466,916`、stdout SHA-256各
`a6d4029c9af9ff73f504183932bf7e80679fba9f87bdcb1b119f2f634b8ebb68`、両stderr 0を記録した。stage2/stage3 manifestは source commit
`bf9868de98649634714c939964be6e283e4f357b`、data `2,757` bytes、entrypoint `11,462,308`、function-start length `3,430`、main function index
`3,439`で一致した。verified stage2をVM-side lock付きで再利用した target-only App.Cli artifactの
`ci-artifacts/native-linux-x86-hostgen-vm/bf9868de-lsp-infinite-cli/manifest.json`は source tree SHA-256
`823dde15ddaccbbd0d442275b6cfc7771acfd429e62e0b08cb38029886330ead`、`selfhost_fixed_point=true`、code `13,404,192` bytes、program
`13,446,952` bytes、program SHA-256 `7f8570d17a4d8b44a6b3399d839479be9bb6d375ba2c8e63ff9e36b0fa799e1c`、`--version` smoke
`lsharp 0.1.0`、stderr 0を記録した。同じLinux ELFをVM内で実行したnative core runtime matrixは`41 cases`全passだった。VM free-space gateは
current selfregen available `7,669,288,960` / required `4,294,967,296` bytes、target-only available `7,669,272,576` bytes。target-only後にguest
runner/workdirを回収し、`lsharp-linux-x86`を停止した。

これはLS1003 parameterized self-applicationのexact range projectionと、同一current-source App.Cli artifactの両対応target runtimeに限定したverified partialである。
LS1002 if-conditionの正確なspan、複数diagnosticの順序/dedup、全rule code/message parity、parser/type/lint全体のdiagnostic parity、component/packaged
release parity、Rust-free aggregateは未完了である。V2-16b / V2-16c / V2-16eは[~]を維持する。Evidence commit: `bf9868de`。

### V2-16b / V2-16c native standard LS1002 if-condition Diagnostic span projection (2026-08-11)

REDでは、既存fixture `(defn main [] (if 1 true false))` のLS1002 if-conditionがnative App.Cliからpoint range `0:0..0:0`で返る差分を固定し、
Rust oracleが返すif式全体のexact range `0:14..0:31`をRust/native共通のdidOpen wire contractへ追加した。保存済みMac artifact runnerは actual
`0:0..0:0`で失敗し、Rust focused E2Eは `0 passed / 1 failed / 359.36s`となった。

`86a9b1fe`では、selfhost `TypeInfer.ls` のif-condition failure resultへ既存のif式 `if-start/if-end` spanを保持し、`App.Cli`の標準 type
Diagnostic projectionで`error-code-if-cond`もspanを使用するようにした。LS1001、LS1002 if-branch、LS1003、LS1004の既存
code/message/range契約は変更していない。同じRust focused E2Eは `1 passed / 359.18s`となった。実装・fixture・native runnerのcommitは
`86a9b1fe`で、`main`へpush済みである。

Mac Apple Siliconのcurrent-source App.Cli release gateは `1 passed / 1035.64s`。artifact
`ci-artifacts/native-release/aarch64-apple-darwin/current-86a9b1fe-lsp-if/manifest.json`は target `aarch64-apple-darwin`、source commit
`86a9b1fe3f6e341f5cfe70eec7f3aef8694e1b0c`、`selfhost_fixed_point=true`、program `4,393,216` bytes、program SHA-256
`bc84916ac0e6dbad8c607b349c70d342994a93b2911650747a9b42080c1562df`、stderr 0を記録した。同じMac programのnative core runtime matrixは
LS1002 if-conditionを含む `41 cases`全passだった。

Linux x86_64のcurrent-source actual self-regeneration summaryは
`ci-artifacts/native-linux-x86-hostgen-vm/86a9b1fe-lsp-if/actual-selfregen-summary.json`にtarget `x86_64-unknown-linux-gnu`、host
`Linux/x86_64`、status `pass`、stage2/stage3 code length各`11,466,916`、stdout SHA-256各
`a6d4029c9af9ff73f504183932bf7e80679fba9f87bdcb1b119f2f634b8ebb68`、両stderr 0を記録した。stage2/stage3 manifestは source commit
`86a9b1fe3f6e341f5cfe70eec7f3aef8694e1b0c`、data `2,757` bytes、entrypoint `11,462,308`、function-start length `3,430`、main function index
`3,439`で一致した。Rust/native stage1 gateは `1 passed / 426.38s`、VM free-space gateは current selfregen available
`7,668,736,000` / required `4,294,967,296` bytesだった。

verified stage2をVM-side lock付きで再利用したtarget-only App.Cli artifactの
`ci-artifacts/native-linux-x86-hostgen-vm/86a9b1fe-lsp-if-cli/manifest.json`は target `x86_64-unknown-linux-gnu`、source tree SHA-256
`37236748940400c811636f07be22786e02eb1e77bbfb0ab69356fcc9aa7e108f`、`selfhost_fixed_point=true`、code `13,404,669` bytes、program
`13,446,952` bytes、program SHA-256 `8beb7c0a4f4e9694f64e3002ec6a488376e24e03e434f4c17f9fecfe1ee1e9bc`、`--version` smoke
`lsharp 0.1.0`、stderr 0を記録した。同じLinux ELFをVM内で実行したnative core runtime matrixは `41 cases`全passだった。target-only free-space
gateは available `7,668,666,368` / required `4,294,967,296` bytesだった。target-only後にguest runner/workdirを回収し、`lsharp-linux-x86`を停止した。

これはLS1002 if-conditionのexact range projectionと、同一current-source App.Cli artifactの両対応target runtimeに限定したverified partialである。
recursive aliasなど別のtype diagnostic、複数diagnosticの順序/dedup、全rule code/message parity、parser/type/lint全体のdiagnostic parity、component/packaged
release parity、Rust-free aggregateは未完了である。V2-16b / V2-16c / V2-16eは[~]を維持する。Evidence commit: `86a9b1fe`。

- [~] `V2-16b` native built-in type environment retention — `0459ad98` の current-source Mac Apple Silicon
  stage0から生成した native `App.Cli`で、numeric/string/container/reference、`file-exists?`、`int-to-string`、
  `map-contains?` を含む valid builtin call matrixと、builtin argument mismatch matrix、`+` の valid/invalid、
  標準 LSP Diagnostic wire objectを確認した。parserが生成する負の builtin name hashが native signed 64-bit
  即値の誤った下位/上位32-bit分解で別値になる failure boundaryを特定し、x86 legacy/append と AArch64 chunkの
  signed floor division・unsigned normalizationを追加した。Mac current-source actual release gate、native matrix
  `5 tests`、Linux x86_64 stage1/stage2/stage3 fixed point、runtime smoke、stderr空、stage2/stage3 code length・
  stdout hash一致を確認済みである。`infer-apply-legacy-raw` の1引数分岐で未使用の外側束縛が native local slotを衝突させていた
  既存原因も `50a2ad3c` で修正済みである。built-in全体、全型診断、全公開command、component/packaged release parityは
  未完了のため、この項目は `[~]` のまま残す。さらに `0459ad98` で比較（`>`, `<`, `<=`, `>=`, `==`, `!=`）と
  論理（`not`, `and`, `or`）の valid/invalid native type contractを追加し、Mac current-source native matrixを
  `5 tests` 全 passで確認した。同じ source commitの Linux x86_64 current-source gateは
  `NATIVE_LINUX_X86_HOSTGEN_VM_ARTIFACT_ID=0459ad98-comparison-logic bash scripts/ci/native-linux-x86-selfregen.sh`
  を一度だけ実行し、`ci-artifacts/native-linux-x86-hostgen-vm/0459ad98-comparison-logic/actual-selfregen-summary.json` で
  target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length各 `11,412,074`、
  stdout SHA-256各 `d5c7adef8f4b164ef216205e5025fb571e63015aabdeff471377c20924f61e89`、stderr空、runtime smoke exit `42`を
  確認した。これは比較/論理の type contract と current-source Mac/Linux fixed pointに限定した verified partialであり、
  built-in全体、全型診断、全公開command、component/packaged release parityは未完了である。
  さらに `496a8ff3` で `write-file-bytes` の `String -> Vector -> Int` valid callと first-argument mismatchを Rust oracleと
  native CLI fixtureへ追加した。Rust focused oracleは `1 passed`、同 commitの Mac current-source actual App.Cli release gateは
  `1 passed` / `821.97s`、native matrixは `5 tests` 全 passとなった。`0459ad98` から `selfhost/src` に差分がないことを確認したため、
  Linux x86_64の重い stage2/stage3 replayは重複実行していない。manifestの source commitが一致しない旧Linux artifactを current evidenceへ
  拡大解釈せず、この fixtureの Linux current-source direct gateは未検証として残す。

  さらに `1996fe6e` / `caba3ad6` / `a5317c95` で standalone Preview1 の `proc_exit` を native Wasm emitterへ接続した。
  `wasi_snapshot_preview1.proc_exit` の import/type追加に伴う defined-function index、user-call、wrapper exportの shiftを揃え、
  `i32.wrap_i64` と trailing `i64.const 0`で selfhost user functionの `() -> i64` result contractを維持した。Rust focused user-call
  regressionは `1 passed` / `316.38s`、既存の raw-byte partial-write regressionは `1 passed` / `315.96s`、`a5317c95` の
  Mac Apple Silicon current-source actual release gateは `1 passed` / `830.50s` で、native I/O runtime matrixは
  `print-string`、`write-file`、`write-file-bytes`、`proc-exit`（exit `7`）の4 cases全 passとなった。成功経路では Rust
  fallbackを使用していない。Linux x86_64の同 current-source stage2/stage3 replayと直接 I/O runtime matrixは未検証として残し、
  standalone read-stdin、全 fd error semantics、全公開command、component/packaged release parityも未完了である。
  その後、clean `HEAD=5163a1d67ad2883ceaea08ea4310e99c201acfe3` の Linux x86_64 current-source gateを一度だけ実行した。
  `actual-selfregen-summary.json` は target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length各
  `11,420,772`、stdout SHA-256各 `86dfc089c2d56311a6e778b393fe4848f784e6c559196ce5a8d2ca659c9fd7b5`、runtime smoke exit `42`、
  stderr空を記録し、stage1/stage2/stage3 manifestの target/source commitも一致した。さらに同じ actual stage1から作成した
  Linux stage0 packageを `native-selfhost-dev.sh` の入口へ渡し、VM内の Linux x86_64 native `Cli`で I/O runtime matrixを実行して
  `print-string`、`write-file`、`write-file-bytes`、`proc-exit`（exit `7`）の4 cases全 passを確認した。raw stage1 payloadを
  public `Cli`として直接起動すると no-opになるため、stage0 package化と `native-selfhost-dev.sh`を必須の再現経路として記録する。
  Linux direct standalone I/Oのこの4ケースは verified partialになったが、read-stdinのstandalone Wasm runtime、全 fd error semantics、
  argv/full command-line、4096 bytes超の動的 layout、全公開command、component/packaged release parityは未完了である。

  さらに `2d96d09e5160dce7a0707e4b690d68260fc28571` の current-source verified partialとして、standalone Preview1 `read-stdin` を
  IR opcode `91`、WASI `fd_read` の 4096-byte bounded chunk loop、tagged String result、defined-function/wrapper export index shiftまで
  native Wasm emitterへ接続した。Rust E2E `test_e2e_selfhost_standalone_read_stdin_runtime` は `1 passed` / `321.86s`、保存した
  Wasmは `wasm-tools validate` と Wasmtime 43.0.0 の stdin `payload` -> stdout `payload` / exit `0`を通過した。
  Mac Apple Silicon current-source actual `App.Cli` gateは `1 passed` / `816.52s`、native I/O matrixは read-stdinを含む5 cases全 passで、
  manifestの targetは `aarch64-apple-darwin`、program SHA-256は `2abf3480c7f237a5271cf14bef63a212c790ead7c0ebcb852b9854ad253eecff` だった。
  Linux x86_64 current-source stage1 -> stage2 -> stage3 fixed pointも同じ source commitで status `pass`、stage2/stage3 code length各
  `11,441,967`、stdout SHA-256各 `848677d2fcce2bbd47fe405ada0ba766fcfffced6899e6308bdc6938856e526f`、stderr各 `0`、VM free-space gate
  `7,676,780,544` / `4,294,967,296` bytesで passした。stage2 artifactを再利用した Linux App.Cli target-only materializeは
  target `x86_64-unknown-linux-gnu`、code `13,367,530` bytes、program SHA-256
  `c4755b25f58d12cfc863ede0da18db79e24051bca13aed3bef74996766b9cb3e`、selfhost_fixed_point `true`を記録し、同VMの native I/O
  matrixも read-stdinを含む5 cases全 passだった。native成功経路では Rust fallback、`cargo`、`rustc`、host `lsharp`を呼び出していない。
  これは standalone read-stdinの両対応target runtime/artifact verified partialであり、fd error/EOFの全 semantics、4096 bytes超の動的
  layout、argv/full command-line、全公開command、component sidecar、release asset acquisition/rollback、Mac/Linux packaged parityは
  引き続き未完了である。`V2-16b` / `LEGACY-IO-01` は `[~]` のまま維持する。

  次の `e553df73e66971f763740587283f72414fdd1b2e` batchでは、`fd_read` が bytesとnon-zero errnoを同時に返したとき旧 standalone
  emitterが stdoutを空にする REDを `test_e2e_selfhost_standalone_read_stdin_runtime` に固定した。Wasm emitterはerrno resultをdropし、
  `nread` scratchを直接 String concatへ渡す最小修正に留めた。Rust focused E2Eは `1 passed` / `316.15s` で、empty stdin、4096 bytes、
  4097 bytes、partial bytes + errnoを含む全ケースを通過した。Mac Apple Silicon current-source App.Cli manifestは target
  `aarch64-apple-darwin`、`selfhost_fixed_point=true`、program SHA-256 `48720214af0e7c2f8ca3ab76cc46067f6f2d369a29de280b35b63f9043daf69a`、
  native I/O matrixは8 cases全 passとなった。Linux x86_64 current-source stage1 -> stage2 -> stage3 fixed pointは status `pass`、
  stage2/stage3 code length各 `11,440,809`、stdout SHA-256各 `c263840ae70301622e3e8d41ca911e3fa536a7ecd5ac87adb809196426169f38`、
  stderr各 `0`、free-space gate `7,679,078,400` / `4,294,967,296` bytesで passした。stage2再利用の Linux App.Cli materializeは
  target `x86_64-unknown-linux-gnu`、code `13,366,372` bytes、program SHA-256
  `e0db7dcddd9f82d32daffc20ad8dea049aa834bd0b14df3158606b610bf5f894`、`selfhost_fixed_point=true`、stderr `0`を記録し、同VMの
  native I/O matrixも8 cases全 passだった。これは nread-bearing fd_read errnoとempty/4096/4097 stdinの両対応target verified partialであり、
  fd error/EOFの全 semantics、argv/full command-line、全公開command、component sidecar、release asset acquisition/rollback、
  Mac/Linux packaged parityは未完了である。`V2-16b` / `LEGACY-IO-01` は `[~]` のまま維持する。

  さらに `33d5483f` で `substring` の3引数 applyを type-inference REDへ追加し、valid `String -> Int -> Int -> String` と途中の
  `Bool` 引数 mismatchを同じ selfhost harnessで比較した。Rust oracleの
  `cargo test -p lsharp-wasm --test e2e e2e::selfhost_typeinfer_builtin_parity:: -- --nocapture` は6 tests全 passし、native builtin
  mismatch fixtureにも `(substring "abc" true 1)` を追加した。保存済み Mac artifact `d2dcea7e` への replay-only sanityは5 tests全 pass
  だったが、artifactの source commitが現 checkoutと一致しないため current-source native evidenceには数えない。current-source Mac/Linux
  native check、全 builtin、全型診断、全公開command、component/packaged release parityは未完了であり、`V2-16b` は `[~]` のまま維持する。

  続く `ee08f23b` current-source gateでは、`bash scripts/ci/native-macos-aarch64-stage0-release.sh` を一度だけ実行し、Mac Apple
  Siliconの App.Cli stage chainを `1 passed` / `832.82s` で完了した。stage0 manifestは target `aarch64-apple-darwin`、source commit
  `ee08f23b132d6146716c5c025cf9d543cfc4b88a` と一致し、packageは
  `ci-artifacts/native-stage0/aarch64-apple-darwin/ee08f23b-substring-type-contract/` に保存した。同packageを
  `scripts/native-selfhost-dev.sh` の入口へ渡して current-source App.Cliをmaterializeし、
  `scripts/ci/test-native-selfhost-type-builtins.py` は `5 tests` 全 pass（`substring` mismatchを含む）だった。
  同じ source commitの Linux x86_64 gateも `NATIVE_LINUX_X86_HOSTGEN_VM_ARTIFACT_ID=ee08f23b-substring-type-contract bash
  scripts/ci/native-linux-x86-selfregen.sh` を一度だけ実行し、
  `ci-artifacts/native-linux-x86-hostgen-vm/ee08f23b-substring-type-contract/actual-selfregen-summary.json` で target
  `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length各 `11,442,429`、stdout SHA-256各
  `2526caaefa9e86b934d5d08eb800847ac96e6b3989f3c3c37c7d2c933516086e` を確認した。VM free-space gateは
  `7,683,088,384` / `4,294,967,296` bytesで passし、成功後にVM workdirとreplay lockを回収してVMを停止した。
  これは current-source Mac/Linux stage provenanceとstage2/stage3 fixed point、およびMac native substring type fixtureの verified
  partialであり、Linux側の全 builtin matrix、全型診断、全公開command、component/packaged release parityは未完了である。

  さらに command-line runtimeの同一 fixtureを、区切り文字付き出力で空要素の位置まで観測できる形へ拡張した。
  `prog name`、空要素、UTF-8の `雪 空`、空白を含む `tail value` を渡す special-argv caseは Rust standalone E2Eと
  native I/O matrixで `prog name||雪 空|4` を確認し、通常の `alpha` / `beta` と strict argc=0も同じ matrixへ保持した。
  Rust focused E2Eは `1 passed` / `336.08s`、Mac Apple Siliconの保存済み current-source App.Cli artifactは `16 cases` 全 pass、
  Linux x86_64の保存済み target-only artifact（`eb8086a8`、replay-only）は `16 cases` 全 passだった。Linux artifactは current-source
  provenance gateではないため、同 targetの current-source native regeneration/runtime evidenceは残る。この batchは argvの
  空要素・UTF-8・空白保持を閉じる verified partialであり、fd error/EOFの全 semantics、dynamic layout、全公開command、component/
  packaged release parity、Rust-free aggregateは未完了である。ADR: [`decisions-v0.3-native-standalone-command-line-argv-boundary.md`](docs/adr/decisions-v0.3-native-standalone-command-line-argv-boundary.md)。

  続く `19b01384` では、262144-byte stdinで旧 standalone bump allocatorが linear memory `0x1000000`（16 MiB）へ trapする
  REDを固定した。allocatorの heap end が現在の `memory.size << 16`を超える場合に必要ページだけ `memory.grow`してから bump
  pointerを保存する narrow fixを追加した。Rust focused E2Eは `1 passed` / `317.18s`、Mac Apple Silicon current-source
  stage0は source commit `19b01384281a8efdcc9f0b9ecddb4faeed36b113` と一致し `1 passed` / `827.33s`、native I/O matrixは
  256KiB stdinを含む17 cases全 passだった。同じ source commitの Linux x86_64 current-source stage1 -> stage2 -> stage3
  fixed pointは target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length各
  `11,445,101`、stdout SHA-256各 `4ddaa27ed209bf8fce4305ea459a10ed99d308db7c1818222f5cfae38dbf44bc`、stderr空となった。
  stage2 artifactを再利用した Linux App.Cli target-only materializeは `selfhost_fixed_point=true`、code `13,370,664` bytes、
  program SHA-256 `25a3dd5c9ca786ac54c7f88ba1be7cccbf77589cee9cb65bf477817167af961d`、`--version`の
  `lsharp 0.1.0`、native I/O matrix17 cases全 passを記録し、stage0 package manifestも同 target/source commitを保持する。
  これは standalone allocator growthと256KiB stdinの両対応target verified partialであり、fd error/EOFの全 semantics、より大きい
  dynamic root/data/heap layout、GC/容量失敗診断、全公開command、component sidecar、release asset acquisition/rollback、
  Mac/Linux packaged provenance parity、Rust-free aggregateは未完了である。`V2-16b` / `LEGACY-IO-01` は `[~]` のまま維持する。
  ADR: [`decisions-v0.3-native-standalone-allocator-memory-growth.md`](docs/adr/decisions-v0.3-native-standalone-allocator-memory-growth.md)。

  続く `8ab2dd58` では、`(4 * 1024 * 1024) - 1` bytes の stdin payloadで、chunkごとの String再確保を繰り返す旧経路を
  REDとして固定した。standalone `read-stdin` は初期容量4096 bytesの単一 String bufferを使い、必要時に容量を倍増して
  新しい String objectへ bytesを `memory.copy` する narrow fixを追加した。fd_readのscratch/iovec、String length offset、
  standalone WASI call indexも実際のWasm bytesとruntimeで確認した。

  Evidence:

  - Rust focused E2E `selfhost_standalone_io::test_e2e_selfhost_standalone_read_stdin_runtime` — 1 passed / 315.43s。
  - Mac Apple Silicon current-source stage0は source commit `8ab2dd589410fa668ffa5c01f596bdfa046d466c` と一致し、1 passed / 825.61s、
    materialized native I/O matrixは18 cases全 pass。
  - Linux x86_64 current-source stage1 -> stage2 -> stage3 fixed pointは
    `ci-artifacts/native-linux-x86-hostgen-vm/8ab2dd58-standalone-stdin-over-4m/actual-selfregen-summary.json` で
    `status=pass`、stage2/stage3 code length各 `11,449,265`、stdout SHA-256各
    `179821d0fddaceaac637b08a128beee7c31c4afdc4bd4a90e88b013755855f3d` を確認した。
  - 同じ source commitの Linux x86_64 App.Cli target-only materializeは `selfhost_fixed_point=true`、code `13,374,828` bytes、
    program SHA-256 `f21ebd22261a2dd392e5123293faac948dcf40c586069966cfcab46545960cc4`、stderr 0 bytesを記録し、Linux native I/O matrixも
    18 cases全 passした。これは target-only materializeを含む current-source runtime evidenceであり、全公開commandの完了証拠ではない。
  - Static read-stdin contract、Wasm validation、Python `py_compile`、`git diff --check`を通過し、VM workdir/replay lock、Wasmtime tarball、
    matrix workdirを回収してVMを停止した。

  これは standalone stdin bufferの容量倍増と4MiB超 payloadの両対応target verified partialである。fd error/EOFの全 semantics、
  より大きい/同時実行のdynamic root/data/heap layout、GC/容量失敗診断、全公開command、component sidecar、release asset
  acquisition/rollback、Mac/Linux packaged provenance parity、Rust-free aggregateは未完了であり、`V2-16b` / `LEGACY-IO-01` は
  `[~]` のまま維持する。ADR: [`decisions-v0.3-native-standalone-stdin-capacity-growth.md`](docs/adr/decisions-v0.3-native-standalone-stdin-capacity-growth.md)。

  続く `ad65eaffdd7b928ec5e2d226c6f4695236afd05c` batchでは、正確に4MiBの file を旧 standalone `read-file` へ渡す REDを
  `test_e2e_selfhost_standalone_read_file_returns_all_bytes_over_4m` と native I/O matrixへ追加した。旧経路は4096-byte
  readごとに `string-concat` を繰り返し、5分超実行しても assertionへ到達しなかったため、bounded chunkの failure boundaryを
  固定した。Wasm emitterは既存の open/read/close と fail-closed semanticsを保ったまま、file objectとiovecを4MiBへ拡張し、
  4MiB単位で一度に読み込んでStringを生成する narrow fixを追加した。任意サイズのdynamic file bufferや全fd error/EOF semanticsは
  この変更では閉じていない。

  Evidence:

  - Rust focused E2E `selfhost_standalone_io::test_e2e_selfhost_standalone_read_file_returns_all_bytes_over_4m` — 1 passed / 316.52s。
  - Mac Apple Silicon current-source stage0は source commit `ad65eaffdd7b928ec5e2d226c6f4695236afd05c` と一致し、1 passed / 828.19s、
    materialized native I/O matrixは19 cases全 pass。
  - Linux x86_64 current-source stage1 -> stage2 -> stage3 fixed pointは
    `ci-artifacts/native-linux-x86-hostgen-vm/ad65eaff-standalone-file-over-4m/actual-selfregen-summary.json` で
    target `x86_64-unknown-linux-gnu`、host `Linux/x86_64`、`status=pass`、stage2/stage3 code length各 `11,448,943`、stdout SHA-256各
    `a66bf8c746a9cf91a6b0cdb0509a9f12b3b7987301f025646d69fdffd1c6677e`、stderr空を確認した。stage1 manifestの source commitも同じ
    `ad65eaff...` と一致した。
  - 同じ stage2 artifactを再利用した Linux x86_64 `App.Cli` target-only materializeは
    `ci-artifacts/native-linux-x86-hostgen-vm/ad65eaff-standalone-file-over-4m-linux-cli/actual-selfregen-summary.json` で
    `selfhost_fixed_point=true`、code `13,374,506` bytes、program SHA-256
    `a090cd8474c6115ac3a2bcf5570226cc912d7479d0285f6991ca02fb5a1d6469`、stderr `0` bytes、`--version` `lsharp 0.1.0`を記録した。
    同じ Linux target-only native programに公式 Wasmtime 43.0.0を渡した I/O matrixは19 cases全 passだった。
  - Static read-stdin contract、Python `py_compile`、`git diff --check`を通過した。Linux VM workdir、replay lock、matrix workdir、
    Wasmtime tarballを回収し、`lsharp-linux-x86` は停止済みである。

  これは standalone `read-file` の4MiB bounded chunkと両対応target runtimeの verified partialである。任意サイズのdynamic file
  buffer、fd_read/fd_close/path_openの全 error/EOF組合せ、同時実行時のdynamic root/data/heap layout、GC/容量失敗診断、全公開command、
  component sidecar、release asset acquisition/rollback、Mac/Linux packaged provenance parity、Rust-free aggregateは未完了であり、
  `V2-16b` / `LEGACY-IO-01` は `[~]` のまま維持する。ADR: [`decisions-v0.3-native-standalone-read-file-4m-chunk.md`](docs/adr/decisions-v0.3-native-standalone-read-file-4m-chunk.md)。

  さらに `e962b6ef` / `13185038` で、builtin type environmentの source-level fixtureを拡張した。Rust oracleは Floatの
  valid/mismatch と、同じ定義内で `print` を `Int` と `String` に独立 instantiateする契約を通過し、Linux x86_64は
  `ad65eaff` の既存 App.Cli artifactへ更新後の matrixを replayして `5 tests` 全 passとなった。production selfhost sourceは
  `ad65eaff` から変わっていないため重い Mac/Linux current-source stage replayは重複実行していない。この Linux結果は test-only fixtureの
  replay evidenceであり、新しい source provenance gateとは数えない。さらに `3d163344` で vector/map の要素・キー・値型を変えた
  repeated collection useを同じ fixtureへ追加し、Rust oracleと同じLinux replay matrixを通過した。続く `56f78589` では Floatの4演算子
  全て（`+.`, `-.`, `*.`, `/.`）の valid/mismatch source contractを同じ matrixへ拡張した。builtin全体、全型診断、全公開command、component/packaged
  release parity、Rust-free aggregateは未完了であり、`V2-16b` は `[~]` のまま維持する。
  続く `cf9ac17f` では、`map-remove` の Bool receiver mismatch `(map-remove true 1)` を Rust oracle と native type-builtin matrixの
  同一 fixtureへ追加した。Rust focused test `1 passed`、builtin parity module `10 passed`、current Mac App.Cli artifactへの native
  matrix `5 tests` 全 passを確認した。REDで試した key 型 mismatch は、canonical `Map` が key/value 型を保持しない消去型であること、
  さらに既存の `Int` と heap handle の runtime representation compatibility があることから仕様契約にならず採用しなかった。
  selfhost implementationの変更はなく、test-only replayのため重い stage1→stage3 regenerationは重複実行していない。この verified
  partialは receiver boundary に限られ、Mapの key/value semantic typing、全 builtin family、全型診断、全公開command、component/packaged
  release parity、Rust-free aggregateは未完了であり、`V2-16b` は `[~]` のまま維持する。Evidence commit: `cf9ac17f`。
  続く `c97c01c0` では、`ref-new` と `ref-set` の source-level let binding contractを追加した。`(ref-new "x")` を束縛して
  `(ref-set r "y")` は成功し、`(ref-set r 1)` は `function argument type mismatch` で拒否する同一 fixtureを Rust oracle と
  native type-builtin matrixへ追加した。Rust focused test `1 passed`、builtin parity module `11 passed`、current Mac App.Cli artifact
  の native matrix `5 tests` 全 passを確認した。既存の `Ref a -> a -> Unit` schemeとselfhost implementationは変更しておらず、
  test-only replayのため stage1→stage3 regenerationは重複実行していない。これは ref source-level apply境界の verified partialであり、
  全 builtin family、全型診断、全公開command、component/packaged release parity、Rust-free aggregateは未完了のまま、`V2-16b` は
  `[~]` を維持する。Evidence commit: `c97c01c0`。

  続く `8cc58a4a` では、標準 LSP wire の `initialize -> didOpen(valid) -> didChange(invalid) -> publishDiagnostics` sequenceを
  Rust oracle と native CLI runtime matrixへ追加した。`file:///tmp/lsharp-lsp-didchange.ls` のURIを保持し、invalid sourceの
  `LS1004`、severity `1`、source `lsharp`、message `function argument type mismatch`、zero-based rangeを固定した。
  Rust focused testは `1 passed`、Mac Apple Siliconの current-source `a0845320` App.Cli artifactを再利用した native core CLI matrixは
  `24 cases` 全 pass、exit `0`、stderr空だった。selfhost production sourceは変わっていないため、stage1 -> stage3 regenerationは
  重複実行していない。これは didChange後のtype diagnostics refreshとwire URI/標準 fieldsの verified partialであり、Linux
  current-source App.Cliでの同じ direct LSP sequence、full diagnostics/type/lint parity、definition/references/renameの全 semantic
  projection、component/release parity、Rust-free aggregateは未完了のため、`V2-16b` / `V2-16c` / `V2-16e` は `[~]` のまま維持する。
  Evidence commit: `8cc58a4a`。

  続く `1512cac9` では、同じ URIで `didChange(invalid) -> publishDiagnostics(LS1004) -> didChange(valid) -> publishDiagnostics([])` を
  追加し、stale type diagnosticsをvalid sourceで消去する順序を固定した。Rust focused testは `1 passed` / `334.00s`、Mac Apple Siliconの
  current-source `a0845320` App.Cli artifactは native core CLI matrix `25 cases` 全 pass、Linux x86_64の保存済み `cbbafe94` App.Cli
  ELF replayもVM内で `25 cases` 全 pass、いずれもexit `0` / stderr空だった。`cbbafe94` から現HEADまで `selfhost/src` の差分はなく、
  stage1 -> stage3 regenerationは重複実行していない。ただしLinux artifactのmanifest source commitは現HEADと異なるためLinux結果は
  replay-only evidenceであり、current-source provenance gateには数えない。これは stale diagnostics clearの verified partialであり、
  full diagnostics/type/lint parity、definition/references/renameの全 semantic projection、component/release parity、Rust-free aggregateは
  未完了のため、`V2-16b` / `V2-16c` / `V2-16e` は `[~]` のまま維持する。Evidence commit: `1512cac9`。

  続く `16871a7a` では、文字列（`string-concat`、`string-eq`、`string-char-at`、`substring`）、Vector（`vector-new`、`vector-set`、
  `vector-push`）、Map（`map-get`、`map-contains?`）、Ref（`ref-get`）の10個の引数位置 mismatchを、Bool/Intの明確な不一致として
  Rust oracleとnative type-builtin matrixへ追加した。Rust focused testは `1 passed`、Mac Apple Silicon current-source App.Cli artifactの
  native suiteは `5 tests` 全 pass、Linux x86_64の保存済み `cbbafe94` App.Cli artifact replayも `5 tests` 全 passした。成功経路の
  stderrは空で、selfhost production sourceは変更していないためstage1 -> stage3 regenerationは重複実行していない。初期REDで試した
  Vector/Map/Ref receiverへのInt mismatchは、Rust oracleの既存 `Int`/heap handle compatibilityにより受理されるため、Map key mismatchと同じく
  新しい仕様境界として採用しなかった。これは builtin argument diagnosticsのverified partialであり、全builtin family、全型診断、runtime/codegen parity、
  全公開command、component/release parity、Rust-free aggregateは未完了のため、`V2-16b` / `V2-16c` / `V2-16e` は `[~]` のまま維持する。
  Evidence commit: `16871a7a`。

- [~] `I-09` / `M3-05-N9` / `EC-M3-05` nested package source ownership — regular な package 内の `src/` directory symlink を
  外部 source として辿らず、source traversal / in-memory package API generation の既存 not-found / empty / ignore 契約を
  Rust/native の同一 fixture で閉じる。root `src/` directory symlinkについては、実装前の RED、外部 source の
  name/content/metadata 非投影、native program no-execution、Rust/native parityまで検証済みの verified partial とする。
  さらに `ce8a4cb7` で `src/` 配下の child symlinkを、Rust API-doc collector、Rust MCP package API、Rust module-index collector、
  native MCP shimの同一 external `Geometry.ls` fixtureで拒否する RED→GREENを追加した。Rust側は `symlink_metadata` で各 entryを検査し、
  外部 sourceを API/module-indexへ投影せず、native側は既存の no-execution/fail-closedを再確認した。
  続く `f76dafb9` では `src/Geometry/Vec2.ls` の通常ファイル、`docs/guides/README.txt` の documentation entryを同じ package API
  fixtureへ追加し、Rust tooling 4 tests、Rust MCP 11 tests、native MCP 109 testsを通過した。nested module `Geometry.Vec2` の
  name/function projection、docs directoryの source tree外扱い、native `doc` invocationの nested source pathを確認済みである。
  これは regular directory/file、`.ls` filtering、symlink拒否、docs exclusionの source ownership verified partialである。
  続く I-09 fixtureでは `src/Linked.ls` の直接 source file symlinkを外部 `Geometry` sourceとして辿らず拒否し、Unix socketを
  `src/S.ls` に置いた special filesystem entryは `.ls` sourceとして列挙しないことを確認した。Rust toolingは6 tests、Rust MCPは13 tests、
  native MCPは111 testsを通過し、native側はいずれも fake `doc` programを起動しなかった。これは直接 file symlinkと special entryを
  含む source ownership verified partialである。
  個別 source file symlink / 特殊 filesystem entry、installer、provider/auth、current-source runtime、
  Mac/Linux packaged/rollback parity はこの一件に含めない。

### まだ完了扱いにできない理由

- 個別 source file symlink / 特殊 filesystem entryの explicit fixture、installer / registry / provider/auth は未閉鎖。
- installer / registry / provider/auth / 実 Ed25519 / current-source Mac/Linux runtime / packaged parity は未検証。
- current-source V2-16b の Mac/Linux native gateは検証済みだが、installer / registry / provider/auth / packaged parity、
  全公開 surface、他の未対応言語機能は引き続き別タスクとして未完了である。

## Rust lane dev loop と Rust-free 日常化 (2026-08-16 追加)

Track 0 (Rust 側 dev loop の即効高速化) と Track 1 の `DEVLOOP-T1-1` / `DEVLOOP-T1-2` は完了し、
判断と代表 evidence は
[`decisions-dev-loop-rust-lane-speedup.md`](docs/adr/decisions-dev-loop-rust-lane-speedup.md)、
[`decisions-dev-loop-rust-free-daily-lane.md`](docs/adr/decisions-dev-loop-rust-free-daily-lane.md)、
[`rust-boundary-reduction.md`](docs/development/operations/rust-boundary-reduction.md) へ移したので
ここには残さない。以下は Track 0 / Track 1 が**開けたまま**にした残件である。

待ち時間は A (不要な Rust 再コンパイル) / B (L# コンパイラのスループット) / C (selfhost の差分ビルド)
に分解される。Track 0 が閉じたのは A だけであり、**C が閉じるまで「Rust 脱却で待ち時間が減る」は
成立しない**。順序は A → C → Rust 脱却 → B を継続改善とする。

- [~] `LEGACY-MODULE-01` selfhost/native module cache — 上記 `C`。既存項目 (本ファイル後段) を参照。
  `DEVLOOP-T1-2` を入れても **`selfhost/src` を編集した瞬間に両 lane とも fingerprint 不一致で `die`
  する**ため、source 編集ループの待ち時間は変わっていない。dev lane が救うのは
  「commit は進んだが `selfhost/src` は同一」のケースだけである。ここが本命。
- [ ] `NATIVE-HEAP-01` aarch64 確保系 helper の bounds check — `I-13` の帰結。
  `selfhost/src/Backend/Native/NativeCodegen.ls:14513` の
  `emit-aarch64-selfhost-alloc-helper` は 18 word / 72 bytes ちょうどで、decode しても
  **limit 比較も条件分岐も無い** (唯一の分岐は heap base 非ゼロ判定の `CBNZ x21`)。
  **対象はこの 1 つではない。** 全 selfhost helper を S 式評価で組み立てて数え直すと、
  frontier を進める helper は aarch64 10 個 / bump 11 箇所あり、**limit を参照するものは 0**。
  x86 は 9 個 / 9 箇所すべてが limit を参照する (全列挙は
  [`decisions-native-heap-reclamation.md`](docs/adr/decisions-native-heap-reclamation.md))。
  ただし aarch64 の 10 のうち `string-concat-helper-chunk3` は呼び出し元 0 (`I-25`) なので、
  **本項目が実際に手を入れる対象は 9 helper / 10 bump 箇所**である。
  **かつ「比較を足す」だけでは済まない** — aarch64 lane は `x21` (base) と `x22` (frontier) しか
  持たず、**上限値の置き場所が無い**。x86 の heap 先頭 16 bytes に倣うか、レジスタを 1 本増やすか、
  helper 内で base から計算するかを先に決める必要がある。
  heap 終端を越えた確保を検出して SIGSEGV ではなく診断メッセージで停止させる。
  `selfhost/src` の編集なので fingerprint が動き、stage0 再生成と両 target の
  native E2E 証跡が要る。単独の slice として扱う。
- [ ] `NATIVE-HEAP-02` native linear heap の回収機構 — `I-13` の本体。materializer は
  `calloc` 1 回 + bump のみで `free` / `munmap` / frontier reset が**一切無い**。
  **ただし「native に collector が無いから作る」ではない** — 回収機構の実体は既に 3 つある
  (wasm compiler world の mark-sweep、wasm http handler world の同型、selfhost の
  `Runtime/GC.ls`) が、**実行中に走るものが 0** である。compiler world は `main` return 後と
  `proc_exit(0)` のみ、http handler world は呼び出し元 0、`Runtime/GC.ls` は e2e fixture 専用。
  移植だけでは何も変わらない。
  消費は生存データ量ではなく累積確保回数に比例するため、heap 拡大は先送りにしかならない
  (4 GiB → 8 GiB へ倍増しても拡大分をちょうど使い切って落ちることを実測済み)。
  `NATIVE-HEAP-01` は症状を可視化するだけで、「115 KB の入力に 8 GiB 超」という増幅は解消しない。
  設計は [`decisions-native-heap-reclamation.md`](docs/adr/decisions-native-heap-reclamation.md)
  で in-design。**次の一手は方式の選択ではなく確保の帰属の動的計測** —
  aarch64 `map-new` の無条件 65,536 bytes 確保が主要容疑だが、静的な数え上げ
  (`(defn ` 6,656 個 × 64 KiB ≈ 416 MiB) では 8 GiB に届かない。
  計測は wasm lane でよい (呼び出し回数は lane に依らない)。
  `LEGACY-ROOT-01` / `LEGACY-IO-01` と関連。
- [ ] `NATIVE-STR-TAG-01` aarch64 文字列表現の bit 32 判別子が 4 GiB で破綻する — Issue `I-29`。
  稼働中の `emit-aarch64-selfhost-string-concat-helper`
  (`selfhost/src/Backend/Native/NativeCodegen.ls:15057`) は 2 引数それぞれに
  `tbnz xN, #63` + `tbz xN, #32` を出し、bit 32 が立つ値を**絶対番地の NUL 終端ポインタ**として
  strlen する。base 相対 offset が 4 GiB に達すると同じ値が誤読される。
  **受入条件**: (1) production materializer の heap 確保サイズを確定させる
  (現状 `native_host_bundle_alloc_size` は harness 側にしか無く、`data_frontier + 4 GiB` = 4 GiB 超)。
  (2) 4 GiB を越えるなら、判別子を heap 上限に依らない位置へ移すか heap を 4 GiB 未満へ抑えるかを
  ADR で決める。(3) 選んだ方の受入 test を置く。
  **この項目に含めない範囲**: heap の回収機構そのもの (`NATIVE-HEAP-02`)。
  ただし回収が入れば 4 GiB 未満に抑える案が現実的になるので、順序としては
  `NATIVE-HEAP-02` の後に (2) を決めるほうが選択肢が広い。
  **未確定**: bit 32 が立つ offset で `string-concat` が呼ばれる経路の有無は未確認。
  静的に確定しているのは「判別子の設計が 4 GiB で破綻する」ことだけである。
- [ ] `NATIVE-DEAD-01` 呼び出し元 0 の native emitter helper の裁定 — Issue `I-25`。
  `NativeCodegen.ls` の `emit-aarch64-selfhost-string-concat-helper-chunk1`〜`chunk4`
  (`:14944` / `:14973` / `:15002` / `:15031`) は定義だけで参照が無く、実際に使われる
  `emit-aarch64-selfhost-string-concat-helper` (`:15057`) は chunk を呼ばずに自前で
  77 word を組み立てている。chunk3 は frontier bump を含むため、確保系 helper の
  数え上げに紛れ込む (`NATIVE-HEAP-01` のスコープ確定で実際に 1 件拾った)。
  **受入条件**: (1) chunk1-4 と本体のバイト列突き合わせ — **済 (2026-08-19)**。
  長さは 77 word で一致するが `0x34` 以降 63 word が乖離する。差は **bit 32 の判定**
  (`tbz xN, #32` = `0xb6000137` / `0xb6000138`) の有無であって、bit 63 の判定
  (`tbnz xN, #63`) はどちらにもある (詳細は `I-25`)。
  当初ここに「chunk 群の方が後発 (`e9f761cb` 2026-04-29 > `901c10d8` 2026-04-22)」
  と書いたが、これは `git log -S` が拾った**名前の初出**であって実装の新旧ではなかった。
  実際は (2) のとおり chunk 群が旧実装である。
  (2) **どちらが正しいかを決める — 済 (2026-08-19)。実行は不要だった。**
  `e9f761cb^` 時点の本体 77 word と現 chunk1-4 の連結 77 word が **byte 完全一致**し、
  同じ `e9f761cb` が旧本体の削除・chunk 追加・新本体追加を同時に行っている。
  つまり **chunk 群は置き換えられた旧実装の分割コピー**で、残すのは本体である。
  `git log -S` の「chunk の方が後発」は名前の初出であって実装の新旧ではなかった。
  食い違う入力も特定済み: **bit 63 と bit 32 が両方 clear の非 0 値** (= tag の無い
  base 相対 offset) のみで、旧版はこれを絶対番地の NUL 終端文字列として誤読する。
  詳細と訂正 (当初 `tbz #63` と書いたが実際は `tbz #32`) は `I-25`。
  (3) 決めた側を残し、もう一方を削除して判断を `I-25` へ書く — **残るのはこれだけ**。
  chunk1-4 を削除する。`I-25` への書き戻しは済んでいるので、実作業は削除と検証。
  なお chunk1-4 は `crates/lsharp-wasm/tests` からも参照が無いので (`I-25` の
  「test からも参照なし」40 件に入る)、負けた方を削除しても test は壊れない。
  **含めない範囲**: chunk 群以外の未参照 defn の削除。**探索自体は完了しており** (64 defn /
  449 行、内訳は `I-25` の表。再現は `python3 scripts/native_codegen_dead_defn.py`)、
  本項目が裁定するのは乖離した別実装である chunk 群だけである。
  他 36 件は生きた関数への委譲か単一命令かのどちらかで意味論を持たない (`I-25` の表) ため、
  裁定が要るのは chunk 群だけである。
  加えて 64 件中 17 件は test に pin されていて単純削除できない (`I-25`)。
  `selfhost/src` の編集になるので fingerprint が動く。`NATIVE-HEAP-01` と同じ slice に
  まとめてよい。
- [ ] `NATIVE-TRAILER-01` x86 lane に helper trailer の補正が無い — Issue `I-26`。
  aarch64 は `aarch64-selfhost-helper-trailer-size` を (a) bundle 初期 capacity と
  (b) 末尾関数の entrypoint offset 再計算 (`NativeCodegen.ls:20828-20831`) の 2 箇所で使うが、
  x86 は (a) を `2048` の直書きで済ませ (`:10561`)、(b) の分岐自体を持たない (`:20791-20795`)。
  `x86-selfhost-helper-trailer-size` (`:10645`) は定義だけで呼び出し元 0。
  なお helper size の実合計は 2,486 bytes で、直書きの 2048 に 438 bytes 足りない。
  **受入条件**: (1) aarch64 に (b) の補正が入った経緯を特定する — **済 (2026-08-19)**。
  `bf35168d` (2026-05-04) が aarch64 に補正を入れ、x86 版の関数は 4 日後の `f56fcabd`
  (2026-05-08) が test と一緒に追加して接続しなかった。かつ aarch64 の補正は
  `collect-callable-actual-layout-aarch64` (`:18562`) の**実測 layout** を前提にしており、
  **x86 にはその実測 layout 自体が無い** (詳細は `I-26`)。
  (2) 同じズレが x86 にもあるかを、末尾関数を entrypoint にした
  bundle で `function-starts` の値と実測 offset を突き合わせて確認する。
  移植が必要なら実測 layout の x86 版から要る。
  **紛らわしい同名関数がある (2026-08-20 に確認)**: x86 側の
  `make-x86-function-emit-layout` (`:13993`) と付随する accessor 4 本 (`:14002`-`:14011`) は
  呼び出し元から base 値を受け取って束ねるだけの**宣言的なレコード**であって、
  `collect-callable-actual-layout-aarch64` (`:18562`) のような**実測**ではない。
  名前が似ているので対応物と誤認しないこと。x86 に実測 layout は雛形も存在しない。
  (3) 判断を `I-26` へ書く。補正が要るなら実装し、要らないなら capacity 計算へ接続する。
  **「削除する」は選択肢に入れない** — `.ls` の呼び出し元は 0 だが
  `crates/lsharp-wasm/tests` に 8 箇所の参照があり、消すと test が壊れる (`I-25`)。
  test ごと畳むなら、それは本項目ではなく別途の裁定が要る。
  **含めない範囲**: `I-23` (aarch64 側 pin の陳腐化) 本体の解消。別の pin である。
- [ ] `NATIVE-INLINE-01` x86 hot path の「user call を挟むな」制約の根拠を台帳へ起こす — Issue `I-27`。
  x86 codegen の 5 箇所で「wrapper を呼ばずに inline 展開する」ことが test の否定 assertion に
  よって pin されているが、**なぜ user call で壊れるのかはどこにも書かれていない**。
  導入は 4 件が 2026-05-17 の `361d0d99` (`wip:` 本文なし、5,391 insertions)、
  1 件が 2026-05-23 の `b9d5d4e5`。
  **受入条件**: (1) 最小再現を作る — hot path の 1 箇所を wrapper 呼び出しへ戻し、
  native 実行で値が壊れることを実測する。壊れないなら制約が既に陳腐化しているので、
  その事実を `I-27` へ書いて否定 assertion 側を畳む。
  (2) 壊れるなら原因が register/stack window か ref rooting か
  **selfhost lowering の local slot 取り違え**かを判別する (`I-27` の候補 (a)/(b)/(c))。
  **(c) を先に当たること** — `7f9fd01c` が disassembly (`[rbp-0x78]` へ書いて
  `[rbp-0x70]` を読む) で実測しており、3 候補のうち唯一の直接証拠である。(3) いずれの結果でも `I-27` へ書き戻し、
  回避形が要るなら **assertion message ではなく `NativeCodegen.ls` のコメントに**
  理由を置く。test が唯一の記録である状態を解消するのが本項目の目的である。
  **含めない範囲**: 根本原因の**修正**。判別と記録までで閉じる。
  修正は codegen の設計変更になるので別項目に切る。
  また回避前の 7 defn (`x86-function-emit-layout-*` 4 件、`native-call-rel-x86`、
  `emit-map-new-bundle-x86`、`emit-four-arg-call-x86-core`) の削除も含めない —
  削除しても否定 assertion は通るが、理由の記録先が決まるまで消さない。
  **cargo と native 実行が要る。** cargo 非依存でできるのは起票と、
  2026-08-19 に実施した後続事例 3 件の履歴突き合わせ (`I-27` の表) までである。

- [ ] `LINT-SPAN-01` lint 診断の span 投影が未実装で、全 lint が `0:0..0:0` へ落ちる — Issue `I-24`。
  `L0001` (unused binding) と `L0002` (empty do block) が実ソース
  `(defn main [] (let [unused (do)] 0))` に対し、どちらも range `0:0..0:0` で publish される
  (`test_e2e_selfhost_cli_lsp_stdio_didopen_preserves_distinct_same_start_diagnostics` が現状として pin)。
  対象の識別子・式の実 span を lint 診断へ載せる。
  **この項目に含めない範囲**: 重複判定の意味論。`I-24` で rule identity を含む形に裁定済みで、
  span が精密になっても判定規則は変わらない (同一 span に別 rule が正当に並ぶため)。
  **受入条件**: 上記 2 診断が別 range を持つこと、および `I-24` の pin 2 本が引き続き pass すること。
  **設計判断は 2026-08-19 に ADR で確定した**:
  [lint span の AST 表現](docs/adr/decisions-lint-span-ast-representation.md)。
  採ったのは「kind ごとに span 付き構築形を足す既存規約への追従」で、
  却下したのは全ノード一律の末尾 slot / 側テーブル / ソース再走査による近似。
  **旧記述の訂正**: 「selfhost の AST はそもそも位置情報を持っていない」は過度な一般化だった。
  var / string / float / apply / if / module-decl / import-decl / type-alias は既に
  byte offset の span を持つ。持たないのは `let` (tag 7) と `do` (tag 9) — L0001 / L0002 が
  対象とするまさにその 2 種だけである。
  **一律の末尾 slot は採れない**: AST ノード対象の長さ probe が 46 箇所あり、
  `TypeInfer.ls:60` は var ノードの長さ > 5 を qualified name の判別子に、
  `:114-115` は if ノードの長さ > 5 を span の有無の判別子に使っている。
  長さを一律に変えると**落ちずに誤動作する**。
  **旧記述の訂正 (2026-08-20)**: 当初ここに「span は byte offset、診断は line/col で、
  selfhost に offset → line/col 変換が存在しない (`selfhost/src` 全体で 0 件)。さらに
  `review-collect-node [node results]` (`DocTools.ls:790`) はソースを受け取らないので、
  走査の signature を変える必要がある。ここが本項目の実質的な重さである」と書いた。
  **2 つとも誤りである。** 変換は `lsp-position-from-offset` (`LspServerNav.ls:285`) と
  `lsp-range-from-offsets` (`:288`) として既にあり、呼び出し元は 7 箇所ある
  (`LspServerNav.ls:536` / `:579` / `:948` / `:1083`、`Cli.ls:1404-1405` / `:1455-1456`)。
  走査の signature も変えなくてよい。投影境界 `lsp-source-lint-diagnostics [src]`
  (`Cli.ls:1681`) が既に `src` を持っており、`src` を要するのは
  `lsp-review-diagnostic-to-lsp [diag]` (`Cli.ls:1660`) と、そこへ渡す
  `lsp-source-lint-diagnostics-loop` だけである。これは兄弟 2 本
  (`lsp-parse-diagnostic-to-lsp [diag src]` `:1400` /
  `lsp-type-diagnostic-to-lsp [code src start end]` `:1450`) が既に取っている引数であり、
  3 本の signature を揃える向きの変更になる。**本項目は当初見積もりより軽い。**
  **`0:0..0:0` の機構を特定した (2026-08-20)**: `DocTools.ls:713` / `:732` が
  `make-review-diagnostic` へ line/column を **1 1 で直書き**している。selfhost 内部は
  1-based で、`render-standard-diagnostic-json` (`LspServerCore.ls:613-616`) が JSON 境界で
  各座標から 1 を引いて 0-based の LSP range にする。1 − 1 = 0 が観測値そのものである。
  **第 2 の消費者がいる**: `docjson-render-review-diagnostic` (`DocJson.ls:111`) が同じ
  slot 4/5 を `line` / `column` として JSON に出し、
  `tests/snapshots/doctools/review-payload.json` が `line: 1, column: 1` を pin している。
  slot 4/5 の意味を offset へ変えると、この snapshot と 2 つのフィールド名が同時に嘘になる。
  しかも `generate-review-schema-json [ast source-id]` (`DocJson.ls:244`) は `src` を持たず、
  この経路では offset → line/col 変換ができない。よって **slot 4/5 は line/col のまま残し、
  offset は末尾 slot へ足す**。review 診断ベクタ (7 slot) に対する長さ probe は 0 件
  (`vector-length` が掛かるのは診断の**配列**側だけ) なので末尾追加は安全である。
  ただし LSP 投影後の 10 slot ベクタは長さ 8 / 10 を判別子に使っているので
  (`LspServerNav.ls:1198` / `:1200`、`LspServerCore.ls:636`)、そちらの長さは変えない。
  **docjson 経路の line/column は本項目に含めない**。1 1 のまま残る事実をここに明示しておく。
  **`Linter.ls` は LSP の生きた経路ではない**: `make-diagnostic ... 0 0` (`:203` / `:236`) は
  別系統で、didopen は `generate-review` → `DocTools` を通る。ここを直しても観測値は動かない。
  **追加の受入条件**: 変更対象 kind に長さ probe が無いことを実装前に grep で確認して
  ADR の Evidence へ記録する。offset → line/col 変換に単体 test を置く
  (行頭 / 行末 / 最終行 / 空行を含む)。
  **前者は 2026-08-19 に達成した** (cargo 非依存)。`scripts/lint_span_probe_survey.py` で
  6,458 defn を走査し、`let` / `do` の長さを**条件に使う分岐は 0 件**、
  掛かるのは診断系統の `(print (vector-length node))` 2 箇所
  (`Backend/Wasm/Compiler.ls:3108` / `:1434`) だけと確定した。ADR の Evidence に記録済み。
  同時に判明した制約: **`do` は `[9, expr-count, expr0, ...]` の可変長ノード**で、
  末尾 span が成立するのは全消費者が `vector-length` ではなく slot 1 の `expr-count` で
  走査を打ち切っているためである (実測で成立)。この不変条件を壊す変更を同時に入れない。
  残るのは offset → line/col の単体 test だが、**変換自体は既にあるので新規実装ではなく
  既存 `lsp-position-from-offset` への test 追加**である (行頭 / 行末 / 最終行 / 空行 /
  `offset == string-length` の境界)。改行判定は char 10 のみで CR は列文字として数える。
  これは `lsp-offset-from-line-col` (`:276`) と逆向きで一貫しており、本項目では変えない。

  **この項目に含めない範囲**: `sort-diagnostics` 側の順序規則 (AC-208 で別途固定済み)。
- [BLOCKED: CI 自動実行が 2026-07-12 から停止中で、push では 1 run も起動しない]
  `SMOKE-GATE-03` `default-path-smoke` job が緑になることの 1 run 観測 — Issue `I-15` / `I-19`。
  受入条件 (a) skipped の原因特定は **2026-08-18 に達成**した。原因は job 側の条件ではなく
  `ci.yml` 全体が `workflow_dispatch` 限定になっていたことで、`I-15` と `I-19` に記録済み。
  残るのは (b)「実際に 1 run 走って緑になる」だけで、これは CI 停止中は push で確認できない。
  **解除条件**: CI 自動実行が再開されるか、`workflow_dispatch` の手動起動を行うか。
  **含めない範囲**: script の assertion 内容 (決着済み)、CI 停止方針そのものの是非 (`I-19`)。
- [ ] `NATIVE-ROOT-02` native x86-64 lane の root API を実装する — Issue `I-21` の残件。
  `emit-root-push-x86` は `xor eax, eax` を出すだけで引数を捨てて常に 0 を返し
  (tier 1 項目 1 違反)、`root_pop` には emitter そのものが無く、`root_set` は store を出さない。
  aarch64 側は `NATIVE-ROOT-01` で tier 1 適合済み
  ([空 stack ガード ADR](docs/adr/decisions-native-root-pop-empty-guard.md))。
  契約は [root API 契約 ADR](docs/adr/decisions-runtime-spec-root-api-contract.md) が正本。
  受入条件は tier 1 の 4 項目を x86-64 で満たし、aarch64 と同じ observable な結果になることを
  検査する test を置くこと (`test_e2e_native_host_binary_selfhost_root_pop_on_empty_stack_keeps_stack_pointer`
  の x86-64 対応版が最小形)。
  **codegen だけでは閉じない**: root stack の確保と base/pointer レジスタの初期化は
  `NativeCodegen.ls` ではなく link 時の entry stub が持つ (`I-21` の 2026-08-18 追記)。
  aarch64 は e2e harness (`selfhost_native_stage_chain.rs:37368-37377`) と製品 materializer
  (`scripts/ci/materialize-native-macos-aarch64-bundle.py:131-133,170`) の両方に stub があるが、
  **Linux x86 materializer には root stack が無い**。受入条件には
  `scripts/ci/materialize-native-linux-x86-bundle.py` への追加も含める。
  **含めない範囲**: native lane への GC 導入 (`NATIVE-HEAP-01/02` が持つ)。
  容量を動的にすること (`NATIVE-ROOT-03` が持つ。x86-64 も aarch64 と同じ固定上限で揃えてよい)。

- [ ] `NATIVE-ROOT-03` native lane の root stack を tier 1 項目 4 (動的容量) に適合させる —
  Issue `I-21`。現状は固定 8 MiB の BSS ブロックで、`emit-root-push-aarch64` は容量検査を持たず、
  上限超過は trap せず隣接 bss を壊す。契約は
  [root API 契約 ADR](docs/adr/decisions-runtime-spec-root-api-contract.md) の tier 1 項目 4
  (「容量は動的で固定上限を定めない。確保できなくなった時点で trap する」) が正本。
  **受入条件**: 初期容量を超える root_push が (a) 拡張に成功するか (b) trap するかのどちらかであり、
  隣接メモリを壊さないことを検査する test を置くこと。wasm 側の
  `test_e2e_runtime_root_stack_grows_past_initial_capacity` が同趣旨の pin なので形を揃える。
  **含めない範囲**: x86-64 の root API 実装そのもの (`NATIVE-ROOT-02`)、
  GC 導入 (`NATIVE-HEAP-01/02`)。拡張方式 (mmap 再確保か倍々か) は実装時に決めてよい
  (契約は「動的であること」までしか定めていない)。
- [ ] `NATIVE-IMPORT-ABI-01` x86 native の int-to-string import 呼び出しで rdi が書かれない
  問題を直す — Issue `I-28`。

  **根本原因は 2026-08-19 のソース読解で確定した (候補 (a)(b) はどちらも否定)。**
  失敗 test が使う harness `run_selfhost_main_native_x86_segmented_host_bytes_harness_with_payload_and_args`
  (`selfhost_native_stage_chain.rs:19572`) は、seed の `push-import-placeholders` (`:350-365`) が
  10 個の import placeholder を **一様に param-count 0** で積む。`int-to-string` は
  `ftable-with-native-runtime-imports` (`CompilerBase.ls:428`) で **func-idx 6** に登録されるので、
  `function-metas[6]` の param-count が 0 になり、`codegen-x86-opcode-call-bundle` の
  `(= target-param-count 1)` 分岐に入れない。`48 89 c7` は構造的に出ない。

  **直し方は既にリポジトリ内にある。** `root_linux_x86_seed_int_to_string_import_arity` (`:348`) が
  既定版を `(if (= idx 6) 1 0)` へ書き換える rewriter で、`:663` の Linux-x86 root seed だけが
  これを適用している。失敗 test の harness にも同じ rewriter を通すのが第一手である。

  **受入条件**:
  1. `test_e2e_selfhost_x86_int_to_string_import_sets_rdi` が GREEN になる
  2. **rewriter を適用しただけで GREEN になったなら、それは production の証明ではない。**
     `selfhost/src/**.ls` に import meta の構築点が存在しない事実 (`I-28` の
     「production 側の帰趨は未確定」節) をどう扱うかを結論づけ、
     production 側にも欠陥があるなら別 ID を起票する
  3. 同族の `..._x86_function_size_...` (`STALE-HARNESS-01`) と混ぜない
  4. **巻き添えを確認する。** rewriter は seed を書き換えるので、同じ harness
     `run_selfhost_main_native_x86_segmented_host_bytes_harness_with_payload_and_args`
     を共有する test すべてに効く。import 6 が 1 引数呼び出しになると call site の
     byte 長が 7 byte (`50 e8 .. .. .. .. 59`) から 10 byte
     (`48 89 c7 51 e8 .. .. .. .. 59`) へ伸びるため、`int-to-string` を呼ぶ経路の
     byte offset / size pin を持つ現状 GREEN の同族 test がずれうる。
     ずれたものを「陳腐化」と即断せず、伸びが上記 3 byte 差で説明できるかを先に確かめる

  **期待値を実装に合わせて書き換えるのは禁止** — `48 89 c7 51 e8 .. .. .. .. 59` は
  `emit-one-arg-call-x86-core-with-call-bytes` (`:7179-7197`) の出力そのもので、古い定数ではない。
  **含めない範囲**: `I-27` の値破壊 (別原因)、`NATIVE-ROOT-02/03`、
  `wrap-ir-functions-as-meta-loop` (`:20577-20585`) が param-count 0 を固定する件
  (`emit-native-bundle` 経路の別個の欠陥として `I-28` に記録済み)。
  **cargo と native 実行が要る。**

- [ ] `ROOT-SLOT-PROBE-01` selfhost bundle 実行後の root slot 残留を検査する regression guard —
  main の `crates/lsharp-wasm/tests/e2e/selfhost_rooting_parity.rs` は **codegen が `root_push` を
  出すか**を 24 test で見ているが、**bundle を実行し終えた後に caller が積んだ root slot が
  そのまま残っているか**は見ていない。`root_top` が caller-owned slot を食い潰す種類の
  imbalance は、現状どの test 経路でも落ちない。
  参照実装が `codex/v0.2-ec-m1-06-all-form-differential` の `b415f8cb` にある
  (`crates/lsharp-wasm/tests/root_slot_probe.rs`、179 行)。型推論 20 module の MODULES 列を
  Wasm harness で回し、nested allocation 後の `root_top` を突き合わせる。
  同 commit が直した `TypeInfer.ls` / `TypeInferFunctions.ls` の余分な `root_pop` 2 箇所は
  **main では既に balance 済み**なので、移植対象は probe だけである
  (ADR [`decisions-worktree-absorption-2026-08-20.md`](docs/adr/decisions-worktree-absorption-2026-08-20.md))。
  受入条件: 意図的に `root_pop` を 1 個増減させた fixture で非 0 になること。
  **含めない範囲**: native lane の root API 実装 (`NATIVE-ROOT-02`)、
  root stack の動的容量 (`NATIVE-ROOT-03`)。**cargo が要る。**

- [ ] `FMT-ROUNDTRIP-01` AST / token の Display を re-parse 可能にする — Issue `I-36`。
  `crates/lsharp-syntax/src/ast.rs:602` と `crates/lsharp-syntax/src/token.rs:71` が
  string literal の escape を復元せず、`ast.rs:325` (defn) / `:519` (lambda) /
  trait method / defmacro が parameter の型注釈と `where_clauses` / `return_ty` /
  `macro_type` を落とす。参照実装は `codex/legacy-maintenance-stage-chain-integration` の
  `05b98847` (共通 `fmt_string_literal`) と `fe5ed3c1` (`Param` / `WhereClause` の Display) だが、
  1510 commit 越しに patch は当てず main の現行 file 構成の上で書き直す
  (ADR [`decisions-worktree-absorption-2026-08-20.md`](docs/adr/decisions-worktree-absorption-2026-08-20.md))。
  受入条件: `test_gen.rs` の generator を escape 必須文字と `ty: Some(..)` / `where_clauses`
  付き `Decl` まで広げたうえで、`pretty_printed_ast_reparses_to_the_same_source` が PASS すること。
  **generator を広げずに Display だけ直すのは不可** — 現状 gate が盲点を持っていること自体が
  `I-36` の根拠の半分である。
  **含めない範囲**: metadata projection の Display (別契約)、`fmt` サブコマンドの CLI 公開判断。

- [ ] `GC-LEAK-CYCLE-01` 強制回収ごとに live allocation が baseline へ戻ることを契約化する —
  Issue `I-06`。main の GC 検証は object table / free list / root stack の**容量成長** 5 件
  (ADR [`decisions-legacy-test-runtime-limits-lane.md`](docs/adr/decisions-legacy-test-runtime-limits-lane.md))
  と soak 2 件で、**`__lsharp_gc_collect` を強制した後に live が 0 へ戻るか**は見ていない。
  参照実装は `codex/legacy-maintenance-stage-chain-integration` の `8be951e4` に含まれる
  `test_e2e_runtime_collector_returns_live_allocations_to_baseline_after_each_cycle`
  (128 allocation x 10 cycle の mixed-size churn)。
  受入条件: cycle ごとに `live=0` を assert すること。
  **branch が記録している `freed` / free-list entry 数の実測値 (10 cycle で 384→402 等) は
  旧 first-fit 設計に紐づくので期待値に使わない。** size-class heads の下では違う数になる。
  **含めない範囲**: 到達不能な legacy free-list path の修正 (`I-35`)、GC アルゴリズムの変更。

- [ ] `ALLOC-DEAD-BR-01` allocator の到達不能 free-list search を直すか消すか決める —
  Issue `I-35`。`crates/lsharp-wasm/src/wasi/allocator.rs:140` の無条件 `Br(0)` が
  legacy first-fit search 全体を skip しているため、`:172` の誤った `Br(0)`
  (`Br(1)` であるべき) は現状発火しない。path を再有効化すると無限 loop する。
  受入条件: `I-04` (free list 線形探索) の設計判断とセットで、
  「ABI 互換のために残す」なら誤りを直す、「残さない」なら区間ごと削る、のどちらかへ倒すこと。
  **含めない範囲**: size-class allocator 自体の変更。

- [ ] `RUST-FILE-SIZE-GATE-01` workspace 全域の 800 行 gate を入れる — Issue `I-01`。
  main は per-file の targeted guard 7 本 (`*_file_size.rs`) しか持たず、`crates/**/src/**` と
  `crates/**/tests/**` を走査する gate が無い。参照実装は
  `codex/legacy-maintenance-stage-chain-integration` の `e6ae428e` / `3c37f574`
  (`crates/lsharp-wasm/tests/rust_file_size_contract.rs` + allowlist 2 本)。
  **2026-08-22 実測で main の allowlist は 39 件必要** (src 6 / tests 33)。最大は
  `crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs` の 62990 行、次が
  `selfhost_cli_core.rs` 19412 行。branch の allowlist は 28 件だった。
  受入条件: allowlist が単調減少すること (追加には ADR を要求する)。
  **含めない範囲**: 超過 file の分割そのもの (`LEGACY-MAINT-01`)。gate と分割は別 slice。

- [ ] `MODULE-DUP-FN-01` 別 module の同名 top-level function を衝突させない — Issue `I-37`。
  現状は `A.helper` と `B.helper` が 1 つに潰れ、**診断なしに誤った wasm を出す**
  (`1 + 20` が `20 + 20` = 40 になる実測あり)。import 順に依らないので、
  登録先の key が module で修飾されていない。参照実装は
  `codex/legacy-maint-native-stage-chain-split` の `f5a343a8` (`incremental/scoped_visibility.rs`) だが、
  main には `crates/lsharp-ir/src/incremental/` が無く 1540 commit 越しなので patch は当てず、
  main の `compile_pipeline.rs` / `compile_surface.rs` の上で書き直す
  (ADR [`decisions-worktree-absorption-2026-08-20.md`](docs/adr/decisions-worktree-absorption-2026-08-20.md))。
  受入条件: 同名 function を持つ 2 module を import した program が
  **(a) 正しい値を返すか (b) 曖昧参照として診断で落ちるか**のどちらかへ倒れること。
  どちらへ倒すかを ADR に書いたうえで、その契約を e2e (wasmtime 実行して値を assert) で張る。
  **compile が成功して誤った値を返す状態は、どちらの設計でも不合格。**
  **前提**: 勝敗は import 順ではなく **module 名の辞書順で最後が勝ち、entry module は常に勝つ**
  (`compile_surface.rs:34`)。selfhost はこの上書きに**意図的に依存している** —
  `selfhost/src/Types/TypeInfer.ls:219-225` は `TypeInferBlock.ls が上書き` と書いた stub を置き、
  `App.Cli` 閉包だけで 65 件の衝突 (本文一致 38 / 相違 27) がある。
  qualify に倒せば `TypeInfer.ls` 内部の呼び出しが stub へ落ちて**型推論が黙って劣化**し、
  reject に倒せば 65 件が一斉に落ちる。**どちらでも selfhost 側の重複整理が先**で、
  `TYPEINFER-SPLIT-01` と順序を組むこと。selfhost を直さずに Rust 側だけ倒すのは不可。
  **含めない範囲**: `type-alias` の module 越し可視性 (`MODULE-ALIAS-EXPORT-01`)、
  block 形式 module body (`MODULE-BODY-FORM-01`)、selfhost の重複整理そのもの
  (`TYPEINFER-SPLIT-01` / `LEGACY-MAINT-01`)。

- [ ] `MODULE-ALIAS-EXPORT-01` `type-alias` の module 越しの扱いを決める — Issue `I-38`。
  現状 alias は同一 file 内でしか展開されず、import 先で使うと
  `型の不一致: expected String, found Text` になる。修飾しても `found A.Text` になるだけで、
  **未知の型名としては弾かれない**。`crates/lsharp-types/src/infer.rs:86` の `type_aliases` が
  単一 map で、multi-file 経路 (`infer/decl.rs:160`) が `{module}.{name}` で register する一方、
  展開側 (`infer.rs:262` / `:276` / `:293`) が解決後の名前で lookup している。
  参照実装は同 branch の `cfcb19a7` (`incremental/scoped_type_alias.rs`)。
  受入条件: 「alias を export する」に倒すなら `:only` / `:open` の可視性と組んだ contract を、
  「export しない」に倒すなら **未定義型としての診断**を、どちらかを ADR に書いて test で張ること。
  **現状の型不一致診断はどちらの設計でも誤りなので、必ず消えること。**
  **含めない範囲**: parametric alias の高階化、`type-constrained` の可視性。

- [ ] `MODULE-BODY-FORM-01` block 形式 module body を実装するか reject するか決める — Issue `I-39`。
  `(module M (defn f ...) ...)` は parser が受理する (`Decl::ModuleDecl { name, body }`) が、
  型推論と lowering が body 内の宣言を登録しないため sibling 参照が
  `未定義の変数` になる。**repo 内の `.ls` でこの形を使っているものは 0 件**なので
  regression ではなく、parser だけが先行して受理している未搭載 surface である。
  参照実装は同 branch の `a5e5929c` / `fa7b4c51` (`incremental/root_module_body.rs`) と
  `68849d55` (nested alias target scope)。
  受入条件: 実装するなら flat 形と同じ program が同じ値を返すことを e2e で、
  reject するなら **parse 時点で「未対応の構文」と分かる診断**を、どちらかを ADR に書いて張ること。
  **含めない範囲**: 入れ子 module (`(module A (module B ...))`) の可視性設計。
  まず 1 段の body を決めてからにする。

- [ ] `DOCTOOLS-META-SLOT-01` DocTools の metadata 契約を parser の 6 slot に合わせる —
  Issue `I-40`。`selfhost/src/Syntax/Parser.ls:1140` は 6 slot を返すが
  `selfhost/src/Tools/Doc/DocTools.ls:120` のコメントは 5 slot を宣言したままで、
  `extract-defn-metadata` が raw vector をそのまま返している。consumer 4 件は
  index guard 済みなので**現状の実害は無く、契約文書と実装の不一致だけ**である。
  受入条件: コメントを 6 slot へ直すか、accessor 側で slot 5 を落とすかを決めて test で張ること。
  **`MODULE-DUP-FN-01` より先には倒せない** — `Tools.Text.FormatterDecl` が同名 accessor を
  持っていて辞書順で勝つため、DocTools 側だけ切り詰めても発火しない。さらに
  `FormatterDecl.ls:323` / `:414` は slot 5 を実際に読むので、切り詰めた側が勝てば formatter が壊れる。
  参照実装 `67624ca7` (`codex/legacy-maint-native-differential-split-audit`) は
  **この理由で却下済み** (ADR)。
  **含めない範囲**: 重複名そのものの解消 (`MODULE-DUP-FN-01`)。

- [ ] `STALE-HARNESS-01` `function_size_matches_generated_length` 診断の埋め込み harness を
  bundle 分割後の emitter へ寄せる — Issue `I-23` の `STALE-PIN-03` 裁定 [B]。
  `crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs:43891` の `check-instr-sizes` は
  サイズを `native-instr-size-x86` (drop bundle 込み) で見積もりながら、実バイト列を
  bundle 分割前の `emit-control-instr-x86` で作っている。実経路
  (`append-control-if-instr-x86` -> `emit-control-if-bundle-x86`) と揃えるなら
  `emit-control-instr-bundle-x86 ir meta offsets idx frame-base-slot-count depth` を呼ぶ
  (両変数とも `check-instr-sizes` のスコープに既にある)。
  **直し方は確定している (2026-08-19 にソース読解で決着)。** production の
  `codegen-x86-control-loop-fallback-native` (`:11204-11216`) が
  **harness と同じ形の分岐を既に持っている**:

  ```
  (if (= (is-control-opcode opcode) 1)
    (emit-control-instr-bundle-x86 ir-func meta offsets idx frame-base-slot-count current-depth)
    ...)
  ```

  harness (`:43891`) はこの行の陳腐化コピーで、末尾 2 引数を落として非 bundle 版を呼んでいる。
  **production の行へ揃えるだけでよく、新しい判断は要らない。**
  `frame-base-slot-count` / `depth` はどちらも `check-instr-sizes` のスコープに既にある。
  **受入条件**: (1) `..._representative_x86_function_size_matches_generated_length_diagnostic` が
  `Some(-1)` を返すこと、(2) 修正後も**診断は最初の mismatch で止まる**ので、
  1 回 GREEN を見て終わりにせず、残る mismatch が無いことまで確認すること。
  当初 (3) として「opcode 80/81/83 で実ループと同じ列を出すか」を置いていたが、
  **ソース読解で discharge した**ので受入条件から外した。根拠は 2 つ:
  `emit-control-instr-bundle-x86` が扱う opcode 集合 {41, 79, 80, 81, 83} は
  `native-control-instr-size-x86` (`:12275-12285`) の非 0 集合と完全一致し、
  `native-instr-size-x86` (`:8997-9002`) が bundle 込みサイズへ回す 3 つ (41 / 81 / 83) は
  bundle 版と非 bundle 版で出力が異なる 3 つと厳密に一致する。79 / 80 は両版で同一である。
  **含めない範囲**: production の legacy 非 bundle 経路
  (`generate-native-instr-loop-x86`。offset 側も `native-plain-instr-size-x86` を使うので
  内部整合しており、欠陥ではないと確認済み)、
  `test_linux_x86_metadata_replays_control_if_control_loop_single_row` (`:8257`。
  production `:12994` へのソース文字列 pin であり本件と無関係)。
  **同族 3 件**: `ignored-lane-expected-failures.txt` の `:135` / `:136` は Lima VM 依存で
  未実測の同族候補、`:189` が裁定済みの本体。加えて
  `test_e2e_native_aarch64_map_insert_instr_size_matches_emitted_length` (`:16762`、
  harness `:18007`) が同じ書き方をしている。**行の削除は GREEN が出てから。**
  **cargo が要る。**
- [~] `PARSER-PARITY-01` metadata directive の allowlist が二重管理 — Issue `I-18`。
  parity test (`crates/lsharp-syntax/tests/metadata_directive_parity.rs`) は設置済みで、
  片側だけの directive 追加は検出できる状態になった。判断と実測は
  [directive allowlist parity ADR](docs/adr/decisions-parser-directive-allowlist-parity.md) が正本。
  受入条件の文言 (一覧の**一致**を検査) は満たしておらず、差分を pin する形にした
  (3 者は正しく運用していても一致しないため。判断は ADR の「受入条件との差」節)。
  残るのは (1) `:roots-unbalanced` の selfhost への port、
  (2) 一覧を単一正本へ寄せる設計変更、(3) `I-20` の受理と読み取りの乖離。
  **含めない範囲**: 上記 3 件はいずれも本項目では扱わない。
- [~] `DOC-07` ドキュメント同期ハーネス — `.claude/rules/doc-sync.md`、`.claude/hooks/doc-guard.sh`、
  `.claude/skills/doc-sync/` を追加した。残るのは実運用での有効性確認と、hook が「正しい正本へ
  正しい粒度で書かれたか」までは判定できない点の運用での補完。

## Next milestone — v0.3 review provenance / lifecycle

正本: [`v0.3-review-provenance-lifecycle.md`](docs/development/planning/v0.3-review-provenance-lifecycle.md)

次版の設計と task 分解（v0.3 completion 後に active backlog へ昇格）:
[`v0.4-lsharp-next-shape.md`](docs/development/planning/v0.4-lsharp-next-shape.md)、
[`v0.4-milestone-01.md`](docs/development/planning/v0.4-milestone-01.md)。

v0.4 M1-01 の semantic fixture matrix では、artifact を要求する fixture が `compile` または
`build` を commands に宣言しない場合を validator が拒否する command/artifact scope boundary を
verified partial として追加した。実 Rust/native artifact、runtime、Mac/Linux matrix は未接続のため
v0.4 の完了項目には移さない。さらに artifact/runtime の期待値と `codegen`/`wasm`、
`runtime`/`runtime` の layer・observable 対を validator が拒否する scope boundary を verified
partial として追加した。実 Rust/native artifact、runtime、Mac/Linux matrix は未接続のため
v0.4 の完了項目には移さない。さらに evidence index の report/comparison/ADR 参照で symlink
traversal と project root 外への resolve を拒否する safe-reference boundary を verified partial として
追加した。実 Rust/native artifact、runtime、Mac/Linux matrix は未接続のため v0.4 の完了項目には
移さない。ADR: `docs/adr/decisions-v0.4-m1-01-artifact-command-boundary.md`、
`docs/adr/decisions-v0.4-m1-01-scope-boundary.md`、
`docs/adr/decisions-v0.4-m1-06-evidence-reference-boundary.md`。さらに evidence index の
`task` と fixture matrix `suite` の identity mismatch を拒否する task-scope boundary を verified
partial として追加した。実 Rust/native artifact、runtime、Mac/Linux matrix は未接続のため v0.4 の
完了項目には移さない。さらに artifact を要求する evidence index の command を `compile`/`build`
に限定し、`check` route で artifact/runtime を宣言できない command-scope boundary を verified
partial として追加した。実 Rust/native artifact、runtime、Mac/Linux matrix は未接続のため v0.4 の
完了項目には移さない。ADR: `docs/adr/decisions-v0.4-m1-06-task-scope-boundary.md`、
`docs/adr/decisions-v0.4-m1-06-command-scope-boundary.md`。さらに oracle/native report の producer
role (`rust-oracle` / `native-stage0`) の取り違えを拒否する producer-scope boundary を verified partial
として追加した。実 Rust/native artifact、runtime、Mac/Linux matrix は未接続のため v0.4 の完了項目には
移さない。ADR: `docs/adr/decisions-v0.4-m1-06-report-producer-boundary.md`。さらに report/index の
`source_commit` を指定 root の current `HEAD` に束縛し、同じ stale commit を全入力へコピーしても
pass しない current-source boundary を verified partial として追加した。実 Rust/native artifact、runtime、
Mac/Linux matrix は未接続のため v0.4 の完了項目には移さない。ADR:
`docs/adr/decisions-v0.4-m1-06-current-source-boundary.md`。さらに evidence index の `adr` を
`docs/adr/*.md` に限定する ADR-reference boundary を verified partial として追加した。実 Rust/native
artifact、runtime、Mac/Linux matrix は未接続のため v0.4 の完了項目には移さない。ADR:
`docs/adr/decisions-v0.4-m1-06-adr-reference-boundary.md`。
同じ `docs/adr/*.md` path shape を JSON Schema にも反映する schema-parity boundary を verified partial
として追加した。実 Rust/native artifact、runtime、Mac/Linux matrix は未接続のため v0.4 の完了項目には
移さない。ADR: `docs/adr/decisions-v0.4-m1-06-adr-schema-boundary.md`。
さらに JSON Schema の `task` を `V4-M1-01` constant に固定する task-schema parity boundary を verified
partial として追加した。実 Rust/native artifact、runtime、Mac/Linux matrix は未接続のため v0.4 の完了項目には
移さない。ADR: `docs/adr/decisions-v0.4-m1-06-task-schema-boundary.md`。

さらに evidence index の oracle/native report と comparison の参照を task-owned `ci-artifacts/`
配下に限定する artifact-namespace boundary を schema と executable audit に揃えた verified partial
として追加した。実 Rust/native artifact、runtime、Mac/Linux matrix は未接続のため v0.4 の完了項目には
移さない。ADR: `docs/adr/decisions-v0.4-m1-06-artifact-namespace-boundary.md`。

さらに evidence artifact の report/comparison path を
`ci-artifacts/v4-m1-01/<source_commit>/<target>/` に分離し、同一 source commit の Mac/Linux
結果の上書きと target 取り違えを拒否する target-scoped artifact boundary を executable audit に接続した
verified partial として追加した。実 Rust/native artifact、runtime、Mac/Linux matrix は未接続のため v0.4 の
完了項目には移さない。ADR: `docs/adr/decisions-v0.4-m1-06-target-artifact-boundary.md`。

さらに evidence index の JSON Schema に Mac/Linux target ごとの `if`/`then` path binding を追加し、
schema-only consumer が別 target の report/comparison path を受け入れない verified partial boundary を
固定した。source-commit freshness、regular-file、symlink、project-root は executable audit の責務として
残り、実 Rust/native artifact、runtime、Mac/Linux matrix は未接続のため v0.4 の完了項目には移さない。
ADR: `docs/adr/decisions-v0.4-m1-06-target-schema-binding.md`。

さらに evidence index 自体も同じ target-scoped artifact namespace 配下の regular file に限定し、
bundle 外の index や symlink index を report/comparison と結び付けられない index-ownership boundary を
executable audit に追加した verified partial として記録する。実 Rust/native artifact、runtime、Mac/Linux
matrix は未接続のため v0.4 の完了項目には移さない。ADR:
`docs/adr/decisions-v0.4-m1-06-index-artifact-boundary.md`。

さらに Mac Apple Silicon と Linux x86_64 の target index を再監査して束ねる two-target aggregate
audit/schema を追加した。片側 pending/mismatch や Mac-only pass を aggregate pass に昇格させない
verified partial として記録し、両 target の selected fixture IDs も一致させる。実 target artifact/runtime、rollback、provider parity は未接続のため
v0.4 の完了項目には移さない。さらに再計算された aggregate result の top-level/target 別 fixture IDs、
fixture count、pending boundary、mismatch projection を JSON Schema に固定した。current source、target order、
cross-target scope は executable audit のまま残る。さらに aggregate input schema の `prefixItems` で
Mac→Linux order と target-scoped `index.json` path を固定し、schema-only consumer の取り違えを拒否する。
ADR: `docs/adr/decisions-v0.4-m1-06-two-target-aggregate.md`、
`docs/adr/decisions-v0.4-m1-06-aggregate-result-schema.md`、
`docs/adr/decisions-v0.4-m1-06-aggregate-index-schema.md`。
さらに semantic fixture matrix の source path は各 component の symlink traversal を拒否し、manifest root
が所有する regular `.ls` file のみを受け付ける source-ownership boundary を verified partial として
追加した。実 Rust/native artifact、runtime、Mac/Linux matrix は未接続のため v0.4 の完了項目には移さない。
ADR: `docs/adr/decisions-v0.4-m1-01-source-symlink-boundary.md`。
さらに current source commit `ed72cb59987dfb8523886f775ab9170ecc436cc6` の Mac Apple Silicon Rust-oracle
で14件の valid fixtureを実行し、regular Wasm artifact、digest/size、Wasmtime runtime stdout/stderr、exit `0`
を確認した。`examples/module.ls` の実際の式 `3 * 4 + 5` に合わせ、`valid/module-import` の stale な期待値
`41\n` を `17\n` へ修正し、matrix contract testで固定した verified partial である。native stage0、Linux
x86_64、full invalid、Rust/native differential、aggregate は未接続のため v0.4 の完了項目には移さない。
ADR: `docs/adr/decisions-v0.4-m1-01-rust-oracle-valid-batch.md`。

さらに current source commit `3b6039fcd3f91e5d5c266aaeaa2f87af7c349948` の Mac Rust-oracle invalid laneを
5 fixture 個別実行し、`invalid/record-field-pattern-literal` (`LS3001`, line 8 columns 19–21) と
`invalid/type-undefined-value` (`LS1001`, line 1 columns 16–29) の code/span/exit/no-artifact boundaryを
確認した。lexer unexpected character、module-not-found、parser unexpected EOF は code または span が
欠落するため producer が synthetic に補完せず拒否し、pending のまま残す verified partial である。
native stage0、Linux x86_64、full invalid parity、Rust/native differential、aggregate は未接続のため
v0.4 の完了項目には移さない。ADR: `docs/adr/decisions-v0.4-m1-01-rust-oracle-invalid-batch.md`。

さらに diagnostic parity 実装を含む current source commit
`6943f488a213e63b5612eeabefe106357c922427` の Mac Rust-oracle で invalid 5件を再実行し、
`LS0001` (line 1 columns 1–2)、`LS3102` (line 1 columns 1–23)、`LS0102`
(line 1 columns 1–14)、`LS3001` (line 8 columns 19–21)、`LS1001` (line 1 columns 16–29) の
code/span/exit/no-artifact/runtime-not-run boundary を全件確認した。これは V4-M1-01 の
verified partial sliceであり、native stage0、Linux x86_64、Rust/native differential、aggregate は
未接続のため `[~]` を維持する。ADR:
`docs/adr/decisions-v0.4-m1-01-rust-oracle-invalid-diagnostic-parity.md`。

さらに current source commit
`8af9af3c30b8260700ca6b7b05030a56c42805e3` の Mac Rust-oracle で semantic fixture matrix の 19件を
一括実行した。valid 14件は全て Wasm artifact を生成し、`wasm-tools validate`、Wasmtime exit `0`、
期待 stdout/stderr、artifact digest/size を確認した。invalid 5件は期待する `LS0001` / `LS3102` /
`LS0102` / `LS3001` / `LS1001` の code/span、exit `1`、no-artifact、runtime-not-run を確認した。
これにより Mac Rust-oracle の current-source full matrix は verified partial になったが、native
stage0、Linux x86_64、Rust/native differential、aggregate は未接続のため `[~]` を維持する。ADR:
`docs/adr/decisions-v0.4-m1-01-rust-oracle-current-source-full-batch.md`。

さらに native-stage0 report producer の implementation commit
`bf7878926a3f937da93bf0b07744874ea54d8a22` で、runner に渡す source を fixture work directory の
task-owned copy へ隔離した。mutating runner の RED/GREEN と native producer 12 tests により、
evidence 生成が checkout の source を変更しない verified partial boundary を確認した。実 native
stage0 package/runtime、Linux x86_64、Wasm validation/runtime、Rust/native differential、aggregate は
未接続のため `[~]` を維持する。ADR:
`docs/adr/decisions-v0.4-m1-01-native-report-source-isolation.md`。

さらに semantic diff helper の implementation commit `4b70bb7d` で observed artifact の `size=0` を
拒否し、空ファイルを pending/pass の artifact evidence として扱わない positive-size boundary を
verified partial として追加した。実 Wasm validation/runtime、native stage0、Linux、Rust/native
differential、aggregate は未接続のため `[~]` を維持する。ADR:
`docs/adr/decisions-v0.4-m1-01-nonempty-artifact-boundary.md`。

さらに Rust-oracle/native-stage0 producer の implementation commit `a20fae09` で、明示した
`--wasm-tools` による `validate` を Wasmtime 前に必須化した。invalid bytes は report を生成せず runtime も
起動しない verified partial boundary である。実 Mac/Linux native artifact/runtime、Rust/native differential、
aggregate は未接続のため `[~]` を維持する。ADR:
`docs/adr/decisions-v0.4-m1-01-wasm-validation-boundary.md`。

- [~] `EC-M3-01` attestation model / canonical bytes — Rust model、strict timestamp、明示 clock、
  canonical bytes、signature encoding boundary、canonical base64url schema/parser parity、
  `sequence` の `1..=u64::MAX` schema/parser boundary、required string と optional `reason_digest` の
  non-blank schema/parser parity、`sequence >= 1` の Rust/selfhost source parity、retired/active
  trust-key rotation と active key の Rust/native preflight parity、verified signature receipt の
  Rust/native canonical handoff は verified partial slice。
  source/native producer、trust store、署名検証、両対応 target の runtime
  evidence は残る。ADR:
  `docs/adr/decisions-v0.3-review-attestation-sequence-boundary.md`、
  `docs/adr/decisions-v0.3-review-wire-schema-base64url.md`、
  `docs/adr/decisions-v0.3-review-wire-sequence-overflow.md`、
  `docs/adr/decisions-v0.3-review-wire-schema-nonblank.md`。

  (以下は旧 "## Next milestone — v0.3 Review provenance lifecycle" 節が持っていた
  同じ項目の記録。節が 2 つに分かれ `[~]` が二重計上されていたため、散文を落とさずここへ統合した。)
  attestation の canonical bytes、strict UTC timestamp、Ed25519 signature、
  canonical base64url schema/parser parity、`sequence` の `1..=u64::MAX` schema/parser boundary、
  required string と optional `reason_digest` の non-blank schema/parser parity、
  `trust_store` exact duplicate の `uniqueItems`/Rust parser parity、
  `trust_store` の retired/active key rotation と provider/algorithm ごとの active key 一意選択を
  Rust wire/native preflight で fail-closed にする parity、Rust の verified signature から
  attestation/trust-store digest と verification clock を束ねる receipt の Rust/native canonical
  handoff、明示 receipt を native `lsharp_validate` へ渡して
  `review_verifications[].receipt` へ exact projection する handoffも verified partial とした。
  current subject/source/provenance binding と explicit report、および
  `reviews[].verification_state` manifest projection を Rust canonical model で検証する。
  attestation input wiring、selfhost/native parity、両 supported target の artifact/runtime evidence
  を閉じる。
- [~] `EC-M3-02` lifecycle transition — append-only registry と stale/revoked 境界の Rust verified
  slice。selfhost reducerにも deterministic ordering、transition、sequence rollback、`effective_at`
  rollback（code `8` と前後 timestamp payload）、explicit clock 以下の最新 `event_at` 選択を接続し、
  Rust `review_lifecycle` 6件・clock gate 1件と selfhost lifecycle E2E 2件で parityを確認した。
  source/native report parity、provider snapshot、release evidence は残る。

  (以下は旧 "## Next milestone — v0.3 Review provenance lifecycle" 節が持っていた
  同じ項目の記録。節が 2 つに分かれ `[~]` が二重計上されていたため、散文を落とさずここへ統合した。)
  append-only lifecycle を deterministic に reduce し、active sequence、superseded、
  revoked、stale を report の事実へ接続する。Rust/selfhost reducerの sequence/transition/
  `effective_at` rollback（selfhost code `8`）と explicit clock `event_at` 選択は verified partial
  sliceだが、provider snapshot の取得、report projection、native parity は残る。
- [~] `EC-M3-03` CLI/MCP explicit inputs — explicit context、clock、trust/lifecycle input の Rust
  CLI/MCP boundary は verified partial slice。MCP input schema の subject/source/now/artifact all-or-none
  も `dependentRequired` で runtime boundaryへ接続した。native MCP subset の `review_now` canonical UTC
  lexical schema と実行前 reject も verified partial として追加した。selfhost/native MCP と target artifact
  parity は残る。ADR: `docs/adr/decisions-v0.3-native-mcp-review-clock-schema.md`。

  (以下は旧 "## Next milestone — v0.3 Review provenance lifecycle" 節が持っていた
  同じ項目の記録。節が 2 つに分かれ `[~]` が二重計上されていたため、散文を落とさずここへ統合した。)
  CLI/MCP の trust store/lifecycle explicit input と project-root boundary を維持し、
  attestation verification state と no-report/no-manifest の失敗境界を CLI/MCP の共通 projectionへ接続した。
  `--review-subject-digest` / `--review-source-commit` / `--review-now` の all-or-none context、
  expiry/binding の Rust CLI/MCP fixture、malformed clock の no-report contract も verified partial
  とした。Rust verified receipt の明示 file input、native command handoff、欠落/不一致 report
  projection の fail-closed も native MCP focused suiteで verified partial とした。selfhost/native
  full parity は残る。ADR:
  `docs/adr/decisions-v0.3-review-explicit-context.md`。
- [~] `EC-M3-04` source / selfhost / native producer parity — `:review` 互換を維持した named-field
  `:review-attestation` の Rust parser/source adapter、selfhost kind 20、`unverified` state、span、
  canonical bytes parity、invalid algorithm/signature/timestamp/time-window の fail-closed contract を verified。native
  source-file smoke、Evidence/manifest projection、current-source と packaged stage0 の provenance は残る。
  2026-07-31 に `tests/fixtures/validation/ec-m3-review-attestation-source.ls` を canonical fixture とし、
  Rust parser/Rust-host selfhost/Mac smoke/Linux stage0 wrapperの同一入力と Linux fake provenance harness の
  fixture copy 契約を追加した。Linux current-source runtime、Mac/Linux matrix、packaged provenance は残る。
  同日、current `f6a6da30` の Mac Apple Silicon stage0 producer/package と actual App.Cli E2E、source-file
  smoke を実行し、Mac current-source runtime の named-field/attestation/validation smoke を pass させた。
  既定 macOS `TMPDIR` の task-owned producer artifact path拒否も再現し、`TMPDIR_ROOT/lsharp-*` のみを
  許可する safe cleanup boundaryを追加した。末尾 `/` を含む既定 `TMPDIR` の `T//lsharp-*` 連結も
  stage0 wrapperで正規化し、actual producer/package（484.89秒）と生成 stage0の source-file smokeを通過した。
  さらに current `3e1b2690` の Mac Apple Silicon producer/package（actual `App.Cli` E2E 510.15秒）と
  `aarch64-apple-darwin` stage0 manifest/source-file evidence smokeを実行し、source commit/target、
  named-field attestation、期限付き/期限なし canonical bytes、`unverified`、directive span、invalid
  source error code `8`、stdout/stderr、exit code、Wasm digest/sizeの一致を確認した。Linux x86_64
  current-source runtime、fetch後の packaged provenance と両 target matrixは残る。
  source-file evidence writer は stage0 manifest の `source_commit` を小文字40桁 hexadecimal に限定し、
  uppercase input を証跡作成前に拒否する contract test を追加した。これは provenance input boundary の
  verified partial sliceであり、Linux current-source runtime、packaged provenance、両 target matrixは残る。
  同じ writer は work directory 内の symlink も staging 前に拒否し、外部 path を参照する証跡を保存しない
  contract test を追加した。これは task-owned evidence の安全境界に限る verified partial sliceであり、Linux
  current-source runtime、packaged provenance、両 target matrixは残る。

  (以下は旧 "## Next milestone — v0.3 Review provenance lifecycle" 節が持っていた
  同じ項目の記録。節が 2 つに分かれ `[~]` が二重計上されていたため、散文を落とさずここへ統合した。)
  source と selfhost/native producer の attestation named-field、canonical bytes、
  state、span、exit code の Rust/selfhost 同一 fixture parity は verified partial。JSON report の
  field order、nullable `expires_at`、canonical bytes、span と native source-file smoke の
  report/manifest fixture contract を追加検証した。Mac current-source stage0 producer/package/source-file
  smoke は実行済みで、native MCP receipt の report→manifest review projectionも verified partial とした。
  native source-file evidence writer が explicit `validation-attestation-json.stdout` の
  `review_attestations` projectionを evidence manifestへ保存し、report欠落時に証跡を作らない
  handoffも verified partial とした。
  official two-target orchestrator の explicit `review-attestation-report` を Mac/Lima の各 source-file
  smokeへ一度だけ伝播し、target別 evidence manifestの `review_attestations` と exact compareする
  postflight、report-free routeの no-implicit projection、report欠落・片側不一致の pre/postflight
  fail-closed を fake two-target harnessで verified partial とした。
  Linux current-source stage0、packaged artifact provenance、Mac/Linux runtime parityを
  `v0.3-milestone-01.md` の M3-04-N1 で閉じる。
- [~] `EC-M3-05` release / evidence gate — Rust CLI/MCP と manifest の入出力 roundtrip が、明示した
  subject/source/artifact/clock と trust-store/lifecycle component digest を `review_evidence_identity`
  として deterministic JSON/text/MCP/manifest へ投影し、競合を fail-closed に拒否する verified partial
  slice。native source-file smoke の explicit identity JSON/text/manifest、nullable digest、all-or-none
  input boundaryも verified partial として追加した。native MCP subset の provider/output/input と
  manifest nested schema parity、canonical `review_now` lexical preflight、explicit identity の
  report/manifest postflight一致検証、identity optionsなし source/file routeの no-implicit boundaryも
  verified partial として追加した。
  selfhost `App.Cli` の explicit identity
  JSON/text/manifest parity も Rust-host actual Wasm で verified partial として追加した。offline
  release identity verifier を native-only
  archive / packaged stage0 の optional identity projection と release smoke の再検証へ接続したが、
  明示 artifact/trust-store/lifecycle bytes を canonical identity へ変換する offline input preparer の
  verified partial sliceを追加した。preparer/verifier の `now` は Rust canonical timestamp と同じ閏年・暦日・時分秒範囲を
  共有し、lexical shapeだけでは通る不正日時を fail-closed にする。verifier は明示 snapshot の raw bytes を任意に再計算し、trust-store/lifecycle digest mismatch と片側指定を fail-closed にする。
  2026-08-01 に `--manifest` の JSON root shape も検証し、配列・null・数値を traceback ではなく
  return code `1` の `review_evidence_identity` 入力エラーへ変換する contract test を追加した。これは
  manifest runtime validation の verified partial sliceであり、native MCP の
  LSP/package API/provider semantics、current-source target runtime、
  packaged bytes parity、manifest runtime validation の残りは未完了である。
  同日、native MCP `lsharp_validate` の inline `manifest` / `manifest_file` についても、malformed JSON と
  配列・null・数値 root を native 実行前に `JSON object` 入力エラーへ変換する preflight を追加した。
  54件の native MCP suite と fake native no-execution contract で確認した verified partial sliceである。
  ADR: `docs/adr/decisions-v0.3-native-mcp-manifest-input-root.md`。
  さらに native MCP が出力する manifest について、malformed JSON、非 object root、schema version不一致、
  required field欠落、未知 top-level field、nodes/evidence/edges の非配列を wrapper 側で fail-closed にした。
  native MCP 55件で traceback なしの出力境界を確認した verified partial sliceである。
  ADR: `docs/adr/decisions-v0.3-native-mcp-manifest-output-root.md`。
  さらに native `lsharp_validate` report の root、required fields、status、count、collection、optional field
  境界を wrapper 側で検証し、array/null/malformed/missing/unknown/invalid status/boolean count を traceback
  なしで拒否した。native MCP 56件で確認した verified partial sliceである。
  ADR: `docs/adr/decisions-v0.3-native-mcp-validate-report-root.md`。
  `lsharp_check` の native output についても root、required fields、boolean `ok`、diagnostics 配列を
  wrapper 側で fail-closed にし、array/null/malformed/missing/unknown/type mismatch を traceback なしで拒否した。
  native MCP 57件で確認した verified partial sliceである。
  ADR: `docs/adr/decisions-v0.3-native-mcp-check-output-root.md`。
  2026-07-31 に native source-file smoke の identity options なし JSON/manifest が
  `review_evidence_identity` を暗黙生成しない no-implicit boundary を追加した。実 native MCP、provider、
  current-source/packaged runtime は残る。
  positional version 1 manifest input の既存 `review_evidence_identity` についても、同値 caller
  identity の再 attach と conflicting identity の `source validation error:14` / exit `1` / no-report・
  no-manifest fail-closed を `ManifestInput` / `App.Cli`、Rust-host actual Wasm、Linux fake provenance
  harness で verified partial とした。Linux current-source native stage0、native MCP、provider、両 target
  runtime は残る。
  さらに Mac current-source stage0 packageを実際に source-file smokeへ渡し、identity source fixtureの
  trace (`motivates` / `tested-by`) と explicit identity JSON/text/manifest、同値再 attach、conflict
  fail-closed を actual runtimeで確認した。Linux runtime、native MCP/provider、fetch後の packaged parityは残る。
  native-only `scripts/release.sh` から snapshot path を verifier へ伝播し、packaging 前の digest mismatch を拒否する wiring も verified partial として追加した。
  stage0 package は `--review-trust-store` / `--review-lifecycle` を同じ verifier へ渡し、release smoke は
  `RELEASE_REVIEW_TRUST_STORE` / `RELEASE_REVIEW_LIFECYCLE` から native-only archive の identity を再検証する
  offline propagation まで verified partial として追加した。さらに
  `native-official-release-local.sh` が同じ snapshot を両 target の release、stage0 package、Mac smoke、
  Linux VM smoke へ伝播する offline orchestrator wiring も verified partial として追加した。さらに
  stage0 fetch 後の directory を Mac host / Linux Lima の既存 source-file runtime smoke へ渡す wiringを
  fake two-target harness で固定した。provider helperの実取得、selfhost/native MCP parity、
  current-source と packaged stage0 の provenance、Mac Apple Silicon / Linux x86_64 の実 release
  artifact/runtime gate は残る。さらに Mac `App.Cli` producer、transport、materializer、stage0 package
  builderを接続する current-source producer wrapper と既存 output の fail-closed を fake harnessで
  固定した。実 Mac/Linux runtime、provider取得、bytes parity は残る。ADR:
  `docs/adr/decisions-v0.3-release-identity-gate.md`,
  `docs/adr/decisions-v0.3-provider-input-identity-preparer.md`,
  `docs/adr/decisions-v0.3-provider-snapshot-digest-verification.md`,
  `docs/adr/decisions-v0.3-native-release-snapshot-wiring.md`,
  `docs/adr/decisions-v0.3-stage0-release-smoke-snapshot-wiring.md`,
  `docs/adr/decisions-v0.3-native-official-multitarget-snapshot-wiring.md`,
  `docs/adr/decisions-v0.3-native-official-stage0-runtime-smoke.md`,
  `docs/adr/decisions-v0.3-native-macos-stage0-producer.md`。さらに
  `NATIVE_SELFHOST_SOURCE_SMOKE_EVIDENCE_DIR` による source-file smoke の一時 work directory、
  stdout/stderr、Wasm bytes、exit code の保存と digest/size manifest を contract test で固定した。
  これは実 target gate の operator evidence を保持する verified partial sliceであり、実 Mac/Linux
  runtime、provider取得、packaged bytes parity は残る。ADR:
  `docs/adr/decisions-v0.3-native-source-smoke-evidence.md`。さらに official stage0 runtime gateへ
  `NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT` を接続し、Mac hostと Linux/Limaの target別 source-file
  smoke evidenceを別 leafへ保持する wiringを追加した。Linux側は evidence writerをVMへコピーし、source
  smoke終了後に stdout/stderr、Wasm digest/size、exit code、stage0 manifestをhostへ再帰コピーする。
  fake `limactl` の成功/失敗 copy-out contractで、失敗時の元 exit code保持も確認した。
  official gateだけでなく直接の Linux source-file smoke入口も hostgen replay lockを
  `limactl` 前に検査し、live ownerでは exit `90`、owner metadata出力、外部呼び出しなしで停止する
  fake contractを追加した。これは operator safetyの verified partial sliceであり、Linux runtimeや
  packaged provenanceの completion evidenceではない。
  2026-07-31 に fresh current `e44ca727` の Mac Apple Silicon stage0 producer（actual App.Cli E2E
  491.75秒）→stage0 archive→local HTTP `fetch-stage0.sh`→fetched package source-file smokeを完走し、
  source commit/target、stage0 payload、`compile.wasm` / `build.wasm` digest/size、App.Cli
  `--version` / `--help` の exit/stderrを確認した。これは証跡伝播と Mac current-source/fetch runtime の
  verified partial sliceであり、実 Linux runtime、provider取得、両 target packaged bytes/rollback parityは残る。ADR:
  `docs/adr/decisions-v0.3-native-official-stage0-runtime-smoke.md`。2026-08-01 に current
  `origin/main` `be17567f35f2f688a51652efc0bb6ba31ed12582` の Mac Apple Silicon stage0 producerを
  actual App.Cli E2E（489.04秒）で完走させ、manifest digest
  `574f4fedbf3a0bd7d034c9443b99fa2137eed3199ad06320f581c583606a4f48`（259 bytes）、source commit/target/payloadを確認した。
  relative `--output-dir` の stage0 release archive生成失敗を RED→GREEN の regression testで閉じ、
  package→local file `fetch-stage0.sh` checksum/provenance→fetched source-file smokeを通過させた。
  producer/fetched smokeとも exit `0`、`compile.wasm` / `build.wasm` は digest
  `afd1638e444a7e8c371dc1d17550479fcc5e4efbbb9e9dbdffa8551933d71a00`（2,559 bytes）で一致した。
  これは Mac current-source/fetch runtimeの verified partial sliceであり、Linux runtime、provider取得、両 target packaged bytes/rollback parityは残る。
  さらに native manifest の
  node/evidence/subject ID fields を一つの serializer helper へ統合し、EC-M3-01 canonical
  manifest bytes の working-tree preflight parity を確認した。これは serializer の verified
  sliceであり、fresh producer と両 target runtime の証拠は残る。ADR:
  `docs/adr/decisions-v0.3-ec-m3-native-id-serializer.md`。さらに native parser/CLI の
  review-attestation span、missing/extra metadata の malformed boundary、evidence registry
  precedence、positional manifest input、manifest write failure、stderr/exit propagation を
  Rust contract に揃え、Mac source-file smoke の全 negative/manifest/report cases を blocked
  tool preflight で通過した。これは working-tree source と旧 stage0 を使った verified preflight
  であり、fresh current-source producer、packaged bytes、Linux x86_64 runtime、provider/rollback
  evidence は残る。ADR:
  `docs/adr/decisions-v0.3-native-validation-boundary-followups.md`。

  2026-08-02 に Linux source-file smoke の stage0 input preflight を、非 symlink の regular directory かつ
  配下に symlink がない provenance-safe inputへ限定した。root symlink と nested symlink を fake Lima harness
  で RED→GREEN にし、拒否前の `limactl` 未呼び出しも確認した。これは stage0 input safety の verified
  partial sliceであり、current-source Linux runtime、fetch後の packaged provenance、provider/auth、両 target
  runtime は残る。ADR: `docs/adr/decisions-v0.3-native-linux-stage0-source-directory-provenance.md`。

  2026-08-02 に `fetch-stage0.sh` の archive preflight を directory または regular file のみ受理する
  fail-closed boundaryへ更新した。unknown tar entry の RED→GREEN と valid local package の fetch/install を
  focused harness で確認した。current source `e7c7e864` と一致する Linux stage0 artifact がなく、実 Linux
  runtime replay は未実行であるため、current-source runtime、packaged runtime、provider/auth、rollback、
  両 target parity は残る。再現 command は `git rev-parse --verify HEAD` と
  `find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 -type f -name manifest.json -path '*lsharp*'`。
  ADR: `docs/adr/decisions-v0.3-stage0-fetch-archive-entry-provenance.md`。

  同日、`STAGE0_RELEASE_BASE_URL` の provider URL preflight を追加し、HTTPS host または local `file://`
  のみを許可し、HTTP・埋め込み credentials・query/fragment を `curl` 前に明示拒否した。
  fake `curl` が呼ばれない RED→GREEN と、既存 local fetch/install の継続を focused harness で確認した。
  これは provider URL input の verified partial sliceであり、live provider API/auth、current-source Linux
  runtime、packaged target parity、rollback parity は残る。M3-05-N9 は `[~]` のまま維持する。ADR:
  `docs/adr/decisions-v0.3-stage0-fetch-provider-url-boundary.md`。

  同日、既存 atomic install harness に final install move failure と rollback restore move failure の二重失敗を
  追加した。restore move が失敗しても旧 stage0 を copy-recover し、成功後に hidden backup を回収する
  RED→GREEN を確認した。これは local rollback restore failure の verified partial sliceであり、実 I/O 障害下の
  recovery、rollback archive parity、live provider/auth、current-source Linux runtime、packaged target parity は
  残るため、M3-05-N9 は `[~]` のまま維持する。ADR:
  `docs/adr/decisions-v0.3-stage0-fetch-rollback-restore-failure.md`。

  同日、rollback compatibility archive の `manifest.json` にある `entry_binary` / `lsp_binary` / `component` を
  release smoke が消費する固定 payload 名と照合する boundaryを追加した。checksum、rollback anchor、target /
  version / source commit が正しくても payload 宣言が異なる archive を拒否する RED→GREEN を provider snapshot
  harness で確認した verified partial sliceである。live provider/auth、current-source Linux runtime、両 target
  の packaged provenance/rollback bytes parity は未検証のため、M3-04-N1 / M3-05-N2 は `[~]` のまま残る。ADR:
  `docs/adr/decisions-v0.3-rollback-archive-manifest-payload-parity.md`。

  続けて同日、native-only primary manifest の `rollback_anchor.kind` を supplied rollback archive の
  `archive_kind` と照合する binding boundary を追加した。primary checksum、asset/SHA、nested rollback identity
  が正しくても kind が異なる archive を拒否する RED→GREEN を provider snapshot harness で確認した verified
  partial sliceである。live provider/auth、current-source Linux runtime、両 target の packaged provenance/
  rollback bytes parity は未検証のため、M3-04-N1 / M3-05-N2 は `[~]` のまま残る。current-source blocker の
  再現 command は `git rev-parse --verify HEAD` と `find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f
  -name manifest.json -path '*lsharp*'`。ADR:
  `docs/adr/decisions-v0.3-rollback-anchor-kind-parity.md`。

  続けて同日、provider/auth snapshot の入力 path を regular non-symlink file に限定した。実体と同じ bytes の
  trust-store / lifecycle symlink を verifier が辿らず拒否する RED→GREEN を identity harness で固定した
  verified partial sliceである。live provider/auth acquisition、current-source Linux runtime、両 target の
  packaged provenance/rollback bytes parity は未検証のため、M3-05-N2 / M3-05-N7 は `[~]` のまま残る。ADR:
  `docs/adr/decisions-v0.3-provider-snapshot-regular-file.md`。

  同日、provider snapshot が指定された official gate の App.Cli / stage0 identity 4入力について、JSON objectの
  `source_commit` を current checkout HEAD に binding した。不正 JSON、非 object、欠落または別 source commitを
  packaging 前に拒否し、fake two-target harnessで invocation log 不変を確認した verified partial sliceである。
  provider API/auth acquisition、current-source Linux runtime、両 targetの packaged provenance/rollback bytes
  parity は未検証のため、M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残る。ADR:
  `docs/adr/decisions-v0.3-native-official-provider-source-commit-binding.md`。

  続けて、provider snapshot 指定時の4つの identityを既存 `verify-native-release-identity.py` へ接続し、
  canonical schema、provider digest、strict timestamp、source commitを packaging 前に検査した。current
  `source_commit` だけの不完全 JSONを拒否し、fake two-target harnessで invocation log 不変を確認した
  verified partial sliceである。provider API/auth acquisition、current-source Linux runtime、両 targetの
  packaged provenance/rollback bytes parity は未検証のため、M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残る。ADR:
  `docs/adr/decisions-v0.3-native-official-provider-identity-schema-preflight.md`。

  続けて、stage0 directoryに `review-evidence-identity.json` が埋め込まれている場合の release package入力を
  explicit identity、trust-store、review-lifecycle snapshotの3点セットへ限定した。provider snapshotなしで
  埋込み identityを包装する REDを追加し、packaging前の fail-closedとarchive不生成を direct release-package
  harnessで確認した verified partial sliceである。live provider/auth acquisition、current-source Linux runtime、
  両 target packaged provenance/rollback bytes parityは未検証のため、M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残る。
  ADR: `docs/adr/decisions-v0.3-native-stage0-release-embedded-identity-provider-preflight.md`。

  同日の最終監査で current `origin/main` / `HEAD` `c20ef6b0a24a6c02de1d504d20e072aebbed6a80` に一致する
  Linux stage0 manifestと expected replay lockは見つからなかった。別セッション所有の Lima hostagentを変更せず、
  Linux runtime replay / stage regenerationは未実行である。再現 commandは
  `current_head="$(git rev-parse --verify HEAD)"; find /tmp /Users/biwakonbu/github/tmp -type f -name manifest.json -path '*lsharp*'`
  と `find /tmp -maxdepth 3 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)`。current-source
  Linux runtime、provider/auth acquisition、両 target packaged provenance/rollback bytes parityは残件のため、
  M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま維持する。

  続けて、offline identity producer / verifier が空の trust-store / review-lifecycle snapshotを
  provider provenanceとしてdigest化できる境界を、provider field限定の `must be non-empty` preflightへ更新した。
  producer output未生成と verifier fail-closedを RED→GREEN の focused testで確認した verified partial sliceである。
  live provider/auth acquisition、署名意味検証、current-source Linux runtime、両 target packaged provenance/
  rollback bytes parityは未検証のため、M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残る。ADR:
  `docs/adr/decisions-v0.3-provider-snapshot-nonempty-preflight.md`。

  続けて、trust-store と review-lifecycle に同じ lexical-normalized pathを渡す provider adapterの誤配線を
  `must be different files` として producer / verifierの両方で拒否した。same-path RED→GREENと producer output
  未生成を確認した verified partial sliceである。live provider/auth acquisition、署名意味検証、current-source Linux
  runtime、両 target packaged provenance/rollback bytes parityは未検証のため、M3-05-N2 / M3-05-N7 / M3-05-N9 は
  `[~]` のまま残る。ADR: `docs/adr/decisions-v0.3-provider-snapshot-role-binding.md`。

  続けて、native MCP が明示 provider snapshotを raw bytes の digestへ変換するだけで署名・lifecycleの
  semantic verificationを実行しない境界を固定した。snapshot使用時に native report の `verified` /
  `stale` / `revoked` stateを受理せず、`unverified` 以外を `provider semantic verification is unavailable`
  で fail-closed にする RED→GREEN と focused suiteを確認した verified partial sliceである。live provider/auth
  acquisition、実 semantic verifier、current-source Linux runtime、両 target packaged provenance/rollback bytes
  parityは未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残る。ADR:
  `docs/adr/decisions-v0.3-native-mcp-provider-semantic-boundary.md`。

  続けて同日、release smoke の `checksums.txt` target path を archive root 内の POSIX relative path に限定した。
  checksum-valid な `../../../outside-checksum-target.txt` fixture を拒否する RED→GREEN を provider snapshot
  harness で確認し、package 外部 file を checksum evidence として受理しない verified partial slice を追加した。
  live provider/auth acquisition、current-source Linux runtime、両 target の packaged provenance/rollback bytes parity
  は未検証のため、M3-05-N2 / M3-05-N7 は `[~]` のまま残る。current-source blocker の再現 command は
  `git rev-parse --verify HEAD` と `find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json
  -path '*lsharp*'`。ADR: `docs/adr/decisions-v0.3-release-smoke-checksum-path.md`。

  続けて同日、rollback archive の `checksums.txt` に smoke-critical payload 全ての checksum entry を必須化した。
  `lsharp` entry だけを除いた checksum-valid rollback fixture を拒否する RED→GREEN を provider snapshot harness
  で確認した verified partial sliceである。live provider/auth acquisition、current-source Linux runtime、両 target
  の packaged provenance/rollback bytes parity は未検証のため、M3-05-N2 / M3-05-N7 は `[~]` のまま残る。current-source
  blocker の再現 command は `git rev-parse --verify HEAD` と `find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f
  -name manifest.json -path '*lsharp*'`。ADR: `docs/adr/decisions-v0.3-rollback-checksum-coverage.md`。

  (以下は旧 "## Next milestone — v0.3 Review provenance lifecycle" 節が持っていた
  同じ項目の記録。節が 2 つに分かれ `[~]` が二重計上されていたため、散文を落とさずここへ統合した。)
  keyset/lifecycle/source/artifact digest の Rust CLI/MCP/manifest と selfhost identity
  projection、nullable field order、conflict rejection は verified partial。offline release identity
  verifier、native-only archive / packaged stage0 の optional projection、artifact/source mismatch の
  release smoke rejectionを追加した。native text/JSON/MCP と release gate の
  `verified/unverified/stale/revoked/invalid` ordering、provider adapter、両 target runtimeを
  M3-05-N1/N2 で閉じる。native MCPの明示 receipt report→manifest exact projectionと
  欠落/不一致 postflight rejectも verified partial とした。official gate の task-owned cleanup path traversal (`.` / `..`) 拒否は
  verified partial として追加したが、actual provider/auth、current-source/packaged runtime、
  rollback/Wasm parity は残る。ADR:
  `docs/adr/decisions-v0.3-native-official-cleanup-path.md`。
  さらに offline release identity verifier の明示 `--artifact` 入力を regular fileかつ非symlinkに限定し、
  期待 digestと同じ bytesを持つ外部 artifact symlinkを `artifact must be a regular non-symlink file` で
  fail-closedにする RED→GREENを `test-native-release-identity.py` で確認した。これは artifact path provenance
  の verified partial sliceであり、provider API/auth取得・意味検証、current-source Linux runtime、Mac/Linux
  両 targetの packaged provenance/rollback bytes parityは未検証であるため、EC-M3-05と関連 milestoneは
  `[~]` のまま残す。ADR: `docs/adr/decisions-v0.3-release-identity-artifact-regular-file.md`。
  さらに非native packaged archiveの `lsharp-lsp --version` 出力を caller-provided `VERSION` と比較し、
  checksum-valid な version mismatch fixtureを `packaged LSP version mismatch` で fail-closed にする
  RED→GREENを `test-release-smoke-provider-snapshots.sh` で確認した。これは packaged LSP version output parity
  の verified partial sliceであり、live provider/auth取得・意味検証、current-source Linux runtime、Mac/Linux両
  targetの packaged provenance/rollback bytes parityは未検証のため、EC-M3-05と関連 milestoneは `[~]` のまま残す。
  ADR: `docs/adr/decisions-v0.3-packaged-lsp-version-output-parity.md`。
  さらに offline release identity verifier に caller-provided `--verification-now` UTC clock を追加し、identity の
  `now` が検証時計より未来の場合を `identity now is after verification now` で fail-closed にする
  RED→GREENを `test-native-release-identity.py` で確認した。これは provider identity caller-clock freshness の
  verified partial sliceであり、live provider/auth取得・署名意味検証、current-source Linux runtime、Mac/Linux両 targetの
  packaged provenance/rollback bytes parityは未検証のため、EC-M3-05と関連 milestoneは `[~]` のまま残す。
  ADR: `docs/adr/decisions-v0.3-provider-identity-verification-clock.md`。
  さらに native-only packaged App.Cli の `--help` を stdout/stderr 別々に収集し、成功・`Usage: lsharp` の stdout・空の
  stderr を要求する boundaryを追加した。checksum-valid な fake packageが help warning を stderrへ漏らしても受理する
  REDを、`native-only App.Cli help must keep stderr empty` で fail-closed にする GREENへ更新し、valid native/rollback fixture
  の継続成功を `test-release-smoke-provider-snapshots.sh` で確認した。これは packaged help output の verified partial sliceであり、
  native/LSP version、archive/rollback manifest・checksum、provider snapshot、live provider/auth取得・意味検証、current-source
  Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、EC-M3-05と関連 milestoneは
  `[~]` のまま残す。ADR: `docs/adr/decisions-v0.3-packaged-native-help-output.md`。
  さらに official release gate の provider input preflight に明示 `NATIVE_OFFICIAL_REVIEW_VERIFICATION_NOW` を接続し、4つの
  target/stage0 identityへ caller clockを渡すようにした。App.Cli artifact identityには sibling `program.native` の bytesも渡し、
  future identity `now` または artifact digest mismatch を release/package/fetch/smoke/Lima開始前に fail-closed とする
  RED→GREENを `test-native-official-release-snapshots.sh` で確認した。これは provider snapshot input→identity freshness→source/artifact
  binding を一つの official gate boundaryへ集約する verified partial sliceであり、live provider/auth取得・署名意味検証、current-source
  Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、EC-M3-05と関連 milestoneは `[~]` のまま残す。
  ADR: `docs/adr/decisions-v0.3-native-official-provider-freshness-binding.md`。

2026-08-01 の current `origin/main` `1cdbe555f63c909fbfb3940c8462cf4b08ba442d` では、Mac/Linux
App.Cli producer、Linux hostgen fixed point、protocol stage0 compiler、provider snapshot identity、
rollback archive、stage0 package/fetch、official release smoke を同一 source commit で確認した。
Mac producer/fetched source-file smoke は exit `0` で Wasm bytes が一致し、Linux current-source
source-file smoke は別タスクの live hostgen replay lock により exit `90` で待機中である。native
MCP と provider API/auth adapter、および Linux runtime は未完了のため `[~]` を維持する。ADR:
`docs/adr/decisions-v0.3-current-source-two-target-runtime.md`。

同日、native selfhost `mcp-server` を JSON-RPC shim経由の `lsharp_check` / `lsharp_validate` /
`lsharp_format` deterministic subsetへ接続した。native program以外の fallbackは呼ばず、provider
snapshot pathは明示した2つのファイルを offline で raw bytes の SHA-256 digestへ変換して native
validateへ渡す。片側指定、欠損、非 regular、空、明示 digestとの不一致は native 実行前に fail-closed
とし、ネットワーク・provider helper・署名/lifecycle意味検証は呼ばない。focused runner/protocol
tools/list の JSON Schema に provider path と review identity 4-field の all-or-none
`dependentRequired` を公開し、`additionalProperties: false` と shim自身の unknown-field拒否も
native 実行前に揃えた。focused
testsは通過したが、Rust MCP 全tool parity、provider取得/auth、署名/lifecycle検証、Linux runtimeは
未完了のため `[~]` を維持する。`lsharp_check` の migrationDiagnostics output schema（code/owner/
semantics/disposition/range）と `lsharp_validate` report の trace gap/review identity/
review verification schemaも Rust contractへ揃え、manifest top-level（version/nodes/evidence/edges）
も明示した。ADR:
`docs/adr/decisions-v0.3-native-mcp-subset-shim.md`,
`docs/adr/decisions-v0.3-native-mcp-provider-snapshot-adapter.md`,
`docs/adr/decisions-v0.3-native-mcp-review-identity-contract.md`,
`docs/adr/decisions-v0.3-native-mcp-validate-input-closed-world.md`,
`docs/adr/decisions-v0.3-native-mcp-check-output-schema.md`,
`docs/adr/decisions-v0.3-native-mcp-validate-output-schema.md`,
`docs/adr/decisions-v0.3-native-mcp-manifest-output-schema.md`。

2026-08-01 に native MCP の lsharp_errors、lsharp_search、lsharp_project_context、lsharp_package_api、lsharp_stdlib_api を追加した。canonical
crates/lsharp-driver/src/error_codes.rs の read-only table projection として LS####、
E0001-E0005（E0003 は LS1002 へ統合）、未知コード、docs link を返し、Rust/native schema の
必須・空文字拒否・additionalProperties: false を揃えた。fake native program を実行しないことを
native MCP 17 tests と Rust schema/error focused tests で確認した。lsharp_search は `.lsharp/packages`
下の `lsharp.toml` を読む offline installed-package projection として、query、決定的順序、closed-world
schema、fake native program 非実行を native MCP 19 tests と Rust MCP focused 56 tests で確認した。
lsharp_project_context は `lsharp.toml` の project/exports/dependencies と `.lsharp/packages` を offline
projection し、依存名の決定的順序、closed-world schema、引数 fail-closed、fake native program 非実行を
native MCP 21 tests と Rust MCP focused 59 tests で確認した。lsharp_package_api は deterministic な
installed package の既存 `docs/api.json` projection に加え、artifact が無い場合の sorted `src/**/*.ls`
ごとの native `doc --json` in-memory projection を追加した。full closed-world schema、nested malformed
field と malformed native doc の fail-closed、引数 fail-closed、既存 artifact route の fake native program
非実行、生成 route の `docs/api.json` 非作成を native MCP 47 tests と native LSP relay 5 tests、Rust `mcp_server::tests` 87 tests
（package API filter 6 tests、driver unit 214 tests）で確認した。lsharp_stdlib_api は Rust canonical `doc --json` から生成した
`.lsharp/packages` の file/dangling symlink は package として列挙せず、directory symlink は path dependency 互換のため保持する Rust/native
discovery contract も追加した。ADR: `docs/adr/decisions-v0.3-native-mcp-package-entry-boundary.md`。
`stdlib/api.json` を読む offline projectionに加え、artifact が無い場合の sorted direct `stdlib/*.ls`
ごとの native `doc --json` in-memory projection を追加した。module 絞り込み、閉世界 schema、引数
fail-closed、artifact と Rust canonical output の一致、malformed native doc の fail-closed、artifact
非作成を確認した。native `lsharp_hover` は source/file、line/character と `col` alias を native
`lsp --stdio` へ渡す scalar signature/doc projection として、malformed frame・child failure・missing
response の fail-closed を確認した verified partial である。
続いて native `lsharp_definition` の source/file/position relay、単一 location の closed `{start,end}`
projection、invalid range/ambiguous location の fail-closed を native MCP 37 tests で確認した verified partial である。
さらに native `lsharp_references` の `includeDeclaration: true` relay、複数 location の closed `{count,ranges}`
projection、native `null` の空集合化、invalid range の fail-closed を native MCP 42 tests で確認した verified partial である。
さらに native `lsharp_completion` の array/CompletionList relay、numeric completion kind の Rust 名称投影、
native `null` の空候補化、invalid item の fail-closed を native MCP 47 tests で確認した verified partial である。
さらに `lsharp_compile_run` の source/file route、native `compile` と外部 `wasmtime` の明示境界、
compile/runtime failure・欠落 artifact・runtime 未設定の fail-closed、stdout の分離、task-owned temporary
source/Wasm の cleanup を native MCP 52 tests で確認した。これは local external-runtime の verified partial
であり、supported-target stage0/packaged runtime parity ではない。package-install semantics、provider
semantics、target runtime、manifest runtime validation の全 surface は [~] のまま残す。ADR:
docs/adr/decisions-v0.3-native-mcp-subset-shim.md。
compile/run boundary の判断は
`docs/adr/decisions-v0.3-native-mcp-compile-run-boundary.md` に記録した。
さらに native `lsharp_format` の出力を `formatted` だけの closed-world schema に固定し、native `fmt`
が stdout を返しても non-zero exit なら stderr/status 付き MCP error とする fail-closed boundary を
native MCP 60 tests で確認した。ADR:
`docs/adr/decisions-v0.3-native-mcp-format-output.md`。これは source text の意味を解釈しない
offline relay の verified partial slice であり、target runtime と Rust/native full parity は残る。
さらに native MCP の `lsharp_hover` / `lsharp_definition` / `lsharp_references` /
`lsharp_completion` で `character` と `col` の oneOf schema を揃え、両方指定した曖昧な入力を
native LSP 実行前に拒否する contract を native MCP 62 tests で確認した。ADR:
`docs/adr/decisions-v0.3-native-mcp-lsp-position-alias.md`。単独 `col` alias は互換性のため維持し、
target runtime と Rust/native full parity は残る。
さらに native MCP `lsharp_check` / `lsharp_format` の `source`/`file` input schema と runtime
preflight を closed-world に揃え、未知引数・source/file 同時指定を native 実行前に拒否する contract を
native MCP 65 tests で確認した。ADR:
`docs/adr/decisions-v0.3-native-mcp-check-format-input.md`。target runtime と Rust/native full parity は残る。
さらに共通 `source` input schema に `minLength: 1` を設定し、native LSP 4 tools、`check`、`validate`、
`format`、`compile_run` の `tools/list` と runtime preflight の非空契約を揃えた。空文字・空白のみの
`source` は native 実行前に拒否し、native MCP 68 tests と Python compile を通過した verified partial
である。ADR: `docs/adr/decisions-v0.3-native-mcp-source-input-schema.md`。target runtime、provider semantics、
Rust/native full parity は残る。
さらに native MCP `lsharp_check` の `migrationDiagnostics` item を postflight 検証し、required field、
code/semantics/disposition enum、range、非負 position、optional message の不正値を native 成功として
返さない contract を追加した。完全な item はそのまま通し、native MCP 69 tests と Python compile を
通過した verified partial である。ADR: `docs/adr/decisions-v0.3-native-mcp-check-output-items.md`。
target runtime、provider semantics、Rust/native full parity は残る。
さらに native MCP `lsharp_validate` の `trace_gaps` / `review_verifications` item を postflight 検証し、
required field、closed field、trace-gap code、review ID pattern、lifecycle state の不正値を native 成功
として返さない contract を追加した。完全な nested report はそのまま通し、native MCP 70 tests と
Python compile を通過した verified partial である。ADR: `docs/adr/decisions-v0.3-native-mcp-validate-output-items.md`。
identity、manifest runtime、target runtime、provider semantics、Rust/native full parity は残る。
さらに native MCP `lsharp_validate` の `review_evidence_identity` report object を postflight 検証し、
required field、unknown field、非 nullable string、nullable provider digest の型境界を揃えた。manifest
input 経由の malformed identity を native 成功として返さず、完全な identity は保持することを native
MCP 72 tests と Python compile で確認した verified partial である。ADR:
`docs/adr/decisions-v0.3-native-mcp-review-identity-output.md`。manifest nested runtime、target runtime、
provider semantics、Rust/native full parity は残る。
さらに native MCP emitted manifest の `edges` を全 relation variant（`motivates`、`constrained-by`、
`tested-by`、`supports`、`contradicts`、`evaluates`、`invalidates`）ごとに postflight 検証し、relation
固有 field、ID、subject kind、unknown field の境界を揃えた。valid edge manifest は保持し、malformed edge
は native 成功として返さないことを native MCP 74 tests と Python compile で確認した verified partial
である。ADR: `docs/adr/decisions-v0.3-native-mcp-manifest-edges.md`。evidence nested、referential integrity、
target runtime、provider semantics、Rust/native full parity は残る。
さらに native MCP emitted manifest の `nodes` / `reviews` item を postflight 検証し、closed field、
required identifier、kind/visibility/state enum、non-empty text、span offset、provenance digest の境界を
揃えた。valid node/review manifest は保持し、malformed item は native 成功として返さないことを native
MCP 73 tests と Python compile で確認した verified partial である。ADR:
`docs/adr/decisions-v0.3-native-mcp-manifest-items.md`。evidence/edges nested runtime、target runtime、
provider semantics、Rust/native full parity は残る。

さらに native MCP emitted manifest の `evidence` item を postflight 検証し、closed field、required
identifier、subject kind、method/outcome/independence enum、execution/sampling、provenance の境界を揃えた。
valid evidence record は保持し、malformed record は native 成功として返さないことを native MCP 75 tests
と Python compile で確認した verified partial である。ADR:
`docs/adr/decisions-v0.3-native-mcp-manifest-evidence.md`。referential integrity、target runtime、provider
semantics、Rust/native full parity は残る。

さらに native MCP emitted manifest の referential integrity を postflight 検証し、node/evidence/explicit
review の duplicate ID、evidence subject、全 edge relation の graph-owned endpoint、node kind mismatch を
fail-closed にした。explicit `reviews` registry がある場合だけ review endpoint を閉じ、contract/change と
省略された review registry は opaque external boundary として保持する。全7 relation variant と opaque
boundary の valid fixture を保持し、malformed closure を拒否することを native MCP 76 tests と Python
compile で確認した verified partial である。ADR:
`docs/adr/decisions-v0.3-native-mcp-manifest-referential-closure.md`。target runtime、provider semantics、
Rust/native full parity は残る。

さらに native MCP emitted manifest の `execution.sampling.coverage` を Rust `SamplingPlan` と同じ invariant
へ揃え、非空 bucket 合計の `cases` 一致、checked u64 overflow 拒否、空/省略 coverage の後方互換を固定した。
canonical、mismatch、overflow、empty coverage の fixture を native MCP 77 tests と Python compile で確認した
verified partial である。ADR: `docs/adr/decisions-v0.3-native-mcp-manifest-sampling.md`。provider semantics、
target runtime、Rust/native full parity は残る。

さらに native MCP `lsharp_validate` report の5つの counter
（`open_questions` / `independent_reviews` / `contradicting_observations` / `stale_reviews` /
`stale_evidence`）を共有 `U64_MAX` 境界まで postflight 検証し、`u64::MAX + 1` を native 成功として
返さない contract を追加した。schema の maximum assertion と overflow fixture を native MCP 77 tests、
Python compile、docs audit、diff check で確認した verified partial である。provider semantics、target runtime、
Rust/native full parity は残る。ADR:
`docs/adr/decisions-v0.3-native-mcp-validate-report-counters.md`。

さらに native MCP shim が直接読む JSON-RPC request、native report、direct/file manifest input、emitted
manifest を strict object-pairs parserで読み、nestedを含む duplicate JSON object key を最後の値へ黙って
上書きせず fail-closedにした。duplicate `id` / `schema_version` / report `status` fixtureを native MCP
78 tests、Python compile、docs audit、diff checkで確認した verified partial である。provider semantics、target
runtime、Rust/native full parity は残る。ADR:
`docs/adr/decisions-v0.3-native-mcp-json-duplicate-keys.md`。

さらに native MCP の LSP response frame と package/stdlib `api.json` artifact も shared strict JSON
decoderへ統合し、duplicate `result` / `package` key を last-value 化せず relay-specific error として
拒否する contract を追加した。LSP、package、stdlib の duplicate fixtureを native MCP 79 tests、Python
compile、docs audit、diff checkで確認した verified partial である。provider semantics、target runtime、
Rust/native full parity は残る。ADR:
`docs/adr/decisions-v0.3-native-mcp-json-relays.md`。

さらに shared strict JSON decoder の `parse_constant` で `NaN` / `Infinity` / `-Infinity` を拒否し、
JSON-RPC、native report、LSP、package/stdlib artifact の非標準 JSON constant を成功値として返さない
boundaryを固定した。native MCP 79 tests、Python compile、docs audit、diff checkで確認した verified partial
である。provider semantics、target runtime、Rust/native full parity は残る。ADR:
`docs/adr/decisions-v0.3-native-mcp-json-constants.md`。

## Current priority — v0.2 Milestone 2

正本:

- [Milestone 2 — Intent and evidence graph](docs/development/planning/v0.2-milestone-02.md)
- [Intent AST と stable ID](docs/development/planning/v0.2-intent-ast.md)
- [Evidence graph](docs/development/planning/v0.2-evidence-graph.md)
- [Intent validation model](docs/development/planning/v0.2-validation-model.md)

- [~] `EC-M2-01` intent AST と stable ID — Rust canonical model、source の
  `:intent` / `:claim` / `:assumption` / `:open-question`、`motivates` /
  `constrained-by` / `tested-by`、fail-closed な typed ID は verified。ID 省略時は自動命名せず
  fail-closed とする。nested `module` / `private` / `impl` をまたぐ project-level duplicate 検査は
  shared fixture、Rust source suite、selfhost E2E、Mac Apple Silicon / Linux x86_64 native source-file
  smokeで verified partial slice として ADR に記録した。`47743365` では Rust CLI の `validate --source` を
  regular file または directory の deterministic project aggregateへ拡張し、cross-file duplicate nodeの
  code `2`、first/duplicate span、exit `1`、stdout空、manifest未生成を verifiedした。validな複数 file graphの
  report/manifest、cross-file typed edge、duplicate evidence/review、directory入力の selfhost/native App.Cli・
  EmbeddedCli・MCP parity、Linux current-source runtimeは残る。selfhost parserの ADT/record 定義 metadata 保持と
  `IntentSource` の node/typed-edge projection は Rust-host actual Wasm の verified slice として ADR に記録したが、
  EC-M2-01 全体の完了条件は満たしていない。
- [~] `EC-M2-02` evidence graph — required provenance を持つ evidence record、
  `supports` / `contradicts` の registry closure、source の `shrinks` / `coverage`、
  canonical manifest と MCP input/output schema の coverage bucket 名 non-blank 境界、
  `evaluates` / `invalidates` の Rust source typed-edge projection、Rust CLI の source→report/manifest
  projection、canonical manifest projection、optional review provenance registry の Rust CLI
  input/output roundtrip と未登録 review edge の fail-closed 検査、
  source `:review` の opaque registry producer と `validate --source --emit-manifest` の review
  registry projection、未知 visibility/空 digest/duplicate ID の span 付き fail-closed 検査、
  Rust MCP の review registry input/output schema と inline manifest projection、
  `public` / `redacted` privacy boundary は verified。実行 trace と generator policy、
  review provenance authentication、review/evaluates/invalidates lifecycle、外部 provenance と
  selfhost/native parity を閉じる。selfhost parser の `:review` kind 16 triple payload と
  `IntentSource` の opaque review registry projection は Rust-host actual Wasm の verified slice
  として追加済みであり、kind 17/18 の `evaluates` / `invalidates` edge projection と review
  registry closure/error boundary も verified slice として追加した。selfhost Evidence consumer の
  review registry、typed review/change edge、登録済み Evidence subject closure、optional `reviews`
  registry と manifest JSON projection も Rust-host actual Wasm で verified slice として追加した。
  ただし review lifecycle/authentication、native stage0 producer/runtime parity は残る。Rust
  canonical `IntentGraph::stale_subjects` による invalidated review → evaluated evidence の
  deterministic stale propagation も verified slice として追加したが、外部 lifecycle/provider
  auth/selfhost-native parity は未完了である。さらに `ValidationReport` が stale review/evidence
  件数を JSON/text facts として投影し、stale が残る場合に `unknown` へ fail-closed する Rust
  canonical report slice を追加した。公開 CLI/MCP、selfhost/native parity は未完了である。
  selfhost `Tools.Validation.Stale` の source graph projection と `App.Cli` /
  `EmbeddedCli` の stale report/unknown wiring は Rust-host actual Wasm で verified したが、
  native stage0 parity は未完了である。
  さらに review の visibility が未知値となる source fixture を native smoke に追加し、stable な
  `source validation error:8`、exit `1`、report/manifestなしの fail-closed 境界を要求した。
  既存 Rust source adapter の `InvalidReviewField`/invalid review 診断と同じ review metadata
  boundary を native contract へ接続する verified sliceだが、current source-commit に一致する
  実 stage0 artifact/runtime の evidence ではない。
  さらに review の `provenance_digest` が whitespace-only となる source fixtureも native smokeへ
  追加し、stable な `source validation error:8`、exit `1`、report/manifestなしの fail-closed 境界を
  要求した。既存 Rust source adapter の空 digest `InvalidReviewField` 診断と同じ metadata validation
  を native contractへ接続する verified sliceだが、実 stage0 artifact/runtime の evidence ではない。
  さらに review ID の wire shape が不正な source fixture を native smokeへ追加し、stable な
  `source validation error:2`、exit `1`、report/manifestなしの fail-closed 境界を要求した。既存 Rust
  source adapter の review ID parse 診断と同じ stable-ID validation を native contractへ接続する
  verified sliceだが、実 stage0 artifact/runtime の evidence ではない。
  さらに review ID が空文字となる source fixtureも native smokeへ追加し、stable な
  `source validation error:8`、exit `1`、report/manifestなしの fail-closed 境界を要求した。これは
  Rust source adapter の必須 review field 検査と同じ invalid-review metadata boundary を native
  contractへ接続する verified sliceだが、実 stage0 artifact/runtime の evidence ではない。
  さらに Rust source adapter が空 review ID を wire-format error へ落とさず `InvalidReviewField` の
  code `8` として返すよう修正し、selfhost actual Wasm E2E でも同じ review metadata boundary を固定した。
  native source-file smoke の既存 empty-review-ID fixtureと Rust/selfhost direct consumer を揃えた verified
  sliceだが、current source-commit に一致する packaged stage0 artifact/runtime の evidence ではない。
  さらに malformed review ID と whitespace-only provenance digest が同時にある場合も、required digest
  の `InvalidReviewField` code `8` を stable-ID wire error より先に返すよう Rust source adapter を修正し、
  selfhost actual Wasm と native source-file smoke の同じ precedence fixtureを追加した verified sliceだが、
  current source-commit に一致する packaged stage0 artifact/runtime の evidence ではない。
  さらに review payload の引数不足となる malformed source fixtureも native smokeへ追加し、stable な
  `source validation error:1`、exit `1`、report/manifestなしの fail-closed 境界を要求した。既存 Rust
  source adapter の malformed review metadata 診断と同じ parser boundary を native contractへ接続する
  verified sliceだが、実 stage0 artifact/runtime の evidence ではない。
  さらに review payload の引数過多となる malformed source fixtureも native smokeへ追加し、stable な
  `source validation error:1`、exit `1`、report/manifestなしの fail-closed 境界を要求した。Rust syntax
  oracle の `LS0101` arity rejection と同じ parser boundary を native contractへ接続する verified
  sliceだが、実 stage0 artifact/runtime の evidence ではない。
  さらに review edge の endpoint が不足する malformed source fixtureも native smokeへ追加し、stable な
  `source validation error:1`、exit `1`、report/manifestなしの fail-closed 境界を `evaluates` /
  `invalidates` の双方で要求した。既存 Rust source adapter の malformed review-edge 診断と同じ
  parser boundary を native contractへ接続する verified sliceだが、実 stage0 artifact/runtime の
  evidence ではない。
  さらに review edge の endpoint が余分になる malformed source fixtureも native smokeへ追加し、stable な
  `source validation error:1`、exit `1`、report/manifestなしの fail-closed 境界を `evaluates` /
  `invalidates` の双方で要求した。Rust syntax oracle の `LS0101` arity rejection と同じ parser
  boundary を native contractへ接続する verified sliceだが、実 stage0 artifact/runtime の evidence
  ではない。
  さらに `validate --source` の negative sampling を `App.Cli` / `EmbeddedCli` の Rust-host actual
  Wasm へ接続し、`source validation error:11`、exit `1`、report/manifestなしの fail-closed boundary
  を両 surface で固定した。current-source stage0、sampling の実行意味論は残る。
- [~] `EC-M2-03` `lsharp validate` — version 1 manifest parser、source adapter、
  `--emit-manifest` の atomic/durable file boundary、deterministic text/JSON report、
  optional `reviews` registry の Rust CLI roundtrip と未登録 review edge の non-zero/
  no-output contract、
  positional manifest と `--source` の読み込み失敗を driver I/O code `LS5001`、空 stdout、
  no-report として返す Rust CLI boundary、
  Rust MCP `lsharp_validate` の `reviews` input/output schema、input top-level unknown field の
  `additionalProperties: false` boundary、空の manifest/path string を schema で拒否する
  `minLength: 1` boundary、coverage bucket 名の non-blank schema boundary と
  `include_manifest` projection、
  `pass=0` / `fail=1` / `unknown=2` の Rust CLI は verified。selfhost `App.Cli` と
  `EmbeddedCli` の source/report/pass-fail-unknown/exit、EmbeddedCli の
  `--emit-manifest` による report と version 1 manifest file の分離出力は Rust-host
  actual Wasm で verified。さらに EmbeddedCli は manifest write failure 時に
  no-report/no-file と exit `1` を返す fail-closed boundary も Rust-host actual Wasm で
  verified。EC-M3-01 の Rust canonical source/manifest fixture、native
  source-file smoke の report/exit/bytes 契約、duplicate node の exit `1` /
  diagnostic-only fail-closed 契約、Cargo/Rust/host lsharp を成功経路から遮断する harness
  contract も verified。selfhost/native の manifest producer/parser、native canonical parity、
  atomic/durable emission、native stage0 の write/provenance failure、release-level
  provenance、両 supported target の runtime evidence を閉じる。Rust canonical report の
  `stale_reviews` / `stale_evidence` facts と stale→`unknown` policy、Rust MCP schema/output
  も verified slice として追加した。selfhost `App.Cli` / `EmbeddedCli` の stale report/exit
  parity は Rust-host actual Wasm で verified したが、native stage0 report parity、native MCP、
  両 supported target の runtime evidence は未完了である。さらに `--format text` の
  deterministic source report（trace gap、status、件数、stale facts）と `--format json` の
  option/exit parity を両 selfhost surface の Rust-host actual Wasm で verified した。Cli の
  complete graph（独立 review 付き）`pass=0` / `status: pass` も同じ text/json status projection
  として verified した。EmbeddedCli の complete graph でも `--format text` の `status: pass` /
  exit `0` と同じ deterministic line projection を verified した。
  独立 review gate は `EvidenceOutcome::Pass` かつ `IndependentReview` の evidence だけを数える。
  failed/unknown/stale/contradicted review evidence を独立 review として扱わない Rust canonical
  report boundary を追加し、failed review の complete graph が `unknown` になる RED→GREEN を固定した。
  これは Rust-host canonical report の verified partial sliceであり、selfhost/native parity、MCP、
  `47743365` では `--source` に regular file または directory を指定でき、directory内の `.ls` fileを
  deterministicに集約してcross-file duplicate nodeを診断-onlyで返す Rust driver boundaryを追加した。
  selfhost/nativeのdirectory入力、project report/manifest producer、current-source artifact/runtime、supported 2 targets は残件である。native source-file smoke の
  inner runnerにも canonical fixtureの `--format text` unknown report（6行の固定順、exit `2`）を
  要求する contract を追加したが、これは fake Lima/provenance harness の verified sliceであり、
  current source-commit に一致する実 stage0 artifact/runtime の証拠ではない。さらに native
  source-file smoke は `--emit-manifest` の親ディレクトリ欠落を stable write failure、exit `1`、
  no-report/no-file として要求する contract を持つが、atomic/durable writer と実 stage0 runtime
  の証拠ではない。さらに independent review を含む complete graph fixtureの JSON `status: pass` /
  exit `0` と text 6行 projection を native smoke contract に追加したが、current source-commit に
  一致する stage0 artifact/runtime での実行 evidence は未取得である。さらに contradiction
  fixture の JSON `status: fail` / exit `1`（`independent_reviews: 1`、
  `contradicting_observations: 1`）と同じ text 6行 projectionを native smoke contract に追加した。
  これは report を保持する判定 failure と parse/graph/write の diagnostic-only failure を分離する
  fake Lima/provenance harness の verified sliceであり、current source-commit に一致する実 stage0
  artifact/runtime の evidence ではない。さらに invalidation された review と、それを
  `evaluates` する evidence の stale propagation fixture を native smoke に追加し、JSON/text の
  `status: unknown` / exit `2`、`stale_reviews: 1`、`stale_evidence: 1`、stderr 空を要求した。
  これは Rust-host/selfhost source projection と native contract を揃える verified sliceだが、
  current source-commit に一致する実 stage0 artifact/runtime の evidence ではない。さらに
  orphan edge が存在しない node を参照する source fixture を native smoke に追加し、stable な
  `source validation error:5`、exit `1`、report/manifestなしの fail-closed 境界を要求した。
  これは既存 Rust source adapter の `MissingNodeReference` と同じ observable error code を
  native contract へ接続する verified sliceだが、current source-commit に一致する実 stage0
  artifact/runtime の evidence ではない。
  さらに edge payload が不足する malformed source fixture を native smoke に追加し、stable な
  `source validation error:1`、exit `1`、report/manifestなしの fail-closed 境界を要求した。
  既存 Rust source adapter の malformed edge/span 診断と同じ入力拒否境界を native contract
  へ接続する verified sliceだが、current source-commit に一致する実 stage0 artifact/runtime
  の evidence ではない。
  さらに wire prefix が不正な edge endpoint の source fixture を native smoke に追加し、stable な
  `source validation error:2`、exit `1`、report/manifestなしの fail-closed 境界を要求した。
  既存 Rust source adapter の malformed ID 診断と同じ入力拒否境界を native contract へ接続する
  verified sliceだが、current source-commit に一致する実 stage0 artifact/runtime の evidence ではない。
  さらに metadata kind と wire prefix が不一致な node fixture を native smoke に追加し、stable な
  `source validation error:3`、exit `1`、report/manifestなしの fail-closed 境界を要求した。
  既存 Rust source adapter の typed kind mismatch 診断と同じ入力拒否境界を native contract へ接続する
  verified sliceだが、current source-commit に一致する実 stage0 artifact/runtime の evidence ではない。
  さらに未登録 evidence を `supports` する source fixture を native smoke に追加し、stable な
  `source validation error:6`、exit `1`、report/manifestなしの fail-closed 境界を要求した。
  既存 Rust source adapter の `EvidenceRegistryRequired` 診断と同じ registry closure 境界を native
  contract へ接続する verified sliceだが、current source-commit に一致する実 stage0 artifact/runtime
  の evidence ではない。
  さらに required field が不足する malformed evidence source fixtureを native smoke に追加し、stable な
  `source validation error:1`、exit `1`、report/manifestなしの fail-closed 境界を要求した。Rust
  syntax oracle の evidence metadata required-field `LS0101` 診断と同じ parser boundary を native
  contractへ接続する verified sliceだが、current source-commit に一致する実 stage0 artifact/runtime
  の evidence ではない。
  さらに evidence の method enum が未定義となる source fixture を native smoke に追加し、stable な
  `source validation error:8`、exit `1`、report/manifestなしの fail-closed 境界を要求した。Rust
  source adapter の `InvalidEvidenceField` 診断と同じ typed evidence field boundary を native
  contractへ接続する verified sliceだが、current source-commit に一致する実 stage0 artifact/runtime
  の evidence ではない。
  さらに evidence の outcome、independence、subject kind が不正となる source fixtureも native smoke
  に追加し、いずれも stable な `source validation error:8`、exit `1`、report/manifestなしの
  fail-closed 境界を要求した。Rust source adapter の `InvalidEvidenceField` 診断と同じ typed
  evidence enum matrix を native contractへ接続する verified sliceだが、current source-commit に
  一致する実 stage0 artifact/runtime の evidence ではない。
  さらに selfhost Evidence consumer の invalid `:method` が native source contract と同じ code `8` を
  返し、field/value を保持する Rust-host E2E を追加した。native stage0 artifact/runtime parity は未完了の
  ままだが、selfhost direct consumer の typed-field code drift を閉じる verified sliceである。
  さらに selfhost Evidence consumer の invalid `:independence` も native source contract と同じ code `8` を
  返し、field/value を保持する Rust-host E2E を追加した。native stage0 artifact/runtime parity は未完了の
  ままだが、selfhost direct consumer の typed-field code drift を閉じる verified sliceである。
  さらに selfhost Evidence consumer の unsupported `:subject` kind も native source contract と同じ code `8` を
  返し、field/value を保持する Rust-host E2E を追加した。native stage0 artifact/runtime parity は未完了の
  ままだが、selfhost direct consumer の typed-field code drift を閉じる verified sliceである。
  さらに selfhost Evidence consumer の invalid `:outcome` も native source contract と同じ code `8` を返し、
  field/value を保持する Rust-host E2E を追加した。既存の selfhost enum 判定が native contract と一致する
  ことを直接固定する verified sliceであり、native stage0 artifact/runtime parity は未完了のままである。
  さらに empty `:method` / `:outcome` / `:independence` を selfhost の required-field code `4` から
  native enum-invalid code `8` へ揃え、3フィールドの Rust oracle、selfhost E2E、native source-file smoke を
  追加した。enum は enum validator、runner/target/provenance は required-field validator が担当する
  boundary を固定した verified sliceだが、native stage0 artifact/runtime parity は未完了のままである。
  さらに同じ evidence ID を二度宣言する source fixtureも native smoke に追加し、stable な
  `source validation error:3`、exit `1`、report/manifestなしの fail-closed 境界を要求した。Rust
  source adapter / selfhost registry の duplicate evidence rejection と同じ identity boundary を
  native contractへ接続する verified sliceだが、current source-commit に一致する実 stage0
  artifact/runtime の evidence ではない。
  さらに evidence named field を同じ record 内で二度宣言する malformed source fixtureも native
  smoke に追加し、stable な `source validation error:1`、exit `1`、report/manifestなしの
  fail-closed 境界を要求した。Rust syntax oracle の duplicate named field `LS0101` 診断と同じ
  parser boundary を native contractへ接続する verified sliceだが、current source-commit に一致する
  実 stage0 artifact/runtime の evidence ではない。
  さらに evidence の required sampling generator が空文字となる source fixtureも native smoke に
  追加し、stable な `source validation error:4`、exit `1`、report/manifestなしの fail-closed 境界を
  要求した。Rust `EvidenceGraph` の `EvidenceValidationError::EmptyField { field: "generator" }` と
  selfhost Evidence registry の required-field code `4` を source contractへ接続する verified slice
  だが、current source-commit に一致する実 stage0 artifact/runtime の evidence ではない。
  さらに evidence の required execution identity runner が空文字となる source fixtureも native
  smoke に追加し、stable な `source validation error:4`、exit `1`、report/manifestなしの fail-closed
  境界を要求した。Rust `EvidenceValidationError::EmptyField { field: "runner" }` と selfhost Evidence
  registry の required-field code `4` を source contractへ接続する verified sliceだが、current
  source-commit に一致する実 stage0 artifact/runtime の evidence ではない。
  さらに invalid evidence ID と空の required runner が同時にある場合も、runner の
  `EvidenceValidationError::EmptyField` code `4` を stable-ID wire error より先に返すよう Rust source
  adapter を修正し、selfhost actual Wasm と native source-file smoke の同じ precedence fixtureを追加した
  verified sliceだが、current source-commit に一致する実 stage0 artifact/runtime の evidence ではない。
  さらに evidence の required execution target が空文字となる source fixtureも native smoke に追加し、
  stable な `source validation error:4`、exit `1`、report/manifestなしの fail-closed 境界を要求した。
  Rust `EvidenceValidationError::EmptyField { field: "target" }` と selfhost Evidence registry の
  required-field code `4` を source contractへ接続する verified sliceだが、current source-commit に
  一致する実 stage0 artifact/runtime の evidence ではない。
  さらに evidence の required source commit が空文字となる source fixtureも native smoke に追加し、
  stable な `source validation error:4`、exit `1`、report/manifestなしの fail-closed 境界を要求した。
  Rust `EvidenceValidationError::EmptyField { field: "source_commit" }` と selfhost Evidence registry の
  wire field `source-commit` / required-field code `4` を source contractへ接続する verified sliceだが、
  current source-commit に一致する実 stage0 artifact/runtime の evidence ではない。
  さらに evidence の required artifact digest が空文字となる source fixtureも native smoke に追加し、
  stable な `source validation error:4`、exit `1`、report/manifestなしの fail-closed 境界を要求した。
  Rust `EvidenceValidationError::EmptyField { field: "artifact_digest" }` と selfhost Evidence registry の
  wire field `artifact-digest` / required-field code `4` を source contractへ接続する verified sliceだが、
  current source-commit に一致する実 stage0 artifact/runtime の evidence ではない。
  さらに evidence の required provenance producer が空文字となる source fixtureも native smoke に追加し、
  stable な `source validation error:4`、exit `1`、report/manifestなしの fail-closed 境界を要求した。
  Rust `EvidenceValidationError::EmptyField { field: "producer" }` と selfhost Evidence registry の
  required-field code `4` を source contractへ接続する verified sliceだが、current source-commit に
  一致する実 stage0 artifact/runtime の evidence ではない。
  さらに evidence の required provenance tool version が空文字となる source fixtureも native smoke に
  追加し、stable な `source validation error:4`、exit `1`、report/manifestなしの fail-closed 境界を要求した。
  Rust `EvidenceValidationError::EmptyField { field: "tool_version" }` と selfhost Evidence registry の
  wire field `tool-version` / required-field code `4` を source contractへ接続する verified sliceだが、
  current source-commit に一致する実 stage0 artifact/runtime の evidence ではない。
  さらに evidence の required provenance timestamp が空文字となる source fixtureも native smoke に
  追加し、stable な `source validation error:4`、exit `1`、report/manifestなしの fail-closed 境界を要求した。
  Rust `EvidenceValidationError::EmptyField { field: "timestamp" }` と selfhost Evidence registry の
  required-field code `4` を source contractへ接続する verified sliceだが、current source-commit に
  一致する実 stage0 artifact/runtime の evidence ではない。
  さらに evidence の required execution runner が空白だけとなる source fixtureも native smoke に
  追加し、stable な `source validation error:4`、exit `1`、report/manifestなしの fail-closed 境界を要求した。
  Rust canonical の `trim().is_empty()` と selfhost Evidence registry の nonblank 判定を同じ
  `runner` fieldへ接続する verified sliceだが、current source-commit に一致する実 stage0
  artifact/runtime の evidence ではない。
  さらに source node の stable ID 欠落と本文欠落の fixtureを native smoke に追加し、stable な
  `source validation error:1`、exit `1`、report/manifestなしの fail-closed 境界を要求した。Rust
  syntax oracle の `LS0101` required-operand 診断と同じ ID 明示 / non-empty text policy を native
  contractへ接続する verified sliceだが、current source-commit に一致する実 stage0 artifact/runtime
  の evidence ではない。
  さらに source node の本文が空白だけとなる fixtureも native smoke に追加し、stable な
  `source validation error:1`、exit `1`、report/manifestなしの fail-closed 境界を要求した。Rust
  canonical の `text.trim().is_empty()` と selfhost `IntentSource` の nonblank 判定を同じ node
  text policyへ接続する verified sliceだが、current source-commit に一致する実 stage0
  artifact/runtime の evidence ではない。
  さらに invalid stable ID と whitespace-only node text が同時にある場合も、本文の
  `NodeTextError::EmptyText` / malformed code `1` を stable-ID wire error より先に返すよう Rust source
  adapter を修正し、selfhost actual Wasm と native source-file smoke の同じ precedence fixtureを追加した
  verified sliceだが、current source-commit に一致する実 stage0 artifact/runtime の evidence ではない。
  さらに evidence の `:subject` が空白だけとなる source fixtureも native smoke に追加し、stable な
  `source validation error:2`、exit `1`、report/manifestなしの fail-closed 境界を要求した。Rust
  canonical の stable ID parser と selfhost `source-wire-shape-valid?` を同じ subject wire policyへ
  接続する verified sliceだが、current source-commit に一致する実 stage0 artifact/runtime の
  evidence ではない。
  さらに evidence の ID が空白だけとなる source fixtureも native smoke に追加し、stable な
  `source validation error:2`、exit `1`、report/manifestなしの fail-closed 境界を要求した。Rust
  canonical の `EvidenceId::parse` と selfhost の evidence ID wire validation を同じ stable-ID
  policyへ接続する verified sliceだが、current source-commit に一致する実 stage0 artifact/runtime の
  evidence ではない。
  さらに evidence の ID が空文字となる source fixtureも native smoke に追加し、stable な
  `source validation error:2`、exit `1`、report/manifestなしの fail-closed 境界を要求した。Rust
  canonical の `EvidenceId::parse` と selfhost の evidence ID wire validation を同じ empty stable-ID
  policyへ接続する verified sliceだが、current source-commit に一致する実 stage0 artifact/runtime の
  evidence ではない。
  さらに review registry が存在する状態で未登録 review を `evaluates` する source fixture を native
  smoke に追加し、stable な `source validation error:10`、exit `1`、report/manifestなしの fail-closed
  境界を要求した。既存 Rust source adapter の `MissingReview` 診断と同じ review registry closure
  境界を native contract へ接続する verified sliceだが、current source-commit に一致する実 stage0
  artifact/runtime の evidence ではない。
  さらに同じ review ID を二度宣言する source fixture を native smoke に追加し、stable な
  `source validation error:7`、exit `1`、report/manifestなしの fail-closed 境界を要求した。
  既存 Rust source adapter の `DuplicateReview` 診断と同じ registry uniqueness 境界を native
  contract へ接続する verified sliceだが、current source-commit に一致する実 stage0 artifact/runtime
  の evidence ではない。
  さらに review edge が review 自体を `evaluates` の subject に指定する source fixture を native
  smoke に追加し、stable な `source validation error:9`、exit `1`、report/manifestなしの fail-closed
  境界を要求した。既存 Rust source adapter の `EdgeSubjectKindMismatch` 診断と同じ review edge
  subject-kind 境界を native contract へ接続する verified sliceだが、current source-commit に一致する
  実 stage0 artifact/runtime の evidence ではない。
  さらに `invalidates` の subject に claim を指定する source fixtureも native smokeへ追加し、
  同じ stable code `9`、exit `1`、report/manifestなしの fail-closed を要求した。`evaluates` と
  `invalidates` の typed review-edge subject boundary を双方の native contractへ接続する verified
  sliceだが、current source-commit に一致する実 stage0 artifact/runtime の evidence ではない。
  さらに登録済み review と未登録 review を同じ source に置き、`invalidates` が未登録 review を参照
  する fixtureも native smokeへ追加した。stable な `source validation error:10`、exit `1`、
  report/manifestなしの fail-closed を要求し、review registry closure の両 edge relation を native
  contractへ接続する verified sliceとしたが、実 stage0 artifact/runtime の evidence ではない。
  さらに明示 review registry の未登録 `evaluates` review と不正な review subject kind を同時に
  持つ fixtureを追加し、subject-kind code `9` より先に missing-review code `10` を返す precedence を
  Rust source adapter、selfhost、native smoke で統一した verified sliceだが、current source-commit に
  一致する実 stage0 artifact/runtime の evidence ではない。
  さらに review の `evaluates` / `invalidates` が未登録 evidence を subject にする source fixtureも
  native smokeへ追加し、stable な `source validation error:6`、exit `1`、report/manifestなしの
  fail-closed を双方の relation で要求した。既存 Rust source adapter の `EvidenceRegistryRequired`
  と同じ review-edge evidence registry boundary を native contractへ接続する verified sliceだが、
  current source-commit に一致する実 stage0 artifact/runtime の evidence ではない。
  さらに `supports` / `contradicts` が未登録で wire shape も不正な evidence を参照する fixtureを
  追加し、stable な `source validation error:6`、exit `1`、report/manifestなしの fail-closed を
  両 relation で要求した。Rust source adapter も stable-ID parse より先に registry closure を
  判定するよう修正し、selfhost/native と同じ evidence-edge precedence を固定した verified sliceだが、
  current source-commit に一致する実 stage0 artifact/runtime の evidence ではない。

さらに明示 review registry の未登録 `evaluates` / `invalidates` review endpoint を source-local
`MissingReviewReference` として保持し、relation、ID、directive span を Rust source adapter の
diagnostic に残すようにした。selfhost の code `10` / span と native source-file smoke の
no-report/no-manifest boundary に対応する verified sliceだが、driver の EmbeddedCli build、実
stage0 artifact/runtime、Mac/Linux matrix、native fallback exclusion の evidence ではない。

さらに evidence の required field (`runner` / `target` / `source-commit` / `artifact-digest` /
`generator` / `producer` / `tool-version` / `timestamp`) が空または whitespace-only の場合を
`InvalidEvidenceRequiredField` として field、value、directive span 付きで返すようにした。selfhost
の stable code `4` / span と native source-file smoke の no-report/no-manifest boundary に対応する
verified sliceだが、driver の EmbeddedCli build、実 stage0 artifact/runtime、Mac/Linux matrix、
native fallback exclusion の evidence ではない。

さらに source node の `text` が空または whitespace-only の場合を `InvalidNodeField` として
field、value、directive span 付きで返すようにした。selfhost source adapter の stable code `1` /
span E2E と native source-file smoke の no-report/no-manifest boundary に対応する verified sliceだが、
driver の EmbeddedCli build、実 stage0 artifact/runtime、Mac/Linux matrix、native fallback exclusion
の evidence ではない。

さらに source node の stable ID wire format／segment が不正な場合を `NodeIdAt` として directive
span 付きで返すようにした。selfhost source adapter の stable code `2` / span E2E と native
source-file smoke の no-report/no-manifest boundary に対応する verified sliceだが、driver の
EmbeddedCli build、実 stage0 artifact/runtime、Mac/Linux matrix、native fallback exclusion の evidence
ではない。

さらに evidence record 自体の stable ID wire format／segment が不正な場合を `EvidenceIdAt` として
directive span 付きで返すようにした。selfhost Evidence consumer の stable code `2`、field/value、
span E2E と native source-file smoke の no-report/no-manifest boundary に対応する verified sliceだが、
driver の EmbeddedCli build、実 stage0 artifact/runtime、Mac/Linux matrix、native fallback exclusion
の evidence ではない。

さらに evidence の `subject` ID wire format／segment が不正な場合を `EvidenceSubjectIdAt` として
directive span 付きで返すようにした。selfhost Evidence consumer の stable code `2`、field/value、
span E2E と native source-file smoke の no-report/no-manifest boundary に対応する verified sliceだが、
driver の EmbeddedCli build、実 stage0 artifact/runtime、Mac/Linux matrix、native fallback exclusion
の evidence ではない。

さらに evidence の `:coverage` に空 bucket が含まれる場合を Rust source adapter の
`InvalidEvidenceField`（field/value/directive span）として拒否し、selfhost Evidence consumer の
empty-field code `4` と malformed／empty／negative／duplicate coverage の directive span を揃えた。
Rust source suite 54件、selfhost evidence registry 39件、source adapter 30件、Linux x86_64 native
stage0 source-file smoke/provenance gate を通過した verified partial sliceである。whitespace-only
bucket policy、coverage count、parser/manifest/validate CLI 全体、current-source artifact/runtime、
Mac/Linux matrix、EC-M2-02 aggregate は残件。ADR: `docs/adr/decisions-v0.2-native-validation-evidence-coverage-bucket.md`。

canonical `SamplingPlan` / `EvidenceGraph` でも空 coverage bucket を登録前に
`EvidenceValidationError::EmptyField { field: "coverage" }` として拒否するよう揃えた。
`evidence_required_fields` 6件、`evidence_graph` 5件、source suite 54件、Linux x86_64 native
stage0 source-file smoke/provenance gate を通過した verified partial sliceである。duplicate/
whitespace policy、coverage count/cases、manifest/validate CLI、current-source artifact/runtime、
Mac/Linux matrix、EC-M2-02 aggregate は残件。ADR: `docs/adr/decisions-v0.2-native-validation-evidence-canonical-sampling.md`。

続く coverage whitespace policy では、canonical `SamplingPlan` / `EvidenceGraph`、Rust source
adapter、selfhost Evidence consumer の whitespace-only bucket を共通の empty-field policy へ揃えた。
元の bucket value と directive/form span を保持し、selfhost/native の stable code `4`、exit `1`、
report/manifestなしを固定した。evidence required fields 8件、evidence graph 5件、source suite 55件、
selfhost evidence registry 40件、source adapter 31件、Linux x86_64 native source-file smoke/provenance
gateを通過した verified partial sliceである。Unicode whitespace、duplicate/count/cases、manifest/
validate、current-source artifact/runtime、Mac/Linux matrix、EC-M2-02 aggregate は残件。ADR:
`docs/adr/decisions-v0.2-native-validation-evidence-coverage-whitespace.md`。

さらに duplicate coverage bucket の parser-owned boundary を native source-file smokeへ接続した。
Rust parser は `LS0101`、selfhost direct Evidence registry は code `10` で重複を拒否し、native stage0
 source-file smoke は parser error code `1`、exit `1`、report/manifestなしを要求する。Rust source
 adapter に後段の `BTreeMap` duplicate 再判定は追加せず、parser を通過した canonical source record
 の責務を維持した verified partial sliceである。canonical map が duplicate を表現できない点、
 coverage count/cases、Unicode whitespace、manifest/validate、current-source artifact/runtime、
 Mac/Linux matrix、EC-M2-02 aggregate は残件。ADR:
`docs/adr/decisions-v0.2-native-validation-evidence-duplicate-coverage-parser.md`。

version 1 JSON manifest input でも whitespace-only coverage bucket を canonical graph 登録前に
`GraphError::InvalidEvidence { source: EvidenceValidationError::EmptyField { field: "coverage" } }`
として拒否する回帰テストを追加した。`validation_input` 17件、rustfmt、diff checkを通過した
verified partial sliceである。manifest の native source/runtime parity、report/atomic writer、
coverage count/cases、Unicode whitespace、Mac/Linux matrix、EC-M2-03 aggregate は残件。ADR:
`docs/adr/decisions-v0.2-native-validation-manifest-coverage-whitespace.md`。

2026-07-31 に Rust MCP `lsharp_validate` の input schema を runtime の empty-string boundary と同期した。
`manifest` の JSON string variant、`file`、`manifest_file`、`trust_store`、`review_lifecycle` に
`minLength: 1` を追加し、runtime が空文字を parse/path error として拒否する routeを static schemaでも
fail-closed にした。空 `source` は既存 semantics（空 programを受理し得る）を保つため変更していない。
`test_validate_tool_input_schema_rejects_empty_manifest_and_path_strings` の Draft 2020-12 validator matrix
と `mcp_server::tests` 70件を通過した。selfhost/native MCP、current-source artifact/runtime、対応2 target、
EC-M2-03 aggregate は残件。ADR: `docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-31 に coverage bucket 名の non-blank policy を canonical manifest schema と MCP input/output schemaへ
接続した。runtime の `trim().is_empty()` boundary と同じく、空文字・ASCII空白・NBSP-only の property nameを
Draft 2020-12 `propertyNames.pattern: "\\S"` で rejectする。canonical schema、MCP input、MCP outputの
validator matrixを `test_manifest_schemas_use_draft202012_validator_for_valid_and_invalid_fixtures` に追加し、
既存の valid fixtureを保ったまま3種類の空 bucketを全validatorで拒否することを固定した。Rust-host schema
verified partial sliceであり、selfhost/native manifest parser、current-source artifact/runtime、対応2 target、
EC-M2-02/03 aggregate は残件。ADR: `docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-31 に review provenance wire schema の `lifecycle.effective_at` を、Rust
`ReviewLifecycleEvent` が要求する canonical UTC timestamp 定義へ接続した。これまで schema は
`non_empty_string` のみを参照していたため、offset付き・非canonicalな時刻を schema consumer が受理できた。
`review_provenance_schema_requires_canonical_timestamp_for_lifecycle_effective_at` の RED→GREEN と、既存の
`wire_rejects_noncanonical_lifecycle_effective_timestamp` を含む `validation_schema` 8件 / `review_wire` 5件で
lexical schema/parser boundary を固定した。Rust-host schema verified partial sliceであり、暦日の実在性を含む
validator matrix、provider/authentication、selfhost/native producer、対応2 target、EC-M3 aggregate は残件。
ADR: `docs/adr/decisions-v0.3-review-wire-schema-timestamp.md`。

2026-07-31 に MCP `lsharp_validate` input schema の review verification context を runtime の all-or-none
policyへ接続した。`review_subject_digest` / `review_source_commit` / `review_now` の相互依存と、
`review_artifact_digest` 指定時の4 field依存を Draft 2020-12 `dependentRequired` で宣言し、complete contextを
保ったまま5種類の partial inputを `test_validate_tool_input_schema_requires_complete_review_context` で拒否する
validator matrixを追加した。Rust-host MCP schema/runtime verified partial sliceであり、selfhost/native MCP、
current-source artifact/runtime、provider/authentication、対応2 target、EC-M3 aggregate は残件。ADR:
`docs/adr/decisions-v0.3-review-explicit-context.md`。

2026-07-31 に MCP `lsharp_validate` の `review_now` schemaを runtime の canonical UTC timestamp lexical
boundaryへ接続した。`minLength` だけだった input propertyへ `^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$`
patternを追加し、offset付き、空白区切り、fractional seconds、任意文字列を
`test_validate_tool_input_schema_requires_canonical_review_now` の Draft 2020-12 validator matrixで拒否した。
暦日の実在性は既存 Rust canonical timestamp validatorの責務として維持する。Rust-host MCP schema/runtime
verified partial sliceであり、selfhost/native MCP、current-source artifact/runtime、provider/authentication、
対応2 target、EC-M3 aggregateは残件。ADR: `docs/adr/decisions-v0.3-review-explicit-context.md`。

次の実装は `EC-M2-01`〜`EC-M2-03` の未接続入力を一つの RED に絞る。current plan の
acceptance と依存順を確認し、完了 slice の履歴を TODO へ再展開しない。

## Next milestone — v0.3 Review provenance lifecycle

この節は上記 `## Next milestone — v0.3 review provenance / lifecycle` へ統合した。
同じ `EC-M3-01`〜`05` を両方の節が `[~]` で抱えており、二重計上になっていたため。
旧節が持っていた verified partial 記録は一つも捨てず、統合先の対応する項目の直下へ
そのまま移してある。v0.3 の未完項目は上記節を正本とする。

この milestone の verified slice は ADR に残すが、項目全体の completion boundary を満たすまで
`[~]` を維持する。次の RED と validation gate は
[`v0.3-milestone-01.md`](docs/development/planning/v0.3-milestone-01.md) の M3-04-N1 に従い、
現在の `lsharp-types` clock contract を暗黙に拡張しない。

## v0.2 Milestone 1 closure

個別 slice の履歴と current boundary は
[Rust 依存境界の縮小](docs/development/operations/rust-boundary-reduction.md) を正本とする。

- [~] `EC-M1-01` Rust/selfhost observable parity — invariant scope、computation/match、
  diagnostics、module/import、qualified/private record の parser/type/runtime slice と
  両 supported target の current-source core stage0 smoke は verified。constructor/record/GADT の
  残る semantics、全 diagnostic/span、standalone source check、full cross-target aggregate を閉じる。
  2026-07-31 に selfhost `Types.TypeInferApply` の 3-64 引数 application を共通の 64 要素
  bounded rooted continuationへ通し、8/64 の成功、65 の fail-closed、failure propagation、既存
  curried applyを合わせた focused E2E 13件を通した。Linux x86_64 fixed point は status `pass`、
  stage2/stage3 code length 各 `11332908`、stdout SHA-256 は両方
  `aa5cee91b5f47dd54a7da64492859bb1b9eede381059051713e85310115ba7ad` で一致した。これは
  TypeInferApply の 0-64 bounded arity と current-source Linux native self-regeneration の
  verified sliceであり、65以上の診断/span契約、全 diagnostic/span、Mac/Linux aggregate parity は
  残る。
  2026-07-31 に selfhost `Types.TypeInferPattern` の constructor/record child pattern inference を
  64 要素 bounded rooted continuationへ集約し、65 要素 cross-chunk contract/runtime 2件、
  match error 10件、quote/pattern 12件を通した。Linux x86_64 fixed point は status `pass`、
  stage2/stage3 code length 各 `11168596`、stdout SHA-256 は両方
  `dad391cd36df64b6354b1f4429aaf7a4c410697b7ca74606fbb2865dc2186bb1` で一致した。これは
  TypeInferPattern child traversal と current-source Linux native self-regeneration の verified
  sliceであり、record schema inference、全 diagnostic/span、Mac/Linux aggregate parity は残る。
- [~] `EC-M1-02` canonical metadata IR — canonical case/assert/property inventory、
  typed binder、precondition/postcondition、directive span の slice は verified。一般 `TypeExpr`、
  全 `ContractSuite` evaluator、binder/predicate 個別 span、formatter/docs、2 target evidence を閉じる。
- [~] `EC-M1-03` form separation and migration — canonical form と legacy migration report の
  text/JSON slice は verified。全 form evaluator、schema、formatter/docs/MCP、2 target evidence を閉じる。
- [~] `EC-M1-04` strict predicate and non-vacuity — Bool preflight、zero-case、
  static reachability/vacuity の slice は verified。動的・compound predicate、全 diagnostic/span、
  evaluator/runtime、2 target aggregate を閉じる。2026-07-31 に selfhost
  `Types.TypeInferAssertions` の string scanner、assertion/case form、program/module/property
  aggregation を 64 要素単位の bounded rooted continuationへ揃え、65要素境界を跨ぐ focused E2E と
  parser diagnostics `0` を確認した。Linux x86_64 fixed point は status `pass`、stage2/stage3
  code length 各 `11491724`、stdout SHA-256 は両方
  `bfff156740a634e25a4fc968ca2a83c9ce62227ed3846d70a3d59658fd6d1d76` で一致した。
  これは TypeInferAssertions の bounded traversal verified sliceであり、全 diagnostic/span、
  evaluator/runtime、Mac/Linux aggregate parityは残る。
- [~] `EC-M1-05` reproducible type-directed sampling — Int/Bool/String の deterministic prefix は
  verified。一般 `TypeExpr`、constraint generator、seed/shrink/coverage、2 target evidence を閉じる。
- [~] `EC-M1-06` structured assurance report — implementation conformance と intent validation を
  混同しない text/JSON report の slice は verified。Rust driver の `test --format json` にも
  canonical `:case`/`:assert` の pass/fail と preflight failure、exit `0/2` を接続した。全 form、
  EmbeddedCli、Rust/selfhost report field differential、provenance、2 target evidence を閉じる。
  参照実装が `codex/v0.2-ec-m1-06-all-form-differential` にある (ADR
  [`decisions-worktree-absorption-2026-08-20.md`](docs/adr/decisions-worktree-absorption-2026-08-20.md)
  で 25 commit を判定済み)。live なのは次の 4 点で、**branch 単位では取り込まない**
  (main と 1454 commit 乖離しており、`mcp_server.rs` の分割前を前提にしている)。
  - JSON provenance の field 集合 — main は `Cli.ls:954` の 3 field、branch は 7 field。
    ただし **branch の 4 field はすべて literal 定数**で、commit hash / digest の注入はしていない。
    7 field という shape が正しいか、`runner` を `"selfhost"` から `"selfhost-cli"` へ変えて
    Rust report との differential 相手を動かしてよいかは、**ここで決める設計判断**
  - JSON の `contracts` field (form 別の内訳) — main に無い
  - mixed case+property の JSON report と、CLI `--format text` の e2e —
    main の e2e に `test_format_text_*` は 1 件も無い
  - canonical `:case` の failure message — main の selfhost `run-cases-loop` は message を持たず、
    `crates/lsharp-wasm/src/test_runner.rs:297` の Rust oracle 側にしか無い
- [~] `EC-M1-07` native parity and migration closure — current-source native fixed-point と
  source-file smoke は両 target の verified slice を持つ。Rust oracle、standalone Wasm、
  full public surface、guide/schema/MCP/migration docs を同じ observable contract へ揃える。
  migration enum の契約自体は既に閉じている (selfhost の fail-closed validator
  `legacy-migration-row-schema-valid?` + [ADR](docs/adr/decisions-v0.2-selfhost-migration-enum-schema.md)
  + `crates/lsharp-driver/src/mcp_tests.rs:1756`)。残るのは **機械可読な契約文書が無い**一点で、
  参照実装が `codex/v0.2-ec-m1-06-all-form-differential` の `806929ff`
  (`docs/schemas/legacy-migration.schema.json`、100 行) にある。

## V2-16 — Rust dependency boundary reduction

`V2-16a` no-Cargo development loop と `V2-16d` native development E2E は完了履歴へ移動済み。
残る aggregate は次のとおり。

- [~] `LEGACY-LANG-01` record pattern parity — source/ftable の direct/nested pattern、
  nominal marker、field binding は verified。2026-07-31 に `TypeInferRecord.ls` の
  record field/value inference、record literal field lookup、declared literal/update
  inference を 64 要素 bounded/rooted scan へ移行し、65 要素 fixture と既存 record
  computation regression、Linux x86_64 stage2/stage3 fixed-point を確認した。一般 Map API、
  `Type.ls` の `type-record-field-type` / `type-record-fields-eq` も 64 要素 bounded/rooted
  scan へ移行し、65 field lookup/equality fixture、private visibility、record computation
  regression、Linux x86_64 fixed-pointを確認した。
  `TypeInferRecordDecl.ls` の `:only` 判定と accessor filtering も 64 要素 bounded/rooted
  scan へ移行し、65 要素 export filtering fixture と既存 private visibility regression を
  確認した。
  `Syntax.Parser.ls` の name hash、qualified symbol dot、整数桁、string literal hash scanも
  64 要素 bounded/rooted scanへ移行し、65文字境界、正負整数、escape hash、parser formsの
  regression、Linux x86_64 fixed-pointを確認した。
  `vector-set-at` と `defn-signature-param-present-v3` の collection scanも 64 要素
  bounded/rooted continuationへ移行し、tokenizerが生成した64要素超のvectorで setter と
  signature presence の chunk境界を確認した。`selfhost_parser_collection_scanners` の
  3 tests、parser forms 22 tests、metadata forms 29 tests、Linux x86_64 fixed-pointを確認した。
  Parser の structural/recovery scan (`parse-skip-bracket-v3`、`parse-skip-brace-v3`、
  `scan-defn-param-form-end-v3`、delimiter balance、`recover-to-next`) も 64 要素
  bounded/rooted continuationへ移行し、129 token の bracket/brace skip、parameter span end、
  未閉鎖 delimiter code、recovery cursor を `selfhost_parser_structural_scanners` で確認した。
  parser forms 22 tests、metadata forms 29 tests、Linux x86_64 fixed-pointも通過した。
  `collect-example-expression-spans-v3-loop` も 64 要素 bounded/rooted continuationへ移行し、
  129 expression の `(start,end)` 順序と bracket boundary を
  `selfhost_parser_expression_spans` の2 testsで確認した。parser forms 22 tests、metadata forms
  29 tests、Linux x86_64 stage2/stage3 fixed-pointも通過した。
  `parse-defn-meta-case-loop-v3` と `parse-defn-meta-assert-loop-v3` も 64 要素
  bounded/rooted continuationへ移行し、65 case / 65 assert の件数と EOF cursorを
  `selfhost_parser_metadata_scanners` で確認した。metadata scanner 2 tests、metadata forms 29 tests、
  parser forms 22 tests、assertion span 4 tests、case span 5 tests、Linux x86_64 fixed-pointも通過した。
  `parse-source-evidence-shrinks-loop-v3` と `parse-source-evidence-coverage-loop-v3` も item単位の
  step、64要素 bounded loop、rooted continuation、public wrapperへ移行し、65 shrinks / 65 coverage
  entries の件数と EOF cursorを `selfhost_parser_metadata_sequences` で確認した。sequence scanner 2 tests、
  parser filter 114 tests（既知の projection baseline 1件を除く）、evidence registry 59 tests、Linux
  x86_64 fixed-pointも通過した。
  `parse-defn-param-signature-loop-v3` も `[done, next-index, next-signature]` stateを返す step、64要素
  bounded loop、rooted continuation、public wrapperへ移行し、65 typed paramsの先頭/末尾/return typeと
  signature countを `selfhost_parser_signature_scanners` で確認した。signature scanner 2 tests、metadata
  forms 29 tests、parser forms 22 tests、Linux x86_64 fixed-pointも通過した。
  `parse-defn-metadata-loop-v3` の各 directive handler も一 directive の metadata を返す single-step に分離し、
  outer loopを64 directive bounded/rooted continuationへ移行した。65個の `:property []` で metadata form
  件数65とEOF cursorを `selfhost_parser_metadata_outer_scanners` のstatic/runtime 2 testsで確認し、
  `selfhost_parser_` grouped filter 68 tests、Linux x86_64 stage2/stage3 fixed-pointを通過した。summaryは
  `ci-artifacts/native-linux-x86-hostgen-vm/7b7a4f24-parser-metadata-outer/actual-selfregen-summary.json` に保存し、
  stage2/stage3 code lengthは双方 `11297622`、stdout SHA-256は双方
  `6082b4494b46a2244b1d1e84c4faf3824f9b6ff664ca6859785acc8c73240a3c` で一致した。
  evidence/review metadataの field handler と field loopも、各 fieldを一つ処理する step、64 field
  bounded loop、rooted continuation、public wrapperへ移行した。65個の重複 `:subject` / `:subject-digest`
  fieldで evidence/review payload length `82` / `75` と body cursor `10` を
  `selfhost_parser_metadata_fields` のstatic/runtime 2 testsで確認し、`selfhost_parser_` grouped filter
  70 tests、scoped rustfmt、`git diff --check`、Linux x86_64 stage2/stage3 fixed-pointを通過した。summaryは
  `ci-artifacts/native-linux-x86-hostgen-vm/7f1e21d8-parser-metadata-fields/actual-selfregen-summary.json` に保存し、
  stage2/stage3 code lengthは双方 `11314848`、stdout SHA-256は双方
  `63466a74503a2e979f7bb805b8d99f91a848f883ea5c0f4124ac8ffdc0a288a7` で一致した。固定長の
  `source-evidence-seen-new-v3-loop`、required-field check、`source-review-attestation-seen-new-loop` は
  このbatchの対象外として残る。gate後に `origin/main` へ入った差分はMCP専用で、gate時の
  `7f1e21d8` と統合後の対象3ファイルは byte-for-byte 一致したため、同じLinux replayは重複実行していない。
  evidence/review metadataの fixed-size seen vector初期化と evidence required-field 判定も、各要素を一つ処理する
  step、64要素 bounded loop、rooted continuationへ移行した。evidence seen length `17`、空seenの required 判定
  `0`、required fieldを全て埋めた判定 `1`、review seen length `12` を
  `selfhost_parser_metadata_initializers` のstatic/runtime 2 testsで確認し、`selfhost_parser_` grouped filter
  72 tests、scoped rustfmt、`git diff --check`、Linux x86_64 stage2/stage3 fixed-pointを通過した。summaryは
  `ci-artifacts/native-linux-x86-hostgen-vm/0064a5bb-parser-metadata-initializers/actual-selfregen-summary.json` に保存し、
  stage2/stage3 code lengthは双方 `11332908`、stdout SHA-256は双方
  `aa5cee91b5f47dd54a7da64492859bb1b9eede381059051713e85310115ba7ad` で一致した。
  2026-08-01 に `test_e2e_selfhost_typeinfer_record_pattern_uses_declared_field_type` の
  fixtureを現在の visibility contractへ修正した。空の value envに schema registryだけを渡していた
  ための `1` vs `0` は selfhost実装の未対応ではなく、constructor visibilityを表現していない
  test gapだった。可視 constructor scheme経由の field binding成功と、同じ registryだけを持つ
  非可視経路の fail-closedを同一fixtureで固定し、quote/pattern 13 tests と private visibility
  1 testが passした。selfhost source bytesは変更していないため、直近の current-source native
  fixed-point evidenceを再利用する。record schema patternのsemantic parity、全 pattern、import
  target、Rust ABI parityは actual E2Eで引き続き閉じる。
  2026-08-01 に parametric record application (`Box a`) を含む nested record pattern の
  field bindingを追加した。Rust側は `record_registry` に登録済みの `Type::App` だけを
  parameter substitution付き `Type::Record` へ materializeし、selfhost側は record patternの
  field boundaryごとに同じ変換を遅延適用する。Rust focused test、nested E2E 1件、既存の
  `selfhost_typeinfer_quote_patterns` 15件が passした。Mac Apple Silicon native gateは
  source commit `fa97fa948489f635dc8888b5a269755a75776670` の 1 test pass（artifact 4660 KiB）、
  Linux x86_64 hostgen/VM gateも status `pass` で、stage2/stage3 code length 各 `11332908`、
  stdout SHA-256 は両方 `aa5cee91b5f47dd54a7da64492859bb1b9eede381059051713e85310115ba7ad`
  で一致した。追加の `test_e2e_selfhost_compiler_mode_nested_parametric_record_pattern_runs`
  は selfhost compiler-mode の実 Wasm runtime で binder/literal/fallback の `41\n1\n7\n` を
  確認した（test-only追加のため native source producerは不変）。さらに imported source
  compiler-mode と ftable alias-qualified の同じ fixtureも passした。これは nested parametric
  pattern、import、source/ftable runtime fixtureの verified sliceであり、arbitrarily deep
  patternについては4-record chain (`Root a -> Outer a -> Middle a -> Box a`) の source runtime
  regressionまで確認し、同じ4段を ftable runtimeでも確認した。それを超える深さ、full
  ftable/linear-memory ABI、`LEGACY-LANG-01` aggregateは残る。さらに Rust WasmGC backendの
  `TypeExpr::App` named headを登録済みrecordの `Ref` へ解決する狭い修正を行い、IRの
  `Outer.inner = Ref(Box)` と Wasmtimeでの nested pattern実行結果 `41` を確認した。
  これはRust IR/emitter backendのverified sliceであり、native stage0 producer parityや
  Rust-free selfhostの証拠には拡大しない。
- [~] `LEGACY-LANG-02` ADT/GADT execution parity — ordinary ADT の direct/nested constructor と
  GADT parser/type refinement は verified。2026-07-31 に selfhost
  `Types.TypeInferAdt` の type parameter、constructor field、variant、type declaration scan を
  64 要素単位の bounded rooted continuationへ揃え、65 要素で chunk 境界を跨ぐ focused E2E
  (`selfhost_typeinfer_adt_scanners`) を通した。Linux x86_64 fixed point は status `pass`、
  stage2/stage3 code length 各 `11168596`、stdout SHA-256 は両方
  `dad391cd36df64b6354b1f4429aaf7a4c410697b7ca74606fbb2865dc2186bb1` で一致した。
  これは ordinary ADT の bounded type-inference traversal と current-source Linux native
  self-regeneration の verified sliceであり、nominal/exhaustiveness、full ftable/import、
  linear-memory/WasmGC runtime parity、Mac/Linux aggregate は残る。2026-08-01 に Rust
  WasmGC backendの non-parametric `Option (Some Int) None` を実際に lower/emitter/Wasmtime
  で実行し、`Some` の typed payload、variant tag、`None` fallbackを `42` の結果で確認した。
  これはRust IR/emitter backendの verified sliceであり、parametric ADT、nominal/exhaustiveness、
  full ftable/import、native stage0 producer parityの完了を意味しない。
  さらに `test_e2e_selfhost_compiler_mode_imported_adt_constructor_pattern_runs` で、別ファイルの
  `App.Shapes` に定義した parameterized `Maybe a` と `App.Main` の
  `(import App.Shapes :open :only [Just Nothing])` を selfhost compiler-mode でコンパイルし、生成
  Wasm を実行して `41\n0\n` を確認した。これは source-file import、constructor export filtering、constructor
  pattern binder/fallback、生成 artifact/runtime を同じ fixtureで閉じる verified sliceである。
  さらに `(import App.Shapes :as S :only [Just Nothing])` と `S.Just` / `S.Nothing` の construction・pattern
  を同じ selfhost compiler-mode fixtureで実行し、alias-qualified 側も `41\n0\n` を確認した。Parserは
  型推論用の full constructor hashを維持し、constructor childの後ろへ raw suffix hashを保持し、Wasm
  tag checkだけが raw suffixを使う。open/alias-qualified の2 testsを同じ focused invocationで実行し、
  2 passed（101.19s）となった。これは source-file importと CompilerMode prelude ftableの alias registration、
  constructor call、pattern binder/fallback、生成 artifact/runtimeを閉じる verified sliceである。
  flat ftable compilerの同じ import target、module-qualified multi-segment name、parametric/recursive ADT の広い形、
  nominal/exhaustiveness、両 supported target の current-source native stage0 parity、
  `LEGACY-LANG-02` aggregateは残る。selfhost sourceとtestの変更だが、既存の native producer evidence は拡大していない。
  2026-08-01 に `test_e2e_selfhost_typeinfer_analysis_filters_import_alias_only_adt_constructor_pattern` を追加し、
  `(import Lib :as L :only [Some])` の `L.Some` pattern を selected `0` diagnostics、除外された
  `L.Other` pattern を `1` diagnostic として selfhost TypeInfer bundle で確認した。Rust oracle の
  `Pattern::Constructor` も式と同じ alias/`:only` qualified resolution を使うように揃え、selected を受理し、
  excluded を拒否することを確認した。これは parser/type-infer の alias visibility sliceであり、native
  stage0 producer、flat ftable、Wasm runtime、両 supported target の parity evidenceは広げない。
  さらに flat ftable compiler で `Left.Thing` と `Right.Thing` を別 module の同名 constructor として
  `:as L` / `:as R` から構築・pattern matchし、`41\n5\n` を actual Wasm runtime で確認した。
  `test_e2e_selfhost_ftable_compiler_imported_alias_qualified_same_name_adt_constructors_run` は alias
  keyから各 constructorの異なる function targetを解決する verified sliceである。併せて source-awareと
  ftableの nested ADT が `41\n7\n42\n`、`Packed(Packed(Node ...))` の深い pattern が両経路で
  `42\n` になることを確認し、`pattern-temp-base` と map opcodeの6-local strideで再帰 childの
  scratch/binder localを分離した（`selfhost_pattern_scratch_contract`）。これらは flat ftableの同名
  alias target分離と recursive pattern scratch の verified sliceに限られ、file-import ftable parity、
  module-qualified multi-segment name、parametric/recursive ADT全体、nominal/exhaustiveness、
  WasmGC/linear-memory ABI、両 supported targetの current-source native stage0、
  `LEGACY-LANG-02` aggregateは残る。selfhost source/actual Wasm E2Eの証拠であり、native producer
  evidenceや Rust-free aggregateを拡大しない。
  2026-08-01 に `test_e2e_selfhost_compiler_mode_imported_same_name_adt_aliases_run` を追加し、
  実ファイル `App.Left` / `App.Right` の同名 `Thing` constructorを `L` / `R` aliasで importした
  `App.Main` を `compile-file-mode` から生成・実行して `41\n5\n` を確認した。これは source-file
  importと CompilerMode の prelude ftable alias registrationが、同名 constructorの function targetを
  混同しない verified sliceである。flat `program-functions-base` の file-import transport、
  module-qualified multi-segment name、parametric/recursive ADT全体、nominal/exhaustiveness、
  両 supported targetの current-source native stage0 parity、`LEGACY-LANG-02` aggregateは残る。
- [~] `LEGACY-COMP-01` full-program compiler closure — 主要 CLI builder は full-program 化済み。
  `TypeInferBlock.ls` の大きな do/computation 子要素走査は 64 要素 bounded/rooted scanへ移行し、
  Linux x86_64 stage2/stage3 fixed-pointを確認した。full-program compiler closure、
  diagnostic-only legacy `lower`、no-arg pipeline runtime/native E2E、component sidecar の
  artifact boundary を閉じる。2026-08-02 に
  `test_e2e_selfhost_cli_main_compile_and_build_output_actual_preview1_wasm` で actual `Cli`
  bundleの `compile` / `build` が実 Preview1 Wasmを出力し、Wasm validationとstandalone runtimeを
  通過することを確認した。`wasi-component` は外部 packaging未接続として exit `1`、artifactなしを
  維持した。これは Rust-host actual artifact/runtime の verified sliceであり、native stage0の
  full-program entrypoint、component sidecar、両 supported targetのartifact/runtimeは残る。
  同日、Mac Apple Silicon current-source native release gateから `App.Cli` の `program.native` を
  生成し、manifestの `target=aarch64-apple-darwin`、`source_commit=0dc6d673...`、
  `selfhost_fixed_point=true`、program SHA-256、4,327,168 bytesを確認した。artifact単体の
  no-arg helpと `parse` file postflightも exit `0` / stderr `0` で通過した。Linux x86_64 native
  entrypoint、component sidecar、両 targetの full-program parityは残る。
- [~] `V2-16b` / `LEGACY-IO-01` native artifact I/O — bounded argv/file/raw-byte Preview1 と
  4096 bytes 超 read の slice は verified。`valid/io-read-file` は manifest の明示的な
  UTF-8 runtime input snapshot と task-owned preopen まで Rust oracle/native producer contract
  で固定した。`valid/io-read-file-empty` は zero-byte file と EOF の区別、
  `valid/io-read-file-missing` は明示的な空 directory と missing-path fd error の fail-closed 境界、
  `valid/io-read-stdin` は明示的な UTF-8 stdin snapshot と producer 境界を固定する。
  全 fd error semantics、dynamic root/data/heap layout、component sidecar、target 別
  release artifact を閉じる。2026-08-02 に actual `Cli` の `compile` / `build` output gateで
  generated Preview1 Wasmの magic、validator、standalone runtimeを確認し、component targetの
  external packaging拒否も確認した。これは Rust-host source bundleの artifact I/O verified sliceであり、
  native stage0の output path、fd error semantics、component sidecar、両 supported targetの
  release artifactは残る。

  2026-08-02 の verified partialとして、`read-stdin` を native selfhost compiler の builtin/opcode `91` に固定し、
  x86_64 helper（104 bytes）と AArch64 helper（156 bytes）、nullary stack effect、runtime call target、helper trailer
  offsetを追加した。`scripts/ci/test-native-selfhost-read-stdin-contract.sh` の RED→GREENで、source-level `Cli` の
  `run-lsp-stdio-server` が同じ builtinで wire inputを読む接続も固定し、x86/AArch64 helperは独立 disassemblyで確認した。
  source commit `1ee26eef38fe6f32ac1f6a1e7342bcf8cb1fec41` の Mac actual `App.Cli` release gateは `1 passed` /
  `877.69s`、packageは `aarch64-apple-darwin` / Mach-O arm64だった。同packageの current source-file smokeも
  parse/check/fmt/test/metadata test/compile/buildとvalidation positive/negativeをexit `0`で通過し、compile/build
  Wasmは各 `2,559` bytes、SHA-256 `afd1638e444a7e8c371dc1d17550479fcc5e4efbbb9e9dbdffa8551933d71a00`、
  evidenceはstage0 manifest SHA-256 `fd1f47bd7a61e0f45bd2b8d086021fe0b919e3d75de172c82102b0e653518dd7`、
  payload SHA-256 `36f7cd4e27c58bb23b299eca5d5f0d1be266b821b1f1e9708d3a9b8fca70b5c9`を記録した。成功経路では
  `cargo`、`rustc`、host `lsharp`、Rust fallbackを使用していない。
  同packageの `lsp --stdio` はstdin wireを読み、stderr空、`Content-Length` frameを返すところまで実native確認したが、
  initialize resultのselfhost配列表現とRust LSP object shape、hover等のsemantic projectionは未完である。また
  `valid/io-read-stdin` の生成 Preview1 Wasm compileはnative CLIの明示的な `unsupported standalone Preview1 runtime capability`
  境界で拒否され、Wasm runtime証拠にはしていない。Linux x86_64 current HEADの同opcode stage0/package/runtime、
  component sidecar、release asset acquisition/rollback、Mac/Linux packaged parityは未検証のため、`V2-16b` /
  `V2-16c` / `V2-16e` と aggregateは `[~]` のまま残す。

  さらに 2026-08-03 の current Linux x86_64 verified partialとして、clean `HEAD=113d3785a54e0d4af0bc970edfe45c234a96449d`
  から read-stdin helperを含む actual stage1を `302.68s` で生成し、manifestの target、source commit、code `4,408,352`
  bytes、data `2,757` bytes、entrypoint `4,405,792`、function-start `3,418`、main function `3,427` を確認した。
  stage1 artifactを `lsharp-native-selfhost-stage0` packageへ変換し、target `x86_64-unknown-linux-gnu`、compiler、transport
  driver、materializer、source commitを確認した。hostgen実行時に source commit env が未指定だったため、生成byte列と clean HEADの
  一致を確認した後に保存 manifestの provenance fieldだけを補正している。これは source commitの実行時検証を省略した証拠ではないが、
  artifact manifestの生成時自動記録としては扱わない。
  同packageの current-source native stage0 source-file smokeは exit `0` で、parse/check/fmt、通常/metadata test、compile/build、
  validation positive/negativeを通過し、`compile.wasm` / `build.wasm` は各 `2,559` bytes、SHA-256
  `afd1638e444a7e8c371dc1d17550479fcc5e4efbbb9e9dbdffa8551933d71a00`、stage0 manifest/payload SHA-256はそれぞれ
  `8f05341f559400a6df0de7c73cc357dcdd3caa8f5ebaf280888923a0c4522dd6` / `f7d152eccbe02ff4bcf10bd8f48494a1d8fef2b6c94b5a58a795324ae0b30dda6`
  だった。成功経路では `cargo`、`rustc`、host `lsharp`、Rust fallbackをblocklistしている。
  同じ current-source stage1を再利用した stage2→stage3 transport/materialize/compareは status `pass`、stage2/stage3 manifestの
  source commitとtargetが一致し、code length各 `11,408,204`、stdout各 `1,434,730` lines、stdout SHA-256各
  `b9569a6202412633e2ff258a6988bdd5d9556e1b354ab8fe203b67b5f511b26a`、stderr各 `0` bytesで一致した。stage2 seedの
  `int-to-string` / `string-concat`経路と `42` の tiny output markerは stage2/stage3双方で一致したが、これは standalone Preview1
  `io-read-stdin` runtimeやLSP semantic parityの証拠ではない。Linux direct LSP JSON semantic projection、component sidecar、
  公開 release asset acquisition/rollback、Mac/Linux packaged artifact parityは残る。

  2026-08-02 の current-source LSP projection batchでは、clean `HEAD=5db1c2a4f5147469f24eaca97976e2e62cfb6455` の
  Mac Apple Silicon actual `App.Cli` release gateを一度だけ実行し、`1 passed` / `836.52s`、manifest target
  `aarch64-apple-darwin`、`selfhost_fixed_point=true`、Mach-O arm64、program SHA-256
  `4b2650908f2e55037ab6e76a6129aed038133e78839a135525fb9f03be1ff4d8`、artifact `4,676 KiB` を確認した。
  同artifactの direct `lsp --stdio` wire batchは exit `0`、stderr `0` bytesで、initialize capabilities object、hover range object、
  completion item object、formatting `TextEdit` objectを返した。Linux x86_64でも同じ source commitの stage1/package/source-file
  smokeを再生成・実行し、stage1 manifestは code `4,408,352` bytes、data `2,757` bytes、entrypoint `4,405,792`、
  function-start `3,418`、main function `3,427`、package/source-smoke manifestとtarget/source commitが一致した。
  同packageをVM内の `native-selfhost-dev.sh lsp --stdio` に渡した direct wire batchも exit `0`、stderr `0` bytesで、同じ4種の
  object projectionを確認した。stage2→stage3 fixed pointは status `pass`、code length各 `11,408,204`、stdout各 `12,249,104`
  bytes、SHA-256各 `b9569a6202412633e2ff258a6988bdd5d9556e1b354ab8fe203b67b5f511b26a`で一致した。これは4 operationの
  current-source native projectionを閉じる verified sliceであり、definition/references/renameのURI/location/workspace edit、
  diagnosticsの標準Diagnostic shape、position base、component sidecar、公開 release asset acquisition/rollback、
  Mac/Linux packaged artifact provenance parityの完了証拠ではない。Lima VMは検証後に停止し、task-owned workdirを削除した。
  同じ current Mac `program.native` に `native-selfhost-component.py --command compile` を渡した実native boundaryでは、
  `wasm-tools` が `wasi_snapshot_preview1::fd_write` importを解決できず exit `1`、stderrに component encode failureを返し、
  component artifactを作成しなかった。この明示的外部 packaging拒否は正しい fail-closed boundaryだが、component sidecarの
  Rust-free実装完了やstandalone component runtimeの証拠ではない。

  さらに current `HEAD=9175c6e50f4a6845ae97836b3ac6897102f3dd52` で LSP wire positionを更新した。Mac Apple Siliconの
  actual `App.Cli` release gateは `1 passed`、manifestは target `aarch64-apple-darwin`、`selfhost_fixed_point=true`、program
  SHA-256 `8106ebcb373da7d4b4183ee23b3a87b423afce5e2300e956fdb8915031865d18`、artifact `4,676 KiB` を記録した。同artifactの
  direct `lsp --stdio` batchは exit `0` / stderr空で、numeric URIの hover が `line=0,character=6` から `line=0,character=12`、
  formatting が `line=0,character=0` から `line=1,character=3` の zero-based wire rangeを返した。Linux x86_64では同じ current
  sourceから stage1を `287.20s` で fresh生成し、manifestは code `4,408,352` bytes、data `2,757` bytes、entrypoint `4,405,792`、
  function-start `3,418`、main function `3,427`、source commit/target一致だった。package/source-file smokeは exit `0`、compile/build
  は各 `2,559` bytes、SHA-256 `afd1638e444a7e8c371dc1d17550479fcc5e4efbbb9e9dbdffa8551933d71a00` で、同stage1を再利用した
  stage2→stage3 transport/materialize/compareは status `pass`、code length各 `11,408,204`、stdout各 `12,249,104` bytes、
  SHA-256各 `b9569a6202412633e2ff258a6988bdd5d9556e1b354ab8fe203b67b5f511b26a`、stderr各 `0` bytesで一致した。Linux direct LSPの
  semantic projection自体はこのbatchでは再実行していないため、definition/references/renameのURI/location/workspace edit、
  diagnostics shape、component sidecar、公開 release asset acquisition/rollback、Mac/Linux packaged artifact parityは残る。

  さらに `79bde035095e71a2baa85532c0430a9d76ea2ca7` で、標準 nested LSP params の string URI を state に保持し、definition /
  references を URI付き `Location` object、renameを URI-keyed `WorkspaceEdit` へ投影する host-Wasm verified partialを追加した。
  RED は `test_e2e_selfhost_cli_lsp_stdio_standard_uri_navigation_contract` で、旧実装が integer URI と配列 locationを返すことを確認し、
  GREEN は同じ実際の `lsp --stdio` fixtureで `1 passed` / `319.24s` となった。URI hashの負値を非負keyへ正規化し、standard `didOpen`
  の `[uri, source, path, uri-text]` parameter shapeを固定したため、state-aware rendererは wire URIをJSON stringとして再利用する。
  `test_e2e_selfhost_lsp_state_preserves_wire_uri` と LSP docs/ops focused group（22 passed / 23 ignored）、
  `scripts/ci/test-native-selfhost-lsp-stdio.py`（5 passed）も確認した。numeric URIは既存互換のlegacy projectionを維持する。
  definition/referencesの内部 location は現状 point range、open stateに実URIがない imported filesystem locationは
  `lsharp://document/<id>` fallbackとなるため、full symbol range、cross-document URI provenance、diagnosticsの標準Diagnostic shapeは残る。
  current `HEAD=79bde035` のMac/Linux native artifactをこのURI変更込みで再生成したdirect LSP gateは未実行であり、直前の `9175c6e5`
  native evidenceをcurrent evidenceへ拡大解釈しない。`V2-16b` / `V2-16c` / `V2-16e` と aggregateは `[~]` のまま残す。

  さらに `6f0238c240d12e5e0f4bbb31dc9ff75b774b4535` で、didOpen/didChange後に自動追加する diagnostics refreshも保存済みの wire URIを
  使うよう修正した。REDの `test_e2e_selfhost_lsp_transport_diagnostics_preserves_wire_uri` は didOpenのURIだけが stringで、後続
  `publishDiagnostics` が numeric hashになる failureを確認し、GREENは同じ実際の `App.Cli` bundleで `1 passed` / `318.71s`、
  `test_e2e_selfhost_lsp_state_preserves_wire_uri` は `1 passed` / `9.13s`、native LSP shimは `5 passed` となった。これは diagnostics
  refreshのURI保持に限定した verified partialで、標準Diagnostic fields、cross-document provenance、current-source Mac/Linux native
  artifact gate、component/release parityは残る。`V2-16b` / `V2-16c` / `V2-16e` と aggregateは `[~]` のまま維持する。

  さらに 2026-08-03 の clean `HEAD=d6517499b6b00287e901b278b1f549d56cc4fc2c` で、Mac Apple Siliconの
  `native-macos-aarch64-selfhost-release.sh` を一度だけ実行し、current `App.Cli` native releaseを再生成した。
  gateは `1 passed` / `941.46s`、manifestは target `aarch64-apple-darwin`、同じ source commit、
  `selfhost_fixed_point=true`、program SHA-256 `ee884e4828e6263830e3b3d7908ad85444b012130e67f0d068b2bd360f5c072d`、
  Mach-O arm64、program `4,343,680` bytesを記録した。同artifactの標準 string URI fixtureによる direct
  `lsp --stdio` は exit `0` / stderr `0` bytes / 5 framesで、didOpen後の diagnostics refresh、definition / referencesの
  `Location`、renameの URI-keyed `WorkspaceEdit` がすべて `file:///tmp/lsharp-uri-contract.ls` を保持した。
  artifactは `ci-artifacts/native-release/aarch64-apple-darwin/current-d6517499-lsp-diagnostics/` に保持した。

  同じ clean HEADから、Linux x86_64 hostgen→Lima VM gateも一度だけ実行した。actual stage1 manifestと stage2/stage3
  debug manifestは target `x86_64-unknown-linux-gnu`、source commit `d6517499...` が一致し、stage1 code/dataは
  `4,408,352` / `2,757` bytes、entrypoint `4,405,792`、function-start `3,418`、main function `3,427` だった。
  actual self-regeneration summaryは `status=pass`、stage2/stage3 code lengthは各 `11,408,204` bytes、transport stdoutは
  各 `12,249,104` bytes / SHA-256 `b9569a6202412633e2ff258a6988bdd5d9556e1b354ab8fe203b67b5f511b26a`、stderrは各 `0` bytesで一致した。
  これは Linux stage2→stage3 transport/materialize/fixed-pointの current-source evidenceであり、Linux target-only
  App.Cli release binaryやLinux direct LSP semantic projectionの evidenceではない。後者、標準 Diagnostic fields、
  cross-document URI provenance、component sidecar、公開 release asset acquisition/rollback、Mac/Linux packaged
  provenance parityは残る。Lima `lsharp-linux-x86` は検証後に停止し、task-owned VM workdirと replay lockは回収した。

  さらに current `HEAD=95656144` で parse-only diagnostics の標準 LSP Diagnostic projectionを RED→GREENで固定した。
  `test_e2e_selfhost_lsp_render_standard_parse_diagnostic_json` は enriched recordから zero-based range、severity、`LS0101`、
  `source="lsharp"`、messageを標準 objectへ投影し、legacy renderer/snapshotを含む render groupは `5 passed` / `17.05s`。
  実際の `App.Cli` bundleで `lsp-source-parse-diagnostics ")"` を通す
  `test_e2e_selfhost_lsp_parse_diagnostics_use_standard_projection` は `1 passed` / `321.40s` となり、
  `scripts/ci/test-native-selfhost-lsp-stdio.py` は `5 passed` だった。parse-only の producer/renderer境界は
  host-Wasmで verified partialになったが、type/lint diagnosticsの標準 fields、複数診断の全文、full span/code/message parity、
  current HEADのMac/Linux native artifact direct gate、cross-document URI provenance、component sidecar、公開 release asset
  acquisition/rollback、Mac/Linux packaged provenance parity、LSP aggregateは残る。`V2-16b` / `V2-16c` / `V2-16e` と aggregateは
  `[~]` のまま維持する。

  その後 current `HEAD=ab5122c18786f82d6277f36bbddaf6b31ff98f16` で Mac Apple Siliconの
  `native-macos-aarch64-selfhost-release.sh` を一度だけ実行し、`1 passed` / `922.36s`、target
  `aarch64-apple-darwin`、`selfhost_fixed_point=true`、Mach-O arm64、program `4,360,192` bytes、program SHA-256
  `d66c271e96545b0a2310424ee8a4f12c01bc56d1ad7180db154f53a9dc10d0e6` を確認した。artifactは
  `ci-artifacts/native-release/aarch64-apple-darwin/current-ab5122c1-lsp-parse/` に保持した。同artifactへ invalid parseの
  `lsp --stdio` wireを直接渡し、publishDiagnosticsの標準 `range` / severity / `LS0101` / source / messageと URIを
  一致確認した。Linux x86_64の current-source hostgen→Lima VM gateも同じ source commitで exit `0` となり、stage1 manifestは
  target `x86_64-unknown-linux-gnu`、code/data `4,408,352` / `2,757` bytes、entrypoint `4,405,792`、function-start
  `3,418`、main function `3,427`。stage2/stage3 manifestの target/source commitが一致し、code length各 `11,408,204`、
  stdout各 `12,249,104` bytes / SHA-256 `b9569a6202412633e2ff258a6988bdd5d9556e1b354ab8fe203b67b5f511b26a`、
  stderr各 `0` bytes、summary `status=pass` で一致した。artifactは
  `ci-artifacts/native-linux-x86-hostgen-vm/current-ab5122c1-lsp-parse/` に保持し、VM workdir/lockを削除、Limaは停止した。
  これは parse-only標準 DiagnosticのMac native direct evidenceとLinux stage2/stage3 fixed-pointを追加する verified partialであり、
  type/lint diagnosticsの標準 fields、複数診断の全文、full span/code/message parity、Linux direct LSP semantic projection、
  cross-document URI provenance、component sidecar、公開 release asset acquisition/rollback、Mac/Linux packaged provenance parity、
  LSP aggregateは残る。`V2-16b` / `V2-16c` / `V2-16e` と aggregateは `[~]` のまま維持する。

  さらに 2026-08-04 の `d2dcea7e135ba839c41cb2f29e416b91b2993d72` batchで、standalone command-line runtimeのREDを
  `test_e2e_selfhost_standalone_command_line_runtime` に追加した。`alpha` / `beta` を渡した場合は `alphabeta2\n` となったが、
  strictなargc=0では、Wasm emitterの整数ゼロ処理が符号分岐内に閉じていたため改行だけになった。Rustのcanonical
  `print_i64` と比較して、ゼロを符号分岐の後で1桁出力する narrow fixを selfhost `WasmEmit.ls` に追加した。
  Rust focused E2Eは current-source cache生成後 `1 passed` / `1.27s`（初回生成 `317.69s`）で、保存Wasmの
  `wasm-tools validate` と disassemblyでもゼロ桁の命令列を確認した。広い append helper、spill floor、offset-depth
  ctx/state refactorは導入していない。

  同じ source treeの Mac Apple Silicon actual `App.Cli` release gateは target `aarch64-apple-darwin`、source commit
  `d2dcea7e...`、`selfhost_fixed_point=true`、program SHA-256
  `f6e63869a8ea69d3ff6177639454f229dec63a18ae51ef82b1f8c8fe1e80a9ec`、`1 passed` / `825.50s` となった。
  同artifactの native I/O matrixは `11 cases` 全 passで、`print-zero` を含めた。Linux x86_64では current `HEAD=eb8086a8d5bf3bb5893102a4d692ff8aa1a058ef`
  の actual stage1 -> stage2 -> stage3 fixed pointが status `pass`、target `x86_64-unknown-linux-gnu`、stage2/stage3 code length各
  `11,442,429`、stdout SHA-256各 `2526caaefa9e86b934d5d08eb800847ac96e6b3989f3c3c37c7d2c933516086e` で一致した。
  VM free-space gateは `7,678,435,328` / `4,294,967,296` bytesで passし、成功後に workdir/lockを回収してVMを停止した。
  stage2を再利用した Linux `App.Cli` target-only manifestは source commit `eb8086a8...`、source tree SHA-256
  `b79ae7dca8f81b661355f5f4cae7f872dae4150012b1111a7a849dfd4fb514d1`、`selfhost_fixed_point=true`、code `13,367,992` bytes、
  program SHA-256 `af9c7944db08acf1ccd966c2ac40fe1162cc0f56e0bfed1599dc3d0a6597d537`、stderr `0`で passした。
  Linux native I/O matrixも `11 cases` 全 passで、native成功経路に `cargo`、`rustc`、host `lsharp`、Rust fallbackは入っていない。
  matrixの非空argvは Wasmtime CLIへ `--argv0` を明示して Rust runnerの `alpha` / `beta` と一致させた。一方、外部CLIはargc=0を表現できないため、
  empty-argv0 caseは `argc=1` の境界（出力 `1\n`）として分離し、strict argc=0の証拠はRust standalone E2Eに残した。
  これは standalone command-line/zero-printの両target runtime verified partialであり、fd error/EOFの全semantics、より大きい入力での
  dynamic root/data/heap layout、完全なargv/command-line semantics、全公開command、component sidecar、release asset acquisition/rollback、
  Mac/Linux packaged provenance parity、Rust-free aggregateは未完了である。`V2-16b` / `LEGACY-IO-01` は `[~]` のまま維持する。

  続く `9b7ac735b...` では、既存の native I/O matrixを `11` から `15 cases` へまとめて拡張し、`read-file` の通常入力、zero-byte file、
  4097-byte file、missing-pathを追加した。Mac Apple Siliconの既存 `d2dcea7e...` App.Cli artifactと、Linux x86_64の既存
  `eb8086a8...` target-only artifactを再利用し、Mac/Linuxとも `15 cases` 全 passした。Linux側は Wasmtime `43.0.0` をVM内へ一時配置し、
  `program.native` は ELF `x86-64`、target `x86_64-unknown-linux-gnu`、program SHA-256 `af9c7944db08acf1ccd966c2ac40fe1162cc0f56e0bfed1599dc3d0a6597d537`
  のままである。runner-onlyのテスト拡張であり、重い stage1再生成は再実行していない。native成功経路に `cargo`、`rustc`、host `lsharp`、
  Rust fallbackは入っていない。これは normal/empty/large/missing read-fileの両target runtime verified partialを追加するが、fd_read/fd_close/path_open
  errno注入のnative matrix、EOF/errorの全組合せ、dynamic root/data/heap layout、完全なargv/command-line semantics、全公開command、component sidecar、
  release asset acquisition/rollback、Mac/Linux packaged provenance parity、Rust-free aggregateは未完了である。`V2-16b` / `LEGACY-IO-01` は `[~]` のまま維持する。
- [~] `V2-16c` / `LEGACY-TOOL-01` public command closure — `install` / `repl` / `lsp --stdio` /
  `doc` / component helper の routing contract は verified。`install` は実 installer helper を
  fake stage0 から public runner 経由で呼び、path dependency、lockfile、module-index、cargo/host
  `lsharp` fallback 不使用まで integration test で確認した。実 stage0 と外部 tool の E2E、
  Rust-only flag/target の明示境界、target 別 release evidence を閉じる。ADR:
  `docs/adr/decisions-v0.3-native-install-runner-e2e.md`。2026-08-02 に
  `test_e2e_selfhost_cli_main_with_args_check_file` の ignoreを外し、actual `Cli` Wasm bundleを
  `check input.ls` argvで実行して `Int` / `diagnostics:0` を `491.23s` で確認した。前段で
  `Cli.ls` の JSON test / validation option formに不足していた閉じ括弧2箇所を修正し、
  `Tools.Validation.ManifestInput` の source path・embedded module・bundle inventoryを揃えた。
  これは Rust hostが生成・実行する actual CLI source bundle の verified sliceであり、native
  stage0 の `check`、install/repl/lsp/doc/componentの実stage0 E2E、Rust-only flag/target境界、
  両 supported targetのrelease evidence、`LEGACY-TOOL-01` aggregateは残る。
  同日、`test_e2e_selfhost_cli_main_no_args_shows_help` の ignoreを外し、actual `Cli` bundleを
  引数なしで実行して `Usage: lsharp <command>` / `Commands:` の help surface と成功終了を
  `447.61s` で確認した。これは no-arg dispatch と help serialization の Rust-host verified sliceであり、
  native stage0の no-arg parity、他の公開 command、両 supported targetのrelease evidence、
  `LEGACY-TOOL-01` aggregateは残る。
  さらに `test_e2e_selfhost_cli_main_batched_version_and_parse_argv` で、同じ actual bundleから
  `--version` / `-v` の `lsharp 0.1.0` と `parse input.ls` の `decls:1` / `diagnostics:0` を
  1回の compile と複数 argv 実行にまとめ、`459.57s` で確認した。これは version alias と
  parse file routingの Rust-host verified sliceであり、native stage0、他の公開 command、
  両 supported targetのrelease evidence、`LEGACY-TOOL-01` aggregateは残る。
  Mac Apple Siliconの current-source `program.native` でも no-arg helpと `parse` fileを postflight
  実行し、exit `0` / stderr `0` を確認した。ただしこれは単一 release artifactの smokeであり、
  Linux x86_64 stage0の `check` / `parse`、全公開 commandの native matrix、release provenanceは残る。
  さらに current `HEAD=3f6c49976f75a5099d524f08ea85cc1698935cbb` から、
  `scripts/ci/native-macos-aarch64-selfhost-release.sh` を一度だけ実行して App.Cli native releaseを再生成した。
  manifestは target `aarch64-apple-darwin`、同じ `source_commit`、`selfhost_fixed_point=true`、program SHA-256
  `a1dac9ff7146fbfd012c6e299df786c3c6c00680e3849cfb98abdeb1efcd76de` を記録し、生成物は Mach-O arm64だった。
  `--version` は `lsharp 0.1.0`、stdout 12 bytes、stderr 0 bytes、E2Eは `1 passed` / `896.66s` で成功した。
  artifactは ignored な `ci-artifacts/native-release/aarch64-apple-darwin/current-3f6c4997/` に保持した。
  これは current-source Mac App.Cli fixed-point/native runtimeの verified sliceであり、Linux x86_64の同一
  current-source gate、native stage0 compiler/package、全公開 command、release acquisition/rollback、両 targetの
  provenance parity、`LEGACY-BOOT-01` aggregateの完了証拠ではない。

  2026-08-02 の current `HEAD=5db1c2a4f5147469f24eaca97976e2e62cfb6455` では、Mac/Linuxの両 supported targetで
  `lsp --stdio` の initialize/hover/completion/formatting wire projectionを実native確認した。Macは actual release
  `program.native`、Linuxは current-source stage0 packageをVM内の `native-selfhost-dev.sh` から実行し、両方とも
  exit `0`、stderr空、object-shaped responseを返した。これは public command closure の LSP routing/projection verified
  sliceを広げるが、全公開 command、definition/references/rename/diagnosticsのsemantic parity、component sidecar、
  release acquisition/rollback、両 targetの packaged provenance、`LEGACY-TOOL-01` aggregateは残る。

  さらに `86534799` で、既存の current-source native App.Cli artifactを直接起動する core CLI runtime matrixを `22 cases` へ拡張した。
  `--help`、`--version`（`lsharp 0.1.0`、改行なし）、`parse`、`check`、`fmt`、text/metadata `test`、`compile`、`build` に加えて、
  `review`/`review --json`、`doc`/`doc --json`/`doc --format json`、`doc-ack`/`--trailer`、`doc-check`/`--strict`、`install`、`repl`、
  bare `lsp` summary、実際の `lsp --stdio` initialize→didOpen→hover wire、`doc --format yaml` の明示拒否を同じ scriptで固定した。
  Mac Apple Siliconの `d2dcea7e...` artifactとLinux x86_64の `eb8086a8...` target-only artifactで各 `22 cases` 全 passした。Linux
  programは target `x86_64-unknown-linux-gnu`、program SHA-256 `af9c7944db08acf1ccd966c2ac40fe1162cc0f56e0bfed1599dc3d0a6597d537`
  の ELFであり、成功経路に `cargo`、`rustc`、host `lsharp`、Rust fallbackは入っていない。runner-onlyの contract testなので、
  stage1/stage2/stage3 replayは重複実行していない。これは public command routingと代表的な doc/REPL/LSP wireの両target verified partialを
  追加するが、実 install/package registry、対話 REPL、LSP の全 semantic projection、component helper、全公開 command、stage0 package
  acquisition/release/rollback、Mac/Linux packaged provenance parity、`LEGACY-TOOL-01` aggregateは未完了である。
  さらに `ad65eaff` の Linux x86_64 `App.Cli` target-only native programへ同じ core runnerを渡し、`22 cases` 全 passを確認した。
  `--help`、`--version`、parse/check/fmt/test、Preview1 compile/build、review/doc/doc-ack/doc-check、install/repl、bare LSP、
  `lsp --stdio` hover、unsupported doc formatを含むが、保存 artifactは現HEADの test/docs commitとは一致しないため replay-onlyである。
  native source provenance、実 install/package registry、対話 REPL、LSP全 semantic projection、component helper、両target packaged
  release parity、`LEGACY-TOOL-01` aggregateは未完了のまま残す。続く `d05d2642` で `validate --source --format text` の
  unknown trace-gap report / exit `2` を同じ runnerへ追加し、Linux replayは `23 cases` 全 passとなった。
  続く `bb2725be` では、実際の `ad65eaff` Linux x86_64 App.Cli target-only programを
  `scripts/ci/test-native-selfhost-mcp-runtime.py` と `scripts/native-selfhost-mcp.py` へ接続し、
  JSON-RPC `initialize`、`tools/list`、`lsharp_check`、`lsharp_format`、`lsharp_install`、
  `lsharp_validate` の `6 requests` を実行した。selfhost CLIの legacy `check --format json` は clean sourceに限り
  `command/type/diagnostics/migration/failureKinds` を検証して structured MCP `{ok:true, diagnostics:[], migrationDiagnostics:[]}`
  へ投影し、正常な `failureKinds:[0]` を許可する。診断、migration row、非ゼロ failure kindは owner/rangeを失わないため
  明示的に fail-closed とし、install は `native MCP package installation requires an explicit external provider adapter` を返す。
  `lsharp_validate` は native process exit `2` の unknown reportを structured resultとして保持した。Linux runtimeは
  `native MCP runtime contract passed: 6 requests`、stderr空で passし、VM workdirを削除して停止した。
  `cf3692e5` でこの6-request runtime contractを追加し、`1f0de6a7` で migration rowを含む legacy clean-looking summaryを
  structured MCP successへ投影しない negative testを追加した。native MCP suiteは `113 tests` 全 passで、migration、診断、非ゼロ
  failure kindを含む legacy summaryは情報を捨てず明示的に fail-closedする境界を固定した。
  これは保存済み production artifactに対する replay-only evidenceであり、現HEADの source provenance gateではない。
  native MCPの診断/migration structured parity、実 install/provider、LSP全 semantic projection、component helper、両target packaged
  release parity、`LEGACY-TOOL-01` aggregateは未完了のまま維持する。
  さらに `cbbafe94` の current checkoutで、host probeを省略する `LSHARP_NATIVE_LINUX_X86_SKIP_HOST_PROBES=1` と既存VM-side lockを
  使い、Linux x86_64 actual stage1→stage2→stage3を一回だけ再生成した。stage1 manifestは target
  `x86_64-unknown-linux-gnu`、source commit `cbbafe9423509030270a4f76ef909ff42ed23663`、code `4,434,755`、data `2,757`、
  entrypoint `4,432,195`、function-start `3,425`、main function `3,434`。stage2/stage3は各 `11,448,943` bytes、stdout SHA-256各
  `a66bf8c746a9cf91a6b0cdb0509a9f12b3b7987301f025646d69fdffd1c6677e`、stderr空で `status=pass` となった。
  同じstage2を再利用した current Linux x86_64 `App.Cli` target-only exportは source tree SHA-256
  `e4a69c95109f00bf3019572efe01c6dff0e7555d14be19308574d0cacadeeedd`、`selfhost_fixed_point=true`、program SHA-256
  `a090cd8474c6115ac3a2bcf5570226cc912d7479d0285f6991ca02fb5a1d6469`、code `13,374,506`、`--version` `lsharp 0.1.0`、stderr空を
  確認した。実Linux programへ `initialize`、`tools/list`、check/format/install/validateの6-request MCP runtime contractを渡し、
  `native MCP runtime contract passed: 6 requests` を確認した。`tools/list` の validate advertisementも含む。artifactは
  `ci-artifacts/native-linux-x86-hostgen-vm/cbbafe94-mcp-current-source-skip/` と `...-cli/` に保存し、VM workdirは削除、VMは停止した。
  同じ `App.Cli` programへ `scripts/ci/test-native-selfhost-cli-core-runtime.py` を渡した public core command matrixも
  `native CLI core runtime matrix passed: 23 cases` となった。help/version、parse/check/fmt、text/metadata test、Preview1
  compile/build、review/doc/doc-ack/doc-check、install/repl、bare LSP、`lsp --stdio` hover、validate unknown report、unsupported
  doc formatを含み、成功経路の stderrは空だった。
  これは current-source Linux native selfhostとMCP runtimeの verified partialを追加するが、現HEADとの差分はtest-onlyであり、
  release acquisition/rollback、Mac/Linux packaged provenance parity、全公開 command、native MCPの診断/migration structured parity、
  実 install/provider、`LEGACY-TOOL-01` aggregateは未完了のまま維持する。Evidence commit: `0e0a6a6c`。
  続く current `HEAD=a0845320584ba044c690404fd0249770a5118fff` では、Mac Apple Siliconの
  `scripts/ci/native-macos-aarch64-selfhost-release.sh` を一度だけ実行し、current-source stage2→stage3 fixed-pointから
  `App.Cli` native release programを生成した。E2Eは `1 passed` / `900.55s`、artifactは `4,748 KiB`、manifestは target
  `aarch64-apple-darwin`、同じ source commit、`selfhost_fixed_point=true`、program SHA-256
  `281d178673c975d89fbb47b810d1fdac380935a173912e6fc46843107f5704c7` を記録し、`file` は Mach-O 64-bit arm64だった。
  `--version` は `lsharp 0.1.0`、stderr `0` bytesで、同じ programを使った public core CLI matrix `23 cases` と native MCP
  runtime `6 requests` も全 passした。生成時の cargo targetは終了時に自動削除し、Mac側の current-source runtime artifactは
  `ci-artifacts/native-release/aarch64-apple-darwin/current-a0845320/` に保持している。これは Mac current-source
  App.Cli/MCPの verified partialを追加するが、Linuxと同一source commitでの同時 packaged provenance parity、公開 release
  asset acquisition/rollback、全公開 command semantics、native MCPの診断/migration structured parity、実 install/provider、
  `LEGACY-TOOL-01` aggregateは未完了のまま維持する。Evidence commit: `a0845320`。
- [~] `V2-16e` / `LEGACY-BOOT-01` bootstrap/oracle/rollback isolation — source commit と
  fingerprint を検証する stage0 package と両 target の daily Rust-free core slice は verified。
  public acquisition、current-checkout regeneration、release asset、rollback 実行、
  Rust oracle/host integration の隔離を閉じる。2026-08-02 に current `main` の Mac Apple Silicon
  native App.Cli releaseを再生成し、stage2/stage3 fixed-point、source commit、target、artifact
  digest、`--version` smokeを確認した。Linux x86_64のcurrent-source package/acquisition、
  release archive、rollback実行、両 targetのprovenance parityは残る。さらに 2026-08-03 に、現行 commitの
  Linux x86_64 stage1/package/source-file smokeと、同stage1を再利用した stage2→stage3 fixed-pointを実行した。
  stage1 manifestは target `x86_64-unknown-linux-gnu`、source commit `113d3785a54e0d4af0bc970edfe45c234a96449d`、
  code `4,408,352` bytes、data `2,757` bytes、entrypoint `4,405,792`、function-start `3,418`、main function `3,427`。
  package manifestとsource-file smoke evidenceも同じ source commitを持ち、source-file smokeは exit `0`、compile/buildは各
  `2,559` bytesでSHA-256 `afd1638e444a7e8c371dc1d17550479fcc5e4efbbb9e9dbdffa8551933d71a00`。
  stage2/stage3 fixed-pointは code length各 `11,408,204`、stdout SHA-256各
  `b9569a6202412633e2ff258a6988bdd5d9556e1b354ab8fe203b67b5f511b26a`、stderr空で passした。
  これは current-source Linux native coreの verified partialを広げるが、公開 release asset acquisition、rollback archiveの
  実行、Mac/Linux packaged provenance parity、component sidecarの完了証拠には数えない。さらに同日、現行 commitの
  Linux x86_64 stage1 host payloadを `333.21s` で生成し、code `4,393,425` bytes、data `2,757`
  bytes、entrypoint `4,390,965`、function-start `3,409` を確認した。Lima VMでこのstage1を
  materializeし、metadata range `0..1` を実行した診断 gateは `status=diagnostic`、stdout `8,353`
  bytes、stderr `0` で成功した。さらに current `HEAD=41be4f2b28a329addffd3cd4de55f075b76a9ec2`
  から同じ stage1 を `347.89s` で再生成し、manifest の source commit と target、code/data/
  entrypoint/function metadata が一致することを確認した。Lima VMで metadata range `0..8` を
  実行した診断 gateは `status=diagnostic`、stdout `53,484` bytes、stderr `0` で成功した。
  これは stage2 metadata prefix の入口が現行 source でも進むことを示すが、full stage2/stage3、
  current-source Linux stage0 package、
  source-file smoke、release/rollbackは未完了のまま残る。
  さらに entrypoint user index `3408`（actual function index `3418`）だけを metadata `3408..3409`
  / prefix `128` で診断し、IR opcode `40` の user call `(3416, 3417, 3415)`について emitted bytes、
  signed rel32、function-relative targetを相関した。3 rowとも `e8 dd 9e ff ff` → `-24867` / `-24851`、
  `e8 cd a8 ff ff` → `-22323` / `-22269`、`e8 24 8b ff ff` → `-29916` / `-29852`となり、stage1の
  expected target diagnosticと一致した。これは call rel32の最小修正を正当化する不一致がないことを
  示す診断であり、full transport/materializeの実行証拠ではない。
  さらに `test-package-native-linux-x86-actual-stage1-vm.sh` の fake Lima harness で、current HEADの
  target/source commitを検証した stage1 packageを stage0 packageへ変換し、その packageを
  `scripts/native-selfhost-dev.sh check` に渡す契約を RED→GREEN で固定した。bundled transport driver、
  decoder、materializer、programを runnerから消費し、`Int` / `diagnostics:0`、stderr空、program materialize、
  `cargo` / `rustc` / host `lsharp` の未呼び出しを確認した。これは package consumer と provenanceの
  fake harness verified sliceであり、実 Linux current-source stage1 packageの source-file smoke、full
  stage2/stage3、release acquisition/rollback、両 target parityの完了証拠には数えない。ADR:
  `docs/adr/decisions-v0.3-native-linux-stage0-package-runner-contract.md`。
  さらに `scripts/ci/test-fetch-stage0-atomic-install.sh` を追加し、local stage0 archiveの checksum/
  provenance検証、checksum未登録 payloadの reject、最終 install move failureの REDを一つの契約にした。
  既存 stage0を復元し、temporary packageと hidden previous backupを残さない GREENを確認した。
  これは fetch package file-set と atomic rollbackの verified sliceであり、公開 release asset、実 Linux runtime、両 targetの rollback archive parity、
  `LEGACY-BOOT-01` 全体の完了証拠には数えない。ADR:
  `docs/adr/decisions-v0.3-native-stage0-fetch-atomic-install.md`。
  さらに Linux x86 selfhost `emit-x86-selfhost-string-concat-helper` の per-allocation `mmap` を、materializer-owned
  `r14` native heapのcursor/limitを使う16-byte aligned bump allocationへ置き換えた。REDでは
  `test_native_codegen_x86_string_concat_uses_bounded_heap_cursor` が旧 `mov eax,9; syscall` byte sequenceを検出し、
  GREENでは同テスト、197-byte helperのemitter回帰、slice/concat call-site、CLI/write-file trailer offsetの
  focused testsが passした。helperは195→197 bytesとなり、後続helper offsetはsource contractと実測byte vectorで
  `+2` に同期した。Lima `lsharp-linux-x86` の最小Linux x86_64実行では、dynamic tagged `"ab"` と `"Z"` を
  `r14` heapへ渡し、連結結果長 `3` を exit code `3` として確認した。これは bounded string-concat allocation/copy/tagging
  の verified sliceであり、full stage2/stage3 fixed-point、current-source Linux stage0 source-file smoke、package
  acquisition/release/rollback、`read-file` の残りの allocation contract、両 target
  release parityの証拠には数えない。ADR:
  `docs/adr/decisions-v0.3-native-linux-string-concat-bounded-heap.md`。
  続けて `emit-x86-selfhost-int-to-string-helper` の固定32-byte `mmap` allocationを、`r14` native heapのcursor
  offset/limitを使うbounded allocationへ移行した。fallback `8192` はlimit確認前には保存せず、`cursor+32 <= limit`
  の成功後だけcursorを更新し、object addressは `r14 + cursor` とする。REDでは
  `test_native_codegen_x86_int_to_string_uses_bounded_heap_cursor` が `mov rax,9; syscall` byte sequenceを検出し、
  GREENでは同テスト、既存のsigned decimal/header/callee-saved ABI回帰、import dispatch source contractが passした。
  helperは169→160 bytes、int helperはtrailer末尾のため開始offset `base+2219`は維持し、size/append lengthを160へ
  同期した。生成byte列とclangで再構成したobject bytesは160 bytesで一致した。Lima `lsharp-linux-x86` では
  `-42` を `-42` として出力し、`limit=cursor+31` の `rax=0` / cursor不変と `+32` の成功を同じVM-side lockで確認した。
  これは signed int-to-string bounded allocationとlimit boundaryのverified sliceであり、full stage2/stage3 fixed-point、
  current-source Linux stage0 source-file smoke、package acquisition/release/rollback、`read-file` の残りの
  allocation contract、両 target release parityの証拠には数えない。ADR:
`docs/adr/decisions-v0.3-native-linux-int-to-string-bounded-heap.md`。
  さらに Linux x86 selfhost `emit-x86-selfhost-substring-helper` の per-allocation mmapを、materializer-owned `r14`
  native heapのcursor/limitを使う bounded allocationへ置き換えた。`8 + (end - start)` を16-byte境界へ alignし、
  limit超過時はcursorを変更せず nullを返し、既存の signed range、String header、payload copy、high-bit tag、callee-saved
  ABIを維持した。helperは145→147 bytesとなり、後続helper offset、append length、rel32 targetを同期した。REDでは
  `test_native_codegen_x86_substring_uses_bounded_heap_cursor` が旧 mmap syscall byte sequenceを検出し、GREENでは同テスト、
  `end - start` length、emitter/call-site/trailer/function-metadataの focused testsが passした。host-side Linux x86 object
  smoke matrixにも substring objectを含む。これは substring bounded allocationの verified sliceであり、current-source VM
  object runtime、full stage2/stage3 fixed-point、Linux native stage0 source-file smoke、package acquisition/release/rollback、
  `read-file` runtime evidence、Mac/Linux release parityの証拠では数えない。ADR:
  `docs/adr/decisions-v0.3-native-linux-substring-bounded-heap.md`。
  さらに source commit `a41cf0655a12d88ac3ed2185492183049234cc7d` の Linux x86_64 stage1 payloadを、
  既存の stage1 code/data/seed debug payloadとして再利用し、Lima `lsharp-linux-x86` で stage2/stage3
  native self-regenerationを一度だけ実行した。`actual-selfregen-summary.json` は `status=pass`、
  stage2/stage3 code length各 `11,374,654`、stdout各 `12,207,069` bytes、SHA-256各
  `7a837812e20e71378632bbe0a101d18c141e3304fb63562890e8ee4425a00930`、stderr空を記録した。
  stage1 manifestは target `x86_64-unknown-linux-gnu`、code `4,393,234` bytes、data `2,757` bytes、
  entrypoint `4,390,778`、function-start `3,409`、main function `3,418` である。artifactは
  `ci-artifacts/native-linux-x86-hostgen-vm/a41cf065-stage23-reuse/` に保持した。後続の `66562385` は
  stage1 replay用の seed source mappingだけを補強したため、この重い固定点replayは重複実行していない。
  この証跡は full stage2/stage3 fixed-pointの verified sliceであり、current HEADでの Linux native stage0
  package/source-file smoke、release acquisition/rollback、Mac/Linux artifact parity、`LEGACY-BOOT-01`
  aggregateの完了証拠ではない。
  さらに Linux x86 selfhost `emit-x86-selfhost-read-file-helper` の per-file `mmap` allocationを、
  materializer-owned `r14` native heapのcursor/limitへ置き換えた。cursorのfallbackは `8192`、予約上限は
  `0x200010`、read上限は `0x200000` とし、read成功後だけcursorを更新して既存の open/read/close、
  String header、high-bit tag、callee-saved ABIを維持する。REDでは
  `test_native_codegen_x86_read_file_uses_bounded_heap_cursor` が旧 mmap syscall byte sequenceを検出し、
  GREENでは helper emitter、call-site、trailer offset、2 MiB cap、read-file object smokeを確認した。
  helperは207→208 bytesとなり、後続helper offsetを `+1` に同期した。host-side artifact
  `ci-artifacts/native-linux-x86-hostgen-vm/0fb8a19b-read-file-bounded/` の summaryは target
  `x86_64-unknown-linux-gnu`、expected/actual exit `7` で `status=pass` である。これは read-file bounded
  allocationと narrow object runtimeの verified sliceであり、全 fd/error semantics、current HEADの
  source-file smoke、Mac parity、package/release/rollbackの完了証拠ではない。ADR:
  `docs/adr/decisions-v0.3-native-linux-read-file-bounded-heap.md`。
  さらに current `HEAD=0b668bf500afbf03ffb55297abd381dfee845957` で Linux x86_64 actual stage1を一度だけ
  fresh生成し、`1 passed` / `305.60s`、code `4,393,234` bytes、data `2,757` bytes、entrypoint
  `4,390,778`、function-start `3,409`、main function `3,418`、manifest source commit一致を確認した。
  これを VM materializerで `lsharp-native-selfhost-stage0` packageへ変換し、manifestの target、compiler、
  transport driver、materializer、source commitを確認した。packageは
  `ci-artifacts/native-linux-x86-hostgen-vm/0b668bf5-stage0-package/` に保持した。
  同じ packageを `LSHARP_NATIVE_LINUX_X86_TRANSPORT_CHUNK_SIZE=64`、timeout `900` 秒で
  `native-linux-x86-native-stage0-source-file-smoke.sh` に渡し、`parse`、`check`、`fmt`、通常/metadata/property
  `test`、EC-M3 validationのpositive/negative fixture、`compile`、`build` を native stage0から実行した。
  runは `exit_code=0`、stderrなしの成功 markerで終了し、`parse` は `decls:1` / `first-body:int` /
  `diagnostics:0`、`check` は `Int` / `diagnostics:0`、`test` は `examples:0` / `invariants:0` /
  `failures:0`、metadata testは `examples:2` / `invariants:1` / `failures:0`、compile/buildは各
  `wasm-size:2559`、Wasm bytesは同一 SHA-256 `afd1638e444a7e8c371dc1d17550479fcc5e4efbbb9e9dbdffa8551933d71a00`
  / 2,559 bytesだった。`cargo`、`rustc`、host `lsharp` は blocklistで成功経路から除外した。
  evidence manifestは `source_commit`、stage0 manifest SHA-256
  `fe1bf90ff3e7542cd6df7b92f9ef5069620d93d49d693a8002b505e104ee9a19`、payload SHA-256
  `0111da02752e0e9e3ffa20deaf7818e1aa167c2f20c2168132a340a220625c8e`、target、exit `0` を記録し、
  `ci-artifacts/native-linux-x86-hostgen-vm/0b668bf5-source-smoke-evidence/` に保持した。これは current
  Linux stage0 package/source-file smokeの verified sliceであり、公開 release assetの acquisition、rollback
  実行、全公開 command、component sidecar、Mac/Linux packaged artifact parity、`LEGACY-BOOT-01` aggregateの
  完了証拠ではない。
  さらに current `HEAD=80b299def6ca9b036b09bc427bc6d3e381fe63bf` で Mac Apple Silicon の
  `native-macos-aarch64-stage0-release.sh` を実行し、`test_e2e_native_macos_aarch64_actual_app_cli_release_program`
  を `1 passed` / `1,248.21s` で完走した。stage0 packageは target `aarch64-apple-darwin`、manifestの
  `source_commit` は同じ current commit、`bin/compiler` は arm64 Mach-O、transport driver/materializerの
  relative pathを保持する `ci-artifacts/native-stage0/aarch64-apple-darwin/80b299de-current/` に保存した。
  同じ packageを Mac native `scripts/ci/native-selfhost-dev-source-file-smoke.sh` に渡し、
  `parse`、`check`、`fmt`、通常/metadata/property `test`、EC-M3 validation positive/negative fixture、
  `compile`、`build` を実行した。runは `aarch64-apple-darwin native selfhost source-file smoke passed`、
  exit `0` で終了し、成功経路の `cargo`、`rustc`、host `lsharp` は blocklistで呼び出されなかった。
  positive outputは `parse=decls:1/first-decl:defn/first-body:int/diagnostics:0`、
  `check=Int/diagnostics:0`、`fmt=(defn main [] 42)`、通常test `examples:0/invariants:0/failures:0`、
  metadata test `examples:2/invariants:1/failures:0`。compile/buildの Wasm bytesは各 `2,559` bytes、
  SHA-256 `afd1638e444a7e8c371dc1d17550479fcc5e4efbbb9e9dbdffa8551933d71a00` で一致した。
  evidence manifestは exit `0`、target、source commit、stage0 manifest SHA-256
  `0452f04df1a6a0b9ec5d57e0833542983f4a6ccfbf4df86a2b505db19e5bd5ec`、stage0 payload SHA-256
  `7d328772978bad0a3a0e4abf9703675cd9e01f5fa3c8c5bdc3461f1c0e8c938c` を記録し、
  `ci-artifacts/native-stage0/aarch64-apple-darwin/80b299de-source-smoke-evidence/` に保持した。
  初回 smokeの cleanup では macOS Bash の `set -u` と空 optional array展開の failureを REDとして検出し、
  `test-native-selfhost-source-file-smoke-evidence.sh` に回帰テストを追加した。writer commandを常に配列化して
  optional argumentだけを条件付きで appendする修正後、同テスト、`bash -n`、`git diff --check`、Mac source-file
  smokeを再実行してGREENを確認した。これは Mac current-source stage0 package/source-file smokeの verified
  sliceであり、公開 release asset acquisition、rollback実行、全公開 command、component sidecar、Linux/Mac
  packaged artifact parity、`LEGACY-BOOT-01` aggregateの完了証拠ではない。

  さらに `2c9bd916` の local verification batch で、現行 checkout の package/release boundary 契約を一括再検証した。
  `test-native-stage0-package.sh`、`test-native-stage0-release-package.sh`、`test-fetch-stage0-archive-provenance.sh`、
  `test-fetch-stage0-atomic-install.sh`、`test-fetch-stage0-provider-url.sh`、`test-release-smoke-provider-snapshots.sh`、
  `test-native-official-release-replay-lock.sh`、`test-native-official-release-snapshots.sh` は全て passした。確認できたのは
  checksum/provenance検証、未登録payloadの拒否、install move failure時の既存stage0復元、unsafe provider URLと live replay lockの
  fail-closed、review/provider snapshotの全量・digest・rollback payload検証である。これは local/fake harness の契約証拠であり、
  実 provider release assetの取得、current-source Mac/Linux packaged archive、rollback archiveの実行、両targetの packaged
  provenance parity、`LEGACY-BOOT-01` aggregateの完了証拠には数えない。VMは起動せず、共有artifactや一時領域も残していない。

  実 provider assetの確認では、GitHub release `v0.1.0-native-rc1`（`2026-05-11T13:23:20Z` 公開）は
  `aarch64-apple-darwin` の experimental archiveのみを持ち、Linux x86_64 assetが存在しなかった。archive SHA-256は
  `cd1a5df9db240eb155fe5b2fc9d6c24f721d6f078a870cfad4954724820725a0` だが、embedded manifestは stage hashのみで
  current source commit/provenanceを持たないため、`HEAD=5db1c2a4` の release evidenceや両 target packaged parityには
  採用しない。current-source release archiveの生成、provider acquisition、rollback archive実行は未完の外部 release
  boundaryとして残す。

  その後 `cbbafe94` の current checkoutで、host probeを省略する明示設定とVM-side lockを使った actual stage1→stage2→stage3
  fixed-pointを完了し、stage1/stage2/stage3 manifestの source commit、target、code/data/entrypoint/function metadata、
  stage2/stage3 stdout hash一致を確認した。さらに同じstage2から Linux x86_64 `App.Cli` target-only native releaseをmaterializeし、
  source tree fingerprint、program digest、`--version` smoke、MCP 6-request runtimeを実行した。これは current-source Linux regeneration
  と target runtimeの verified partialを広げるが、public release asset acquisition、rollback archiveの実行、Mac/Linux packaged
  provenance parity、Rust oracle/host integrationの完全隔離、`LEGACY-BOOT-01` aggregateは未完了のまま維持する。
  さらに `HEAD=a0845320584ba044c690404fd0249770a5118fff` の Mac Apple Silicon current-source App.Cli releaseを生成し、
  manifest source commit/target/fixed-point/program digest、Mach-O arm64、`--version` smoke、CLI `23 cases`、MCP `6 requests`を
  実行した。これは Mac側の current-source release-program producerと runtimeの verified partialであり、公式 release assetへの
  acquisition、archive rollback、Linuxとの同一 source commitを含む packaged provenance parity、component sidecarの完了証拠には
  数えない。Mac/Linux VM・cargo target・一時 lockは検証後に停止・回収済みである。

## ISSUES-derived quality and runtime work

[ISSUES.md](ISSUES.md) の active issue を実装可能な aggregate へまとめる。issue の問題定義と
根拠は ISSUES、作業順と completion boundary は本節を正本とする。

- [~] `LEGACY-DIAG-01` stable diagnostics — Issue `I-02`。syntax/types/IR/codegen と主要
  CLI/LSP/MCP forwarding は verified。compile multi-file、REPL、doc、metadata、native linker、
  LSP incremental/module/codegen の code/span forwarding を閉じる。
- [~] `LEGACY-RUNTIME-01` dynamic GC layout and allocator — Issues `I-03` / `I-04` / `D-10`。
  core WASI の object/free/root table growth と allocation failure の stable `LS4002` は verified。
  free-list size class、sentinel precise discrimination、HTTP/component/selfhost/native parity を
  actual runtime/metrics と両 supported target で閉じる。
- [~] `LEGACY-EXEC-01` advanced runtime — Issues `D-01` / `D-02` / `D-03` / `D-04` /
  `D-06` / `D-09`。WasmGC の record/ADT/string/closure slice は verified。GADT/HKT/
  computation expression、trait vtable、selfhost representation、supported 2 target を閉じる。
- [~] `LEGACY-ROOT-01` rooting discipline — Issue `I-07`。runtime failure ledger と
  compiler root-lifetime ledger の slice は verified。全 selfhost source、stateful REPL/LSP、
  indirect control flow、Mac/Linux native stage0 を閉じる。
- [~] `LEGACY-MODULE-01` SCC inference and cache generalization — Issues `D-07` / `I-05`。
  SCC detection/inference、Formatter batch 特例除去、Rust host の process/artifact cache、
  validation/runtime と明示 maintenance は verified。Formatter 固有 dirty-set、canonical runtime、
  source override の segment/disk persistence、自動 eviction、selfhost/native compiler、
  public command と両 supported target の evidence を閉じる。
  計測起点: `docs/development/operations/rust-boundary-reduction.md` の T0-5 節に、
  `lsharp-ir --lib` の壁時計 107.8s のほぼ全量を単一 test (`incremental_analysis_tests::
  test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds`) が占めることを記録済み。
- [~] `LEGACY-MAINT-01` large-file decomposition — Issues `I-01` / `I-08`。多数の test/
  production split と `lsharp-ir/src/lib.rs` の `Instruction` / `IrType` および
  `Module` / `Function` / GC model、linker seam、compile surface seam、compile/incremental orchestration seam、`validation_source` node/evidence/typed edge seam、validation source adapter test seam、selfhost evidence registry runtime/validation test seam、selfhost evidence parser duplicate-field seam、selfhost native differential test seam、selfhost bootstrap four-layer test seam、selfhost bootstrap acceptance test seam、selfhost typeinfer E2E test seam、selfhost lexer/parser parity E2E test seam、WasmGC probe test seam、selfhost native stage23 gap test seam、validation input manifest/reference seam、native emitter memory seam、atomic/durable writer cleanup test seam、validation output manifest wire seam、native selfhost transport strict payload-length seam、WASI runner Preview1/Preview2 mode seam は verified。`wasi.rs`、`lsharp-ir/src/lib.rs`、`lsharp-tooling/src/compile.rs`、
  `infer.rs`、parser/lower/driver/LSP の責務分割を、型・focused test・snapshot parity を保って完了する。WasmGC emitter の instruction lowering / Component output seam / Preview2・CLI runner seam、WASI HTTP handler core seam、WASI GC collector seam、WASI tests core seam も verified とし、残る責務分割を続ける。
- [ ] `RUNNER-SCANNER-01` selfhost TestRunner の legacy scanner を canonical inventory へ収束させる —
  Issue `I-30`。`TestRunner.ls` に `collect-defn-metadata-loop` / `extract-test-cases-loop` の
  旧 scanner 2 本と `extract-parser-contract-suites` の canonical inventory が並存している。
  受入条件: 旧 scanner 2 本が `TestRunner.ls` から消え、`grep -c` が 0 になること。
  現 runner の result shape と invariant-first suite shape が変わらないこと (E2E で固定する)。
  **含めない範囲**: canonical migration classifier の実装、runner の result shape 変更そのもの。
- [ ] `TYPEINFER-SPLIT-01` `TypeInfer.ls` から signature 推論を別モジュールへ切り出す —
  ADR [`decisions-worktree-absorption-2026-08-20.md`](docs/adr/decisions-worktree-absorption-2026-08-20.md)。
  滞留 branch `codex/lsharp-typeinfer-property-aggregation-batch` (`3b5dbef5`) が
  `selfhost/src/Types/TypeInferSignature.ls` (305 行) として実施済みだが、`TypeInfer.ls` +3013 行と
  `crates/lsharp-wasm/tests/e2e/support.rs` 10 箇所以上を巻き込むため取り込まなかった。
  受入条件: `TypeInfer.ls` が 800 行以内に収まり、切り出し後も `selfhost_typeinfer*` の e2e が
  全件 PASS すること。切り出し単位は branch の分割をそのまま真似る必要はない。
  **含めない範囲**: bounded rooted scan への変換 (別枠)、branch の他の差分の取り込み。
- [ ] `BOUNDED-SCAN-01` 滞留 branch の bounded rooted scan 変換を移植する —
  ADR [`decisions-worktree-absorption-2026-08-20.md`](docs/adr/decisions-worktree-absorption-2026-08-20.md)。
  `codex/lsharp-typeinfer-property-aggregation-batch` (`3b5dbef5`) が入れた
  `<name>-step-64-loop-bounded` 族のうち、**main と branch が同じ defn を両方書き換えている**
  ため機械的に取り込めなかった分。branch ref は消さないこと (唯一の実装が載っている)。
  - `Types/TypeInfer.ls` (両方変更 4 / 17 family): `typeinfer-build-parametric-alias-param-state`
    `typeinfer-free-vars-to-set` `typeinfer-import-only-contains` `typeinfer-next-module-index`
    `typeinfer-pending-env-vars` `typeinfer-predeclare-closed-aliases` `typeinfer-predeclare-defns`
    `typeinfer-prior-definition-failed` `typeinfer-program-analysis`
    `typeinfer-qualify-import-adt-variants` `typeinfer-qualify-import-record-accessors`
    `typeinfer-qualify-imports-with-open` `typeinfer-recursive-alias-count`
    `typeinfer-refresh-closed-aliases` `typeinfer-refresh-closed-aliases-rounds`
    `typeinfer-remove-defns-before-module` `typeinfer-type-expr-contains-name`
  - `Syntax/Parser.ls` (両方変更 27 / 20 family): `collect-example-expression-spans`
    `parse-computation` `parse-constructor-pattern-args` `parse-do-expr` `parse-import-only-symbols`
    `parse-import-options` `parse-int-digits-from-str` `parse-let-binding` `parse-let-fold`
    `parse-match-arm` `parse-params` `parse-record-decl-fields` `parse-recordlit-fields`
    `parse-recordpat-fields` `parse-skip-to-close` `parse-type-alias-param-hashes`
    `parse-type-expr-list` `parse-type-variant-fields` `parse-type-variants`
    `skip-optional-metadata`
  - `Types/TypeInferPattern.ls` (両方変更 13 / 2 family): `infer-match-arms`
    `strip-match-scrutinee-vars`
  - `Types/TypeInferCore.ls` (5 family): branch が `error-code-recursive-alias` を再定義せず
    削除しており、main は 9 箇所から呼ぶ。**diagnostics code の対応関係を先に決めること。**
  - `Types/TypeInferFunctions.ls` (5 family): `TYPEINFER-SPLIT-01` に従属。branch が
    `TypeInferSignature.ls` へ移した 3 defn を main の `TypeInfer.ls` が呼んでいるため単独では入らない。
  受入条件: 移植した family ごとに、chunk 境界 (65 要素) を跨ぐ e2e が 1 本以上あること。
  **含めない範囲**: `Types/TypeInferAdt.ls` (branch のみの family が 0 で取り込むものが無い)、
  branch の非 bounded-scan 差分。
- [ ] `WORKTREE-ABSORB-02` 未取り込み branch 3 本の取り込み判断 —
  ADR [`decisions-worktree-absorption-2026-08-20.md`](docs/adr/decisions-worktree-absorption-2026-08-20.md)。
  母集団は **全 local branch 129 本** (2026-08-22 に worktree 限定から広げ直した)。
  `git cherry main <branch>` が `+` を返すのは **49 本**で、そのうち batch family 26 本は
  `BOUNDED-SCAN-01` が正本 (family 単位 hand-port のみ。merge はしない。
  tip に無い唯一の例外 `a5bb397a` は 2026-08-22 に却下判定済み)。残る非 batch 23 本のうち
  20 本は ADR で判定済み。ここの対象は残る **3 本**。
  **branch ref は消さないこと。** 取り込み済み 80 本のうち、main の祖先 25 本は削除済み、
  patch-id 一致のみの 46 本は **未削除** (worktree 固定の 9 本は対象外)。台帳は
  [`absorbed-branch-refs-2026-08-22.md`](docs/development/operations/absorbed-branch-refs-2026-08-22.md)。
  判定は `git cherry` の commit 数ではなく **touched file の content diff** で行う
  (whole-file take / hand-merge で入れた分は patch-id が一致しないため)。
  commit 数の多い順に:
  - `codex/legacy-module-cache-format-identity` (120) / `codex/v0.2-ec-m1-02-integration` (119) /
    `codex/legacy-maintenance-docs-active-only` (86) —
    いずれも **origin に無い local のみ**。相互に包含関係は無く独立 (`git cherry` で確認済み)。
    `codex/legacy-maintenance-stage-chain-integration` (56) /
    `codex/legacy-maint-native-stage-chain-split` (67) /
    `codex/legacy-maint-native-differential-split-audit` (65) は 2026-08-22 に判定済みで、
    live な残りは `FMT-ROUNDTRIP-01` / `GC-LEAK-CYCLE-01` / `RUST-FILE-SIZE-GATE-01` / `I-35` /
    `MODULE-DUP-FN-01` / `MODULE-ALIAS-EXPORT-01` / `MODULE-BODY-FORM-01` /
    `DOCTOOLS-META-SLOT-01` へ引き取った
  受入条件: 1 本ごとに「取り込む / 却下 (理由付き)」を ADR へ記録し、判断済みの branch を
  この一覧から削除すること。7 worktree の未 commit 内容は 2026-08-22 に main と突き合わせ済みで、
  **salvage すべき内容は 0 件**だった (ADR)。worktree 自体は branch ref を固定するために残す。
  **含めない範囲**: branch ref の削除 (判断が全部終わるまでしない)、CI 設定。
- [ ] `LINT-CLIPPY-01` `lsharp-types` の clippy gate 復旧 — Issue `I-31`。
  `crates/lsharp-types/src/review_trust_store.rs:120` の nested `if` が `collapsible_if` に当たり、
  `cargo clippy -p lsharp-types -- -D warnings` が lib / lib test / all-targets の 3 経路で
  compile error になる。`INFER-DEPTH-01` の変更以前から出ている。
  受入条件: 当該 3 経路が exit 0 になること。
  **含めない範囲**: `-D warnings` を CI で常時要求するかの判断 (CI の扱いは別 slice)。
- [ ] `LINT-FMT-01` `lsharp-ir` の rustfmt gate 復旧 — Issue `I-34`。
  `crates/lsharp-ir/src/lower/mod.rs` の `mod` 宣言順が rustfmt の期待と食い違い、
  `cargo fmt --check -p lsharp-ir` が 8 箇所の diff を出す。
  受入条件: 同コマンドが exit 0 になること。
  **含めない範囲**: CI で fmt gate を常時要求するかの判断 (`LINT-CLIPPY-01` と併せて決める)。
- [~] `LEGACY-TEST-01` property/fuzz/limit coverage — Issues `I-06` / `I-08`。syntax/types
  property test と複数の GC/type/runtime limit lane、bounded regex repeat の 64-case property
  lane は verified。再利用可能な generator、leak/rooting stress、performance threshold、
  full fuzz target、native stage0 evidence を閉じる。
- [~] `VALIDATION-PROJECT-01` project source aggregation — Issue `I-10`。Rust `validate --source` の
  regular file/directory collection と cross-file duplicate node fail-closed boundaryは verified。
  valid cross-file edgeの report/manifest、duplicate evidence/review diagnostics、selfhost/native
  directory input、MCP/public surface、Linux current-source runtimeを閉じる。

## Scheduling rules

- current milestone は `EC-M2-01` → `EC-M2-02` → `EC-M2-03`。同時に一つの observable contract だけを進める。
- product/release completion target は `aarch64-apple-darwin` と `x86_64-unknown-linux-gnu` に限定する。
- Linux VM / stage regeneration は共有 lock と既存 artifact を使い、同じ heavy replay を重複起動しない。
- Rust は bootstrap、oracle/differential、rollback、未移行 host integration の明示境界として保持する。
- verified slice は `[~]` のまま残し、aggregate completion 後に evidence を ADR/仕様へ移して本ファイルから削除する。

2026-07-29 に EC-M2-02 evidence required-field Unicode whitespace boundary を追加した。Rust
source adapter の `str::trim()` と selfhost `Tools.Validation.Whitespace` の UTF-8 byte 判定を
揃え、NBSP を含む Unicode White_Space-only `runner` を stable code `4`、directive/form span 付きで
拒否する。Rust source test、selfhost actual Wasm の同一 fixture、主要 Unicode White_Space 10 種の
direct runtime、Linux x86_64 native source-file smoke/provenance gate を通過した verified partial slice
である。node text、review provenance、manifest input、coverage count/cases、current-source
artifact/runtime、Mac/Linux matrix、EC-M2-02/EC-M3 aggregate は残件。ADR:
`docs/adr/decisions-v0.2-native-validation-evidence-unicode-whitespace.md`。

2026-07-29 に EC-M2-02 review provenance digest の Unicode whitespace boundary を追加した。Rust
source adapter の `trim().is_empty()` と selfhost `IntentSource` の共有 UTF-8 byte helper を揃え、
NBSP-only `provenance_digest` を stable code `8`、review ID、directive/form span 付きで拒否する。
Rust source suite 57件、selfhost actual Wasm の同一 fixtureと既存 review precedence、Linux x86_64
native source-file smoke/provenance gateを通過した verified partial sliceである。visibility、review
lifecycle/authentication、manifest/MCP、current-source artifact/runtime、Mac/Linux matrix、EC-M2-02/
EC-M3 aggregate は残件。ADR: `docs/adr/decisions-v0.2-native-validation-review-unicode-whitespace.md`。

2026-07-29 に EC-M2-01 node text の Unicode whitespace boundary を追加した。Rust source adapter の
`trim().is_empty()` と selfhost `IntentSource` の共有 UTF-8 byte helper を揃え、NBSP-only node本文を
stable code `1`、kind/ID、directive/form span 付きで拒否する。Rust source suite 58件、selfhost actual
Wasm の同一 fixtureと既存 whitespace/precedence/type-source 回帰、Linux x86_64 native source-file
smoke/provenance gateを通過した verified partial sliceである。manifest/MCP、current-source
artifact/runtime、Mac/Linux matrix、EC-M2-01/EC-M3 aggregate は残件。ADR:
`docs/adr/decisions-v0.2-native-validation-node-text-unicode-whitespace.md`。

2026-07-29 に EC-M2-02 coverage bucket の Unicode whitespace boundary を追加した。Rust source
adapter の `trim().is_empty()` と selfhost `Tools.Validation.Whitespace` の UTF-8 byte 判定を揃え、
NBSP-only coverage bucket を stable code `4`、raw value、directive/form span 付きで拒否する。Rust
source suite 59件、selfhost actual Wasm の parser→registry 同一 fixture、Linux x86_64 native source-file
smoke/provenance gateを通過した verified partial sliceである。duplicate/count/cases、manifest/MCP、
current-source artifact/runtime、Mac/Linux matrix、EC-M2-02/EC-M3 aggregate は残件。ADR:
`docs/adr/decisions-v0.2-native-validation-evidence-coverage-unicode-whitespace.md`。

2026-07-29 に EC-M2-02 coverage count の負値 boundary を追加した。Rust source parser は負の
coverage count を parse error 集約 code `LS0104` で拒否し、selfhost actual Wasm の parser→Evidence
registry は invalid-sampling code `11`、field `coverage`、bucket value、directive/form span を保持
して拒否する。native source-file smoke には `source validation error:11`、exit `1`、report/manifestなし
の fixture/assertion を追加し、provenance gate と `bash -n` を通過した verified partial sliceである。
`sum(coverage counts) == cases` の意味論、count 上限、manifest/validate、current-source artifact/runtime、
Mac/Linux matrix、EC-M2-02 aggregate は残件。ADR:
`docs/adr/decisions-v0.2-native-validation-evidence-negative-coverage-count.md`。

2026-07-29 に EC-M2-02 evidence `cases` の負値 boundary を追加した。Rust source parser は
`:cases -1` を stable code `LS0101`、selfhost actual Wasm の source parser→Evidence registry は
invalid-sampling code `11`、field `cases`、empty raw value、directive/form span 付きで拒否する。native
source-file smoke には `source validation error:11`、exit `1`、report/manifestなしの fixture/assertionを
追加し、provenance gate と `bash -n` を通過した verified partial sliceである。`sum(coverage counts) ==
cases` の意味論、非負値上限、manifest/validate、current-source artifact/runtime、Mac/Linux matrix、
EC-M2-02 aggregate は残件。ADR:
`docs/adr/decisions-v0.2-native-validation-evidence-negative-cases.md`。

2026-07-29 に EC-M2-02 evidence `seed` の負値 boundary を追加した。Rust source parser の既存
negative-seed testは `LS0101`、selfhost actual Wasm の source parser→Evidence registry は
invalid-sampling code `11`、field `seed`、empty raw value、directive/form span 付きで拒否する。native
source-file smoke には `source validation error:11`、exit `1`、report/manifestなしの fixture/assertionを
追加し、provenance gate と `bash -n` を通過した verified partial sliceである。generator/shrink/coverage
の実行意味論、manifest/validate、current-source artifact/runtime、Mac/Linux matrix、EC-M2-02 aggregate
は残件。ADR: `docs/adr/decisions-v0.2-native-validation-evidence-negative-seed.md`。

2026-07-29 に EC-M2-02 evidence `shrinks` の負値 boundary を追加した。Rust source parser の既存
negative-shrink testは `LS0101`、selfhost actual Wasm の source parser→Evidence registry は
invalid-sampling code `11`、field `shrinks`、empty raw value、directive/form span 付きで拒否する。native
source-file smoke には `source validation error:11`、exit `1`、report/manifestなしの fixture/assertionを
追加し、provenance gate と `bash -n` を通過した verified partial sliceである。generator/shrink の実行
意味論、manifest/validate、current-source artifact/runtime、Mac/Linux matrix、EC-M2-02 aggregate は残件。
ADR: `docs/adr/decisions-v0.2-native-validation-evidence-negative-shrinks.md`。

2026-07-29 に `validate --source` の negative sampling CLI boundary を `EmbeddedCli` へ拡張した。
`App.Cli` と同じ `:cases -1` fixtureを EmbeddedCli の Rust-host actual Wasm で実行し、
`source validation error:11`、exit `1`、validation reportなし、`--emit-manifest` の出力なしを固定した。
これで両 selfhost CLI surface の source/report fail-closed boundary が verified partial slice となったが、
current-source stage0 artifact/runtime、sampling の実行意味論、supported 2 targets、EC-M2-02/03 aggregate
は残件。ADR: `docs/adr/decisions-v0.2-native-validation-cli-negative-sampling.md`。

さらに `validate --source` の `seed` / `shrinks` 負値 boundary を EmbeddedCli の Rust-host actual Wasm
へ拡張した。`:seed -1` と `:shrinks [-1]` の両 fixtureで `source validation error:11`、exit `1`、
validation reportなし、`--emit-manifest` の出力なしを同一 bundle の実行で固定した。current-source
stage0 artifact/runtime、sampling の実行意味論、supported 2 targets、EC-M2-02/03 aggregate は残件。
ADR: `docs/adr/decisions-v0.2-native-validation-cli-negative-sampling.md`。

さらに version 1 JSON manifest の `sampling.coverage` duplicate key を map 化前の serde visitor で
拒否し、後続値への黙った上書きを防ぐ Rust canonical input boundary を verified slice として追加した。
selfhost/native manifest parity、coverage count/cases の意味論、current-source artifact/runtime は残る。
ADR: `docs/adr/decisions-v0.2-validation-manifest-duplicate-coverage.md`。

2026-07-29 に version 1 JSON manifest の evidence required-field Unicode whitespace boundary を追加した。
execution の `runner` / `target` / `source_commit` / `artifact_digest` と sampling/provenance の
`generator` / `producer` / `tool_version` / `timestamp` を NBSP-only に変異させ、canonical graph 登録前に
`GraphError::InvalidEvidence` / `EvidenceValidationError::EmptyField` として field名を保持して拒否する
ことを `parse_manifest_rejects_unicode_whitespace_only_required_evidence_fields` で固定した。既存の
`str::trim()` policyを manifest inputにも適用する Rust verified partial sliceであり、manifest の
node/review/coverage Unicode parity、selfhost/native manifest parser、current-source artifact/runtime、
supported 2 targets、EC-M2-02/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-manifest-evidence-unicode-whitespace.md`。

2026-07-29 に version 1 JSON manifest の node text Unicode whitespace boundary を追加した。complete
manifest の intent node text を NBSP-only に変異させ、canonical `IntentNode` の登録前に
`ValidationInputError::Node(NodeTextError::EmptyText)` として拒否することを
`parse_manifest_rejects_unicode_whitespace_only_node_text` で固定した。Rust source adapter と同じ
`str::trim()` policyを manifest inputへ拡張した verified partial sliceであり、manifest の review/
coverage Unicode parity、selfhost/native manifest parser、current-source artifact/runtime、supported
2 targets、EC-M2-01/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-manifest-node-text-unicode-whitespace.md`。

2026-07-29 に version 1 JSON manifest の `reviews[].provenance_digest` Unicode whitespace boundary を追加した。
manifest の opaque review digest を NBSP-only に変異させ、registry 登録前に
`ValidationInputError::Graph(GraphError::InvalidReview)` / field `review_provenance_digest` として拒否する
ことを `review_registry_rejects_unicode_whitespace_only_provenance_digest_in_manifest_input` で固定した。
既存の canonical `str::trim()` policy を manifest review registry へ拡張した Rust verified partial sliceであり、
review lifecycle/authentication、manifest の coverage parity、selfhost/native manifest parser、current-source
artifact/runtime、supported 2 targets、EC-M2-02/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-manifest-review-unicode-whitespace.md`。

2026-07-29 に version 1 JSON manifest の `sampling.coverage` Unicode whitespace boundary を追加した。
coverage key を NBSP-only に変異させ、canonical `SamplingPlan` の graph 登録前に
`ValidationInputError::Graph(GraphError::InvalidEvidence)` / field `coverage` として拒否することを
`parse_manifest_rejects_unicode_whitespace_only_coverage_bucket_before_registration` で固定した。source
adapter と同じ non-blank policyを manifest inputへ拡張した Rust verified partial sliceであり、coverage
count/cases の意味論、selfhost/native manifest parser、current-source artifact/runtime、supported 2
targets、EC-M2-02/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-manifest-coverage-unicode-whitespace.md`。

2026-07-29 に version 1 JSON manifest の duplicate review identity boundary を追加した。同一の
`review:checkout/reviewer-001` IDに異なる digest/visibilityを持つ2 recordsを入力し、registry 登録前に
`ValidationInputError::Graph(GraphError::DuplicateReview)` として拒否することを
`review_registry_rejects_duplicate_review_ids_in_manifest_input` で固定した。canonical review registryの
identity policyを manifest inputへ拡張した Rust verified partial sliceであり、review lifecycle/authentication、
other duplicate keys、selfhost/native manifest parser、current-source artifact/runtime、supported 2 targets、
EC-M2-02/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-manifest-duplicate-review.md`。

2026-07-29 に version 1 JSON manifest の edge payload unknown-field boundary を追加した。tagged
`EdgeInput` の relation variant ごとに許可 field を検査し、`motivates` / `constrained-by` /
`tested-by` / `supports` / `contradicts` / `evaluates` / `invalidates` の未知 field と duplicate
field を graph 登録前の `ValidationInputError::Json` として拒否することを全 variant fixture で固定した。
Rust canonical manifest input の verified partial sliceであり、selfhost/native manifest parser、source
producer、current-source artifact/runtime、supported 2 targets、EC-M2-02/EC-M2-03/EC-M3 aggregate は残件。
Evidence: `docs/adr/decisions-v0.2-validation-manifest-edge-unknown-fields.md`。

2026-07-29 に明示 `reviews: []` の review registry presence boundary を追加した。manifest の
省略 (`None`) と empty (`Some([])`) を区別し、空でも `evaluates` / `invalidates` の未登録 review を
`GraphError::MissingReview` として拒否する。canonical output は明示 empty registry を `reviews: []` として
保持し、parse/emit/parse で closure policy を失わない Rust verified partial sliceである。review lifecycle/
authentication、selfhost/native manifest parser、current-source artifact/runtime、supported 2 targets、
EC-M2-02/EC-M2-03/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-manifest-explicit-empty-review-registry.md`。

2026-07-29 に version 1 manifest の明示 `reviews: null` boundary を追加した。省略 (`None`) と
配列 (`Some`) だけを受理し、schema 外の null が registry なしへ変換されないよう custom deserializer で
`ValidationInputError::Json` にする Rust canonical input verified partial sliceである。selfhost/native
manifest parser、CLI/MCP parity、current-source artifact/runtime、supported 2 targets、review lifecycle/
authentication、EC-M2-02/EC-M2-03/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-manifest-null-review-registry.md`。

2026-07-29 に MCP `lsharp_validate` の manifest input schema を version 1 envelope と同期した。
`tools/list` が `schema_version` / `nodes` / `evidence` / `edges` を required、`reviews` を optional として
公開し、unknown top-level field を schema 上で拒否することを `mcp_server::tests` 41件で固定した。
Rust parser と MCP input/output schema の必須境界を揃えた verified partial sliceであり、selfhost/native
MCP、current-source artifact/runtime、supported 2 targets、EC-M2-03/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-29 に EC-M2-02/03 の evidence coverage count invariant を追加した。`coverage` を宣言した
evidence は bucket count の合計を `cases` と完全一致させ、checked-add overflow と不一致を canonical
`SamplingPlan`、Rust source adapter（directive/form span 付き）、version 1 manifest input、公開
`lsharp validate` の全入力境界で fail-closed にする。coverage を省略する既存 observational evidence
は互換性のため受理する。canonical partition、不一致、overflow、source span、manifest/CLI の exit `1`・
空 stdout・manifestなしを focused test で固定した Rust-host verified partial sliceである。coverage の
実行生成/generator policy、MCP/selfhost/native parity、current-source artifact/runtime、Mac/Linux matrix、
EC-M2-02/03 aggregate は残件。Evidence: `docs/adr/decisions-v0.2-validation-evidence-coverage-count.md`。

さらに同じ coverage count invariant を Rust MCP `lsharp_validate` の `manifest` object、JSON string、
`manifest_file` routeへ接続した。`cases=3` に対する `coverage.smoke=2` を direct JSON-RPC に入力すると、
3 routeすべてが `isError: true`、`structuredContent` なし、coverage/cases/covered 付き text error を返す
ことを固定した。さらに source/file route も `sum=1,cases=2` の source adapter diagnostic を同じ
`isError` / no-structured-content boundaryへ投影することを固定した。Rust-host MCP 全 input-route verified
partial sliceであり、selfhost/native MCP producer、current-source artifact/runtime、Mac/Linux matrix、
EC-M2-02/03 aggregate は残件。
Evidence: `docs/adr/decisions-v0.2-validation-evidence-coverage-count.md`。

2026-07-29 に MCP `lsharp_validate` でも review registry の presence semantics を対照固定した。
`reviews` 省略時の未登録 `evaluates` edge は opaque endpoint として `status: unknown` を返し、明示
`reviews: []` の同じ edge は review ID error として拒否することを review registry 6件・MCP suite 43件で確認した。
CLI と MCP の parser policy を揃えた verified partial sliceであり、selfhost/native MCP、current-source
artifact/runtime、supported 2 targets、EC-M2-03/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-29 に review registry の presence semantics を公開 `lsharp validate` で対照固定した。
`reviews: []` の未登録 `evaluates` edge は exit `1`・空 stdout・manifest なしで拒否し、`reviews` 省略時の
同じ edge は opaque endpoint として `status: unknown`・exit `2` を返す。selfhost/native parser、MCP、
current-source artifact/runtime、supported 2 targets、EC-M2-02/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-manifest-explicit-empty-review-registry.md`。

2026-07-29 に明示 `reviews: []` の unregistered review edge closure を公開 `lsharp validate` へ接続した。
未登録 `evaluates` review は exit `1`、空 stdout、manifest file なし、review/missing/identity 診断の
diagnostic-only 結果となることを `manifest_input_cli` と既存 CLI の全30テストで固定した。review lifecycle/
authentication、selfhost/native parser、MCP、current-source artifact/runtime、supported 2 targets、
EC-M2-02/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-manifest-explicit-empty-review-registry.md`。

2026-07-29 に公開 `lsharp validate` の manifest input matrix を拡張した。duplicate review identity は
exit `1`、空 stdout、manifest file なし、review ID/重複診断となり、unknown edge field は同じ
diagnostic-only boundary で unknown field/edge 診断を返すことを新規 `manifest_input_cli` test で固定した。
selfhost/native parser、MCP、current-source artifact/runtime、supported 2 targets、EC-M2-02/EC-M3 aggregate
は残件。Evidence: `docs/adr/decisions-v0.2-validation-manifest-duplicate-review.md`、
`docs/adr/decisions-v0.2-validation-manifest-edge-unknown-fields.md`。

2026-07-29 に `sampling.coverage` duplicate bucket key の wire boundary を公開 `lsharp validate` へ接続した。
同一 `coverage` key を持つ manifest は exit `1`、空 stdout、manifest file なし、`coverage` と duplicate-key
分類を含む stderr の diagnostic-only 結果となることを Rust driver CLI 全27件で固定した。selfhost/native
manifest parity、report/atomic writer、current-source artifact/runtime、supported 2 targets、EC-M2-03
aggregate は残件。Evidence: `docs/adr/decisions-v0.2-validation-manifest-duplicate-coverage.md`。

2026-07-29 に version 1 manifest の subject kind schema parity を追加した。evidence、`evaluates`、
`invalidates` の subject を relation ごとの JSON Schema 定義へ分離し、Rust parser と同じ
`intent`/`claim`/`contract`、`intent`/`claim`/`evidence`、`evidence`/`review` の kind enum を固定した。
これは schema consumer が Rust parser より広い subject を受理しないための static verified partial sliceで
あり、JSON Schema 実 validator、selfhost/native manifest parser、current-source artifact/runtime、
supported 2 targets、EC-M2-02/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-manifest-typed-subject-schema.md`。

2026-07-29 に version 1 manifest の typed subject 診断を追加した。`evaluates.subject` / `invalidates.subject`
の不正 kind を、欠落 node と同じ `MissingNodeReference` にせず relation/kind/stable ID を保持する
`ValidationInputError::InvalidSubjectKind` として fail-closed にする Rust canonical parser verified partial
sliceである。source/native diagnostic parity、selfhost/native manifest parser、CLI/MCP、current-source
artifact/runtime、supported 2 targets、EC-M2-02/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-manifest-subject-kind-diagnostic.md`。

2026-07-29 に `lsharp validate <manifest> --format json` の typed subject input error boundary を追加した。
`evaluates.subject` / `invalidates.subject` の `InvalidSubjectKind` は exit `1`、空 stdout、manifest file なし、
relation path/kind/stable ID を含む stderr の diagnostic-only 結果となり、report JSON と混ざらないことを
Rust driver CLI test で固定した。default EmbeddedCli build
は origin/main の既存 selfhost source に残る `vector-push-single-rooted-v3` 未定義で停止したため、
既存 component artifact を指定した focused test で manifest input 経路だけを検証した。source/native CLI
の同一診断、MCP、current-source artifact/runtime、supported 2 targets、EC-M2-02/EC-M3 aggregate は残件。
Evidence: `docs/adr/decisions-v0.2-validation-cli-subject-kind-diagnostic.md`。

2026-07-29 に `reviews: null` の optional registry type boundary を公開 `lsharp validate` へ接続した。
manifest input error は exit `1`、空 stdout、manifest file なし、`reviews` / `null` を含む stderr の
diagnostic-only 結果となることを Rust driver CLI 全26件で固定した。selfhost/native parser、MCP parity、
current-source artifact/runtime、supported 2 targets、EC-M2-02/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-manifest-null-review-registry.md`。

2026-07-29 に EC-M3-01 version 1 manifest の unsigned numeric overflow boundary を追加した。
`u64::MAX + 1` を span と sampling の全 unsigned fieldへ入力し、Rust canonical parser が graph 構築前に
`ValidationInputError::Json` として fail-closed にすることを
`parse_manifest_rejects_unsigned_numeric_overflow` で固定した。既存の `usize` / `u64` typed serde
decode により production code の変更は不要だった。selfhost/native manifest parser、source producer、
current-source stage0 artifact/runtime、Mac/Linux matrix、EC-M2-02/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-input-numeric-boundaries.md`。

2026-07-29 に EC-M3-01 version 1 manifest の fractional unsigned numeric boundary を追加した。
`0.5` / `1.5` を span と sampling の全 unsigned fieldへ入力し、Rust canonical parser が graph 構築前に
`ValidationInputError::Json` として reject することを
`parse_manifest_rejects_fractional_unsigned_numeric_fields` で固定した。typed serde decode に委譲し、
小数の丸め・切り捨てによる sampling semantics の変質を許可しない。selfhost/native manifest parser、
source producer、current-source stage0 artifact/runtime、Mac/Linux matrix、EC-M2-02/EC-M3 aggregate は残件。
Evidence: `docs/adr/decisions-v0.2-validation-input-numeric-boundaries.md`。

さらに公開 `lsharp validate <manifest> --format json --emit-manifest` へ fractional unsigned numeric
boundary を接続した。span と sampling の全6 fieldで exit `1`、空 stdout、manifest file 未生成、
`manifest` / `floating point` を含む stderr を固定した。default EmbeddedCli は既存 selfhost source の
`vector-push-single-rooted-v3` 未定義で停止するため、既存 component artifact を指定した Rust driver CLI
laneで検証した。selfhost/native CLI、MCP、current-source stage0 artifact/runtime、Mac/Linux matrix、
EC-M2-03/EC-M3 aggregate は残件。Evidence: `docs/adr/decisions-v0.2-validation-input-numeric-boundaries.md`。

同じ公開 CLI laneで `u64::MAX + 1` の unsigned numeric overflow も span / sampling の全6 fieldに対して
exit `1`、空 stdout、manifest file 未生成、manifest input error の stderr となることを固定した。
default EmbeddedCli blockerを避けた既存 component artifact 指定の Rust driver CLI evidenceであり、
selfhost/native CLI、MCP、current-source stage0 artifact/runtime、Mac/Linux matrix、EC-M2-03/EC-M3 aggregate
は残件。Evidence: `docs/adr/decisions-v0.2-validation-input-numeric-boundaries.md`。

2026-07-29 に version 1 manifest の unsigned numeric `null` boundary を追加した。span / sampling の全6
fieldで `null` を 0 や省略へ変換せず、Rust canonical parser が graph 構築前に
`ValidationInputError::Json` として拒否することを `parse_manifest_rejects_null_unsigned_numeric_fields`
で固定した。selfhost/native manifest parser、公開 CLI/MCP、current-source stage0 artifact/runtime、
Mac/Linux matrix、EC-M2-02/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-input-numeric-boundaries.md`。

2026-07-29 に MCP `lsharp_validate` の manifest schema へ unsigned numeric boundary を反映した。
`tools/list` が `nodes[].span.start/end` と `evidence[].execution.sampling.cases/seed/shrinks[]/coverage.*`
を `type: integer`・`minimum: 0` として公開し、input/output で同じ schema helper を共有することを
新規 schema test と MCP suite 44件で固定した。これは static schema と Rust MCP の verified partial
sliceであり、JSON Schema 実 validator、selfhost/native MCP、current-source stage0 artifact/runtime、
Mac/Linux matrix、EC-M3 aggregate は未完了である。Evidence:
`docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-29 に EC-M3-01 の MCP manifest runtime numeric boundary を追加した。direct `manifest` string
input の span / sampling 全6 fieldで fractional、`null`、`u64::MAX + 1` を `isError: true` の
`validation manifest の parse` error として拒否し、report/canonical manifestを成功値として返さない
18ケースを `mcp_server::tests` 45件で固定した。既存 Rust typed serde parser を MCP toolまで接続した
verified partial sliceであり、selfhost/native MCP、current-source stage0 artifact/runtime、Mac/Linux
matrix、JSON Schema実 validator、EC-M3 aggregate は未完了である。Evidence:
`docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-29 に EC-M3-01 の MCP typed edge schema boundary を追加した。`tools/list` の manifest
`edges[]` が公開 intent-graph schema と同じ6 relation variantを `oneOf` で宣言し、stable IDの
namespace/key pattern と evidence/review/invalidation subject kind enum を input/output 共通 schema
へ反映することを新規 parity test と MCP suite 46件で固定した。static schema と Rust MCP の verified
partial sliceであり、JSON Schema 実 validator、selfhost/native MCP、current-source stage0 artifact/runtime、
Mac/Linux matrix、EC-M3 aggregate は未完了である。Evidence:
`docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-29 に EC-M3-01 の MCP `manifest_file` runtime numeric boundary を追加した。file input の
`span.start/end` と `sampling.cases/seed/shrinks[]/coverage.*` 全6 fieldで fractional、`null`、
`u64::MAX + 1` を `isError: true` の `validation manifest の parse` error として拒否し、report/
canonical manifestを返さない18ケースを `mcp_server::tests` 47件で固定した。既存 Rust typed serde
parser を `manifest_file` routeまで接続した verified partial sliceであり、selfhost/native MCP、
current-source stage0 artifact/runtime、JSON Schema実 validator、Mac/Linux matrix、EC-M3 aggregate は
未完了である。Evidence: `docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-29 に EC-M3-01 の JSON Schema 実 validator boundary を追加した。canonical intent graph schema と
MCP `lsharp_validate` input/output schemaを Draft 2020-12 validatorで実行し、canonical fixtureの valid
roundtrip、fractional、`null`、`u64::MAX + 1`、typed subject kind mismatch の4 rejectを固定した。
Rust `u64` と schemaの整数上限を揃えるため、unsigned 6 fieldへ `maximum: 18446744073709551615` を追加した。
`mcp_server::tests` 48件で verifiedした Rust-host schema sliceであり、selfhost/native MCP、current-source
stage0 artifact/runtime、Mac/Linux matrix、EC-M3 aggregate は未完了である。Evidence:
`docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-29 に EC-M3-01 の MCP validation report schema boundary を追加した。`intent-validation.schema.json`
の external `$ref` を canonical intent graph schema resource として解決し、実際の
`lsharp_validate` `include_manifest: true` output（report と inline manifest）を Draft 2020-12 validator
で検証する valid roundtrip と、未知 `status` の reject を固定した。MCP suite 49件、clippy、rustfmt、
diff check、docs auditを通過した Rust-host verified partial sliceであり、native/selfhost MCP producer、
current-source stage0 artifact/runtime、Mac/Linux matrix、EC-M3 aggregate は残件である。Evidence:
`docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-29 に EC-M3-01 の MCP output schema parity boundary を追加した。canonical report schema と
`tools/list` の output schema を同期し、unknown top-level field、trace gap の未知 code / 空 subject ID、
64-bit counter 上限超過を両 validator が reject することを新規 matrix で固定した。report counter 5項目の
`maximum`、trace gap の code enum / non-empty ID、strict object boundary を canonical/MCP に反映し、MCP
suite 50件、clippy、rustfmt、diff check、docs auditを通過した Rust-host verified partial sliceである。
selfhost/native MCP producer、current-source stage0 artifact/runtime、Mac/Linux matrix、EC-M3 aggregate は
残件。Evidence: `docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-29 に EC-M3-01 の MCP provenance schema parity boundary を追加した。canonical manifest が要求する
evidence provenance の `producer` / `tool_version` / `timestamp` を MCP input/output schema でも
`minLength: 1` とし、3 fields の空文字を canonical/MCP 両 validator が reject する matrix を固定した。
MCP suite 50件、clippy、rustfmt、diff check、docs auditを通過した Rust-host verified partial sliceであり、
selfhost/native MCP producer、current-source stage0 artifact/runtime、Mac/Linux matrix、EC-M3 aggregate は
残件。Evidence: `docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-29 に EC-M3-01 の MCP provenance runtime boundary を追加した。`producer` / `tool_version` /
`timestamp` を空文字へ変異させた canonical fixtureを direct `manifest` と `manifest_file` の両 routeへ入力し、
6ケースすべてで `isError: true`、`structuredContent` なし、field名付き text error の fail-closed を固定した。
focused `review_registry_tests` 14件、clippy、rustfmt、diff check、docs auditを通過した Rust-host verified
partial sliceであり、全 MCP suite は既存 compile-run artifact `Invalid argument (os error 22)` 1件を含むため
今回の gateから除外した。selfhost/native MCP、current-source stage0 artifact/runtime、Mac/Linux matrix、
EC-M3 aggregate は残件。Evidence: `docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-29 に MCP `lsharp_compile_run` の共有 temp directory race を閉じた。固定名の directory を呼び出しごとに
削除していたため、別 process / test の同時実行で `Main.ls` / `Main.wasm` が相互に消え、`[LS5001] Invalid argument
(os error 22)` になり得た。呼び出しごとに PID・時刻・sequence を含む専用 directory を作り、RAII cleanupで
成功・失敗を問わず回収する。unique path / cleanup の RED→GREEN と実 MCP compile/run を含む `mcp_server::tests`
52件、clippy、rustfmt、diff check、docs auditを通過した Rust-host verified partial sliceである。selfhost/native
MCP、current-source artifact/runtime、Mac/Linux matrix、EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-29 に version 1 manifest envelope の duplicate top-level key boundary を追加した。
`schema_version` / `nodes` / `reviews` / `evidence` / `edges` の同名 JSON key を最後の値や空配列へ
上書きせず、parser 入口の `ValidationInputError::Json` として拒否する5ケースを
`parse_manifest_rejects_duplicate_top_level_fields` で固定した。既存 serde decode の fail-closed
behaviorを回帰テストと ADR に昇格した Rust canonical verified partial sliceであり、production codeの
変更はない。selfhost/native manifest parser、CLI/MCP parity、current-source artifact/runtime、Mac/Linux
matrix、EC-M2/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-manifest-duplicate-top-level.md`。

2026-07-29 に同じ duplicate top-level key boundary を公開 `lsharp validate` へ接続した。
`manifest_input_cli` の fixture が exit `1`、空 stdout、`--emit-manifest` file 未生成、stderr の
`duplicate` / `schema_version` 診断を返すことを固定した。Rust-host CLI verified partial sliceであり、
selfhost/native manifest parser、MCP report/exit parity、current-source stage0 artifact/runtime、
Mac/Linux matrix、EC-M2/EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-validation-manifest-duplicate-top-level.md`。

2026-07-29 に同じ duplicate top-level key boundary を Rust MCP `lsharp_validate` へ接続した。
`schema_version` の duplicate JSON string を direct `manifest` と `manifest_file` の両 routeへ入力し、
`isError: true`、`structuredContent` なし、`validation manifest の parse` / `duplicate` /
`schema_version` を含む text errorを2ケースで固定した。Rust-host MCP verified partial sliceであり、
selfhost/native MCP producer、current-source stage0 artifact/runtime、Mac/Linux matrix、EC-M3 aggregate は
残件。Evidence: `docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-29 に project config 由来の manifest path safety を公開 `lsharp validate` へ接続した。
`[validation].manifest` の absolute path と project root外を指す symlink を、`..` と同じく
non-zero exit・空 stdout・path boundary診断で拒否する2ケースを追加し、`validate_cli` 全29件で
root/nested discovery、missing config、relative/absolute/traversal/symlink boundaryを固定した。
Rust-host CLI verified partial sliceであり、selfhost/native、current-source stage0 artifact/runtime、
Mac/Linux matrix、EC-M2-03 aggregate は残件。Evidence: `docs/adr/decisions-v0.2-validation-config.md`。

2026-07-29 に project config manifest path safety の残るCLI境界も接続した。empty path、missing file、
directory targetを report生成前に non-zero exit・空 stdout・個別診断で拒否する3ケースを追加し、
`validate_cli` 全32件で project-relative、`..`、absolute、empty、missing、regular-file、外部symlinkを
固定した。Rust-host CLI verified partial sliceであり、selfhost/native、current-source stage0 artifact/runtime、
Mac/Linux matrix、EC-M2-03 aggregate は残件。Evidence: `docs/adr/decisions-v0.2-validation-config.md`。

2026-07-29 に MCP `lsharp_validate` の manifest unknown top-level field runtime boundaryを追加した。
`unexpected` fieldを direct object、JSON string、`manifest_file` の3 routeへ入力し、`isError: true`、
`structuredContent` なし、parse errorとfield名を返す fail-closed contractを固定した。MCP
`review_registry_tests` 16件のRust-host verified partial sliceであり、selfhost/native MCP producer、
current-source stage0 artifact/runtime、Mac/Linux matrix、EC-M3 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-mcp-validation-manifest.md`。

2026-07-29 に failed independent review の complete graph source fixture を native smoke に追加した。
`method=review`、`outcome=fail`、`independence=independent-review` は JSON/text とも `status: unknown`、
exit `2`、`independent_reviews: 0`、stderr 空となることを固定した。Rust canonical の
RED→GREEN を native source-file contract へ接続する fake Lima/provenance harness の verified partial
sliceであり、current source-commit に一致する実 stage0 artifact/runtime、selfhost/native/MCP parity、
Mac/Linux matrix、EC-M2-03 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-native-validation-failed-independent-review.md`。

2026-07-29 に native source-file smoke の manifest roundtrip boundary を追加した。source fixtureから
`--emit-manifest` した version 1 manifestを positional `validate <manifest> --format json` へ再入力し、
source report と manifest report の byte-for-byte 一致、`unknown` / exit `2`、stderr 空を要求する。
これは Rust CLI の source/emitted-manifest parity を native contract へ接続する fake Lima/provenance
harness の verified partial sliceであり、current source-commit に一致する実 stage0 artifact/runtime、
selfhost/native/MCP parity、Mac/Linux matrix、EC-M2-03 aggregate は残件。Evidence:
`docs/adr/decisions-v0.2-native-validation-manifest-roundtrip.md`。Rust oracle focused test は
default EmbeddedCli build の既存 `vector-push-single-rooted-v3` 未定義で停止し、GREEN evidence には
算入していない。

2026-07-29 に `selfhost/src/Tools/Validation/Stale.ls` の rooted vector helper owner importを修正した。
`Syntax.Parser` の直接 import を追加し、default EmbeddedCli build が `vector-push-single-rooted-v3`
undefinedで停止する既存 blockerを解消した。RED→GREEN の selfhost stale validation test、Rust driver
の `validate_cli` 32件、`validate_review_registry` 2件、`manifest_input_cli` 8件が passした verified
sliceである。native stage0 current-source/runtime、MCP、Mac/Linux matrix、EC-M2-03 aggregate は残件。
Evidence: `docs/adr/decisions-v0.2-selfhost-stale-parser-import.md`。

2026-08-01 に EC-M2-01 の project-level duplicate 検査を shared fixtureへ統合した。nested `module` /
`private` / `impl` の同一 typed IDを Rust source adapter と selfhost/native source adapter の両方で
stable duplicate-node code `2`、最初の declaration span、現在の declaration span、IDを保持して拒否する。
Rust `validation_source` 62件、selfhost source-adapter E2E 41件、Mac Apple Silicon current-source
stage0 producer（App.Cli E2E 829.28秒）と source-file smoke、Linux x86_64 actual stage1→stage2→stage3
fixed-point（stage2/stage3 SHA-256一致、code length 11,332,908）と packaged stage0 source-file smokeを
通過した verified partial sliceである。native packageの実行証跡は source commit `197ce48d` に束縛し、
その後の doc/schema-only fast-forward `ed72cb59` の後も selfhost source treeは変更されていない。
project graph aggregate、ID省略の全仕様、manifest/MCP/公開 surface、EC-M2-01全体の completion evidence
は残る。Evidence: `docs/adr/decisions-v0.2-native-validation-project-duplicate.md`。

2026-08-02 に packaged stage0 releaseの identity `artifact_digest` を stage0 manifestの compiler bytesへ
束縛した。別 artifact digestの identityは provider snapshot/source commitが正しくても archive生成前に
拒否し、正しい digestの package成功と archive未生成を `test-native-stage0-release-package.sh` で確認した
verified partial sliceである。provider API/auth取得・意味検証、current-source Linux runtime、Mac/Linux両
targetの packaged provenance/rollback bytes parityは未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 /
M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay lockがないためLinux replayは未実行で、
別セッション所有のLima/cargo processも変更していない。blockerの再現 command は
`current_head="$(git rev-parse --verify HEAD)"; find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'`
と `find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)`。
Evidence:
`docs/adr/decisions-v0.3-native-stage0-release-artifact-binding.md`。

2026-08-02 に stage0 directoryの embedded `review-evidence-identity.json` と明示 identityを比較し、不一致時に
explicit値で silently overwriteせず archive生成前に拒否する conflict boundaryを追加した。正しい identity一致時の
package成功、不一致時の安定診断と archive未生成を `test-native-stage0-release-package.sh` で確認した verified
partial sliceである。provider API/auth取得・意味検証、current-source Linux runtime、Mac/Linux両 targetの
packaged provenance/rollback bytes parityは未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` の
まま残す。current-source manifest/expected replay lockが現HEADに一致しないためLinux replayは未実行で、別セッション
所有のLima/cargo/replayも変更していない。blockerの再現 commandは
`current_head="$(git rev-parse --verify HEAD)"; find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'`
と `find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)`。
Evidence: `docs/adr/decisions-v0.3-native-stage0-release-embedded-identity-conflict.md`。

2026-08-02 に native official release gate の Linux hostgen replay lock preflight を provider snapshot / identity /
source smoke evidence より前へ移動した。別セッションが live replayを所有中で provider identityが欠ける fixtureでも、
provider/package/fetch/smokeへ進まず exit `90` と lock owner/artifact/VM path を返し、release outputを作成しない
RED→GREENを `test-native-official-release-replay-lock.sh` で確認した verified partial sliceである。provider API/auth取得・
意味検証、current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、
M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay lockが
現HEADに一致せず、別セッション所有のLima/cargo/replay processも変更していないため、Linux replay・stage regeneration・
full buildは未実行である。blockerの再現 commandは
`current_head="$(git rev-parse --verify HEAD)"; find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'`
と `find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)`。
Evidence: [`decisions-v0.3-native-official-release-replay-lock-precedence.md`](docs/adr/decisions-v0.3-native-official-release-replay-lock-precedence.md)。

2026-08-02 に release smoke の provider snapshot preflight を archive path確認・展開より前へ移動した。欠落 archiveと
trust-storeのみを指定した fixtureでも、archive lookupへ進まず all-or-none provider診断を返し、release smoke work
directoryを作成しない RED→GREENを `test-release-smoke-provider-snapshots.sh` で確認した verified partial sliceである。
live provider API/auth取得・意味検証、current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback
bytes parityは未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current-source
manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/cargo/replay processも変更していないため、
Linux replay・stage regeneration・full buildは未実行である。blockerの再現 commandは
`current_head="$(git rev-parse --verify HEAD)"; find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'`
と `find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)`。
Evidence: [`decisions-v0.3-release-smoke-provider-preflight-order.md`](docs/adr/decisions-v0.3-release-smoke-provider-preflight-order.md)。

2026-08-02 に stage0 release archiveの stagingから root-level raw `review-trust-store.snapshot` と
`review-lifecycle.snapshot` を除外した。stage0 inputへ private provider bytesを混入したときに archive listingへ漏れる
REDを確認し、identity projectionを保持したまま raw snapshotを公開 archiveへ含めない GREENを
`test-native-stage0-release-package.sh` で確認した verified partial sliceである。live provider API/auth取得・意味検証、
current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、M3-04-N1 /
M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay lockが現HEADに一致せず、
別セッション所有のLima/cargo/replay processも変更していないため、Linux replay・stage regeneration・full buildは未実行である。
blockerの再現 commandは
`current_head="$(git rev-parse --verify HEAD)"; find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'`
と `find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)`。
Evidence: [`decisions-v0.3-native-stage0-release-provider-snapshot-exclusion.md`](docs/adr/decisions-v0.3-native-stage0-release-provider-snapshot-exclusion.md)。

2026-08-02 に stage0 release package の input/output ownership boundaryを追加した。output directory が stage0 input
directory 自身またはその配下にある場合、staging directoryを入力 package内へ作成せず archive生成前に拒否する RED→GREENを
`test-native-stage0-release-package.sh` で確認し、stage0外の relative output directoryを受理する既存経路も維持した。
これは packaged stage0 path ownership の verified partial sliceであり、live provider API/auth取得・意味検証、current-source
Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 /
M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有の
Lima/cargo/replay processも変更していないため Linux replay・stage regeneration・full buildは未実行である。blockerの再現
commandは `current_head="$(git rev-parse --verify HEAD)"; find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name
manifest.json -path '*lsharp*'` と `find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name
'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)`。Evidence:
[`decisions-v0.3-native-stage0-release-output-boundary.md`](docs/adr/decisions-v0.3-native-stage0-release-output-boundary.md)。

2026-08-02 に release smoke の cleanup work directory safetyを追加した。`WORK_DIR` が `/`、`/tmp`、repository root、
repositoryの `target` または `target/ci` に解決される場合、archive/provider workへ進まず `unsafe release smoke work directory`
で拒否する RED→GREENを `test-release-smoke-provider-snapshots.sh` で確認した。task-owned leaf directoryを使う既存の
provider snapshot / rollback smoke は同じ harnessで維持した。これは cleanup ownershipの verified partial sliceであり、
live provider API/auth取得・意味検証、current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes
parityは未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay
lockが現HEADに一致せず、別セッション所有のLima/cargo/replay processも変更していないため Linux replay・stage regeneration・
full buildは未実行である。blockerの再現 commandは `current_head="$(git rev-parse --verify HEAD)"; find /tmp
/Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'` と `find /tmp
/Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)`。Evidence:
[`decisions-v0.3-release-smoke-workdir-safety.md`](docs/adr/decisions-v0.3-release-smoke-workdir-safety.md)。

2026-08-02 に `fetch-stage0.sh` の install destination safetyを追加した。`STAGE0_DIR` が `/`、`/tmp`、`/private/tmp`、
repository root、repositoryの `target` または `target/ci` に解決される場合、release URL validation・temporary workspace・curlより
前に安定診断で拒否する RED→GREENを `test-fetch-stage0-provider-url.sh` で確認した。これは fetch install destination ownershipの
verified partial sliceであり、live provider API/auth取得・意味検証、current-source Linux runtime、Mac/Linux両 targetの packaged
provenance/rollback bytes parityは未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current-source
manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/cargo/replay processも変更していないため Linux replay・stage
regeneration・full buildは未実行である。blockerの再現 commandは `current_head="$(git rev-parse --verify HEAD)"; find /tmp
/Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'` と `find /tmp
/Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)`。Evidence:
[`decisions-v0.3-fetch-stage0-install-directory-safety.md`](docs/adr/decisions-v0.3-fetch-stage0-install-directory-safety.md)。

2026-08-02 に `scripts/release.sh` の release version safetyを追加した。`VERSION` を archive directory、archive filename、
manifest metadataへ流す前に ASCII letters、digits、dot、underscore、hyphenだけへ限定し、`v1/unsafe` を release output作成前に
拒否する RED→GREENを `test-native-release-identity.py` で確認した。これは release version namespaceの verified partial sliceであり、
live provider API/auth取得・意味検証、current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは
未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay lockが
現HEADに一致せず、別セッション所有のLima/cargo/replay processも変更していないため Linux replay・stage regeneration・full buildは
未実行である。blockerの再現 commandは `current_head="$(git rev-parse --verify HEAD)"; find /tmp
/Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'` と `find /tmp
/Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)`。Evidence:
[`decisions-v0.3-release-version-path-safety.md`](docs/adr/decisions-v0.3-release-version-path-safety.md)。

2026-08-02 に native official release gate の source-smoke evidence output root safetyを追加した。`NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT` を canonicalizeし、repository
root、`target`、`target/ci`、`ci-artifacts`、`dist`、`stage0`、system temporary root、または cleaned release smoke root配下を package/output/runtime workより前に拒否する RED→GREENを
`test-native-official-release-snapshots.sh` で確認した。これは source-smoke evidence output ownershipの verified partial sliceであり、live provider API/auth取得・意味検証、current-source Linux runtime、
Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay lockが
現HEADに一致せず、別セッション所有のLima/cargo/replay processも変更していないため Linux replay・stage regeneration・full buildは未実行である。blockerの再現 commandは `current_head="$(git rev-parse --verify HEAD)"; find /tmp
/Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'` と `find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)`。Evidence:
[`decisions-v0.3-native-official-source-smoke-evidence-root-safety.md`](docs/adr/decisions-v0.3-native-official-source-smoke-evidence-root-safety.md)。

2026-08-02 に明示 review-lifecycle snapshot の semantic state preflightを追加した。`verify-native-release-identity.py` は
UTF-8のJSON object、object array、またはJSONL recordの各 `state` を `proposed` / `active` / `superseded` / `revoked` に限定し、
未知state・壊れたJSON・非object recordを fail-closed に拒否する RED→GREENを `test-native-release-identity.py` で確認した。これは
lifecycle state allowlistの verified partial sliceであり、provider snapshot regular-file/nonempty/digest/role binding、MCP semantic、
署名/authentication、sequence reducerの完了証拠ではない。live provider API/auth取得・意味検証、current-source Linux runtime、Mac/Linux両 targetの
packaged provenance/rollback bytes parityは未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current-source
manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/cargo/replay processも変更していないため Linux replay・stage regeneration・full buildは未実行である。
blockerの再現 commandは `current_head="$(git rev-parse --verify HEAD)"; find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'` と
`find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)`。Evidence:
[`decisions-v0.3-provider-lifecycle-state-preflight.md`](docs/adr/decisions-v0.3-provider-lifecycle-state-preflight.md)。

2026-08-02 に明示 review-lifecycle snapshot の duplicate sequence preflightを追加した。integer `sequence` を持つ recordについて同じ
`(review_id, sequence)` が複数回現れる入力を `duplicate review lifecycle sequence` で拒否する RED→GREENを
`test-native-release-identity.py` で確認した。これは duplicate sequence の verified partial sliceであり、sequence必須化・ordering/reducer、
provider snapshot regular-file/nonempty/digest/role binding、MCP semantic、署名/authenticationの完了証拠ではない。live provider API/auth取得・意味検証、
current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は
`[~]` のまま残す。current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/cargo/replay processも変更していないため Linux replay・stage
regeneration・full buildは未実行である。blockerの再現 commandは `current_head="$(git rev-parse --verify HEAD)"; find /tmp
/Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'` と `find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name
'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)`。Evidence:
[`decisions-v0.3-provider-lifecycle-sequence-duplicate-preflight.md`](docs/adr/decisions-v0.3-provider-lifecycle-sequence-duplicate-preflight.md)。

2026-08-02 に明示 review-lifecycle snapshot の sequence rollback preflightを追加した。同じ `review_id` の integer sequenceが入力順で `2 → 1` と
減少する eventを `review lifecycle sequence rollback` で拒否する RED→GREENを `test-native-release-identity.py` で確認した。これは sequence rollback の
verified partial sliceであり、state allowlist / duplicate sequence以外のsequence必須化・effective time ordering・state transition・payload reducer、provider snapshot
regular-file/nonempty/digest/role binding、MCP semantic、署名/authenticationの完了証拠ではない。live provider API/auth取得・意味検証、current-source Linux runtime、
Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay lockが
現HEADに一致せず、別セッション所有のLima/cargo/replay processも変更していないため Linux replay・stage regeneration・full buildは未実行である。blockerの再現 commandは
`current_head="$(git rev-parse --verify HEAD)"; find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'` と
`find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)`。Evidence:
[`decisions-v0.3-provider-lifecycle-sequence-rollback-preflight.md`](docs/adr/decisions-v0.3-provider-lifecycle-sequence-rollback-preflight.md)。

2026-08-02 に明示 review-lifecycle snapshot の `effective_at` preflightを追加した。存在する `effective_at` fieldが strict UTC/calendar timestampでない
`2024-02-30T00:00:00Z` eventを `review lifecycle effective_at must be a strict UTC timestamp` で拒否する RED→GREENを
`test-native-release-identity.py` で確認した。これは native provider inputの timestamp shape/calendar verified partial sliceであり、field必須化、sequence reducer・state transition・
effective-time ordering、provider snapshot regular-file/nonempty/digest/role binding、MCP semantic、署名/authenticationの完了証拠ではない。live provider API/auth取得・意味検証、
current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/cargo/replay processも変更していないため Linux replay・stage regeneration・full buildは未実行である。blockerの再現 commandは
`current_head="$(git rev-parse --verify HEAD)"; find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'` と
`find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)`。Evidence:
[`decisions-v0.3-provider-lifecycle-effective-at-preflight.md`](docs/adr/decisions-v0.3-provider-lifecycle-effective-at-preflight.md)。

2026-08-02 に明示 review-lifecycle snapshot の terminal state preflightを追加した。同じ review の `revoked` / `superseded` 後に続く
`active` event（sequence `1 → 2`）を `review lifecycle terminal state reactivation` で拒否する RED→GREENを
`test-native-release-identity.py` で確認した。これは sequenced provider inputの terminal-state reactivation verified partial sliceであり、initial state、完全な transition matrix・payload reducer・
effective-time ordering、provider snapshot regular-file/nonempty/digest/role binding、MCP semantic、署名/authenticationの完了証拠ではない。live provider API/auth取得・意味検証、
current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/cargo/replay processも変更していないため Linux replay・stage regeneration・full buildは未実行である。blockerの再現 commandは
`current_head="$(git rev-parse --verify HEAD)"; find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'` と
`find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)`。Evidence:
[`decisions-v0.3-provider-lifecycle-terminal-state-preflight.md`](docs/adr/decisions-v0.3-provider-lifecycle-terminal-state-preflight.md)。

2026-08-02 に明示 review-lifecycle snapshot の initial state preflightを追加した。同じ reviewの最初の sequenced eventが `revoked` / `superseded` の場合を
`review lifecycle initial state must be one of active, proposed` で拒否する RED→GREENを `test-native-release-identity.py` で確認した。これは sequenced provider inputの
initial-state rule verified partial sliceであり、既存のstate allowlist・terminal reactivation・sequence rollback/duplicate・effective_at以外の完全な transition matrix、payload reducer、
provider snapshot regular-file/nonempty/digest/role binding、MCP semantic、署名/authenticationの完了証拠ではない。live provider API/auth取得・意味検証、current-source Linux runtime、
Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay lockが
現HEADに一致せず、別セッション所有のLima/cargo/replay processも変更していないため Linux replay・stage regeneration・full buildは未実行である。blockerの再現 commandは
`current_head="$(git rev-parse --verify HEAD)"; find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'` と
`find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)`。Evidence:
[`decisions-v0.3-provider-lifecycle-initial-state-preflight.md`](docs/adr/decisions-v0.3-provider-lifecycle-initial-state-preflight.md)。

2026-08-02 に明示 review-lifecycle snapshot の proposed-to-terminal transition preflightを追加した。`proposed sequence: 1` の後に `revoked` / `superseded sequence: 2` を置く eventを
`review lifecycle terminal transition requires active` で拒否する RED→GREENを `test-native-release-identity.py` で確認した。これは sequenced provider inputの proposed-to-terminal transition verified partial sliceであり、
既存の initial state・terminal reactivation・effective_at・sequence rollback/duplicate以外の完全な transition matrix、payload reducer、provider snapshot regular-file/nonempty/digest/role binding、
MCP semantic、署名/authenticationの完了証拠ではない。live provider API/auth取得・意味検証、current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、
M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/cargo/replay processも変更していないため Linux replay・stage regeneration・full buildは未実行である。blockerの再現 commandは
`current_head="$(git rev-parse --verify HEAD)"; find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'` と
`find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)`。Evidence:
[`decisions-v0.3-provider-lifecycle-terminal-transition-preflight.md`](docs/adr/decisions-v0.3-provider-lifecycle-terminal-transition-preflight.md)。

2026-08-02 に明示 review-lifecycle snapshot の active-state regression preflightを追加した。`proposed → active → proposed`（sequence `1 → 2 → 3`）eventを
`review lifecycle active state regression` で拒否する RED→GREENを `test-native-release-identity.py` で確認した。これは sequenced provider inputの active-to-proposed regression verified partial sliceであり、
既存の initial state・proposed terminal・terminal reactivation・effective_at・sequence rollback/duplicate以外の完全な transition matrix、payload reducer、provider snapshot regular-file/nonempty/digest/role binding、
MCP semantic、署名/authenticationの完了証拠ではない。live provider API/auth取得・意味検証、current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、
M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/cargo/replay processも変更していないため Linux replay・stage regeneration・full buildは未実行である。blockerの再現 commandは
`current_head="$(git rev-parse --verify HEAD)"; find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'` と
`find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)`。Evidence:
[`decisions-v0.3-provider-lifecycle-active-state-regression-preflight.md`](docs/adr/decisions-v0.3-provider-lifecycle-active-state-regression-preflight.md)。

2026-08-02 に明示 review-lifecycle snapshot の active self-transition preflightを追加した。`proposed → active → active`（sequence `1 → 2 → 3`）eventを
`review lifecycle active state self-transition` で拒否する RED→GREENを `test-native-release-identity.py` で確認した。これは sequenced provider inputの active self-transition verified partial sliceであり、
既存の active regression・proposed terminal・initial state・terminal reactivation・effective_at・sequence rollback/duplicate以外の完全な transition matrix、payload reducer、provider snapshot regular-file/nonempty/digest/role binding、
MCP semantic、署名/authenticationの完了証拠ではない。live provider API/auth取得・意味検証、current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、
M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/cargo/replay processも変更していないため Linux replay・stage regeneration・full buildは未実行である。blockerの再現 commandは
`current_head="$(git rev-parse --verify HEAD)"; find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'` と
`find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)`。Evidence:
[`decisions-v0.3-provider-lifecycle-active-state-self-transition-preflight.md`](docs/adr/decisions-v0.3-provider-lifecycle-active-state-self-transition-preflight.md)。

2026-08-02 に明示 review-lifecycle snapshot の proposed self-transition preflightを追加した。`proposed → proposed`（sequence `1 → 2`）eventを
`review lifecycle proposed state self-transition` で拒否する RED→GREENを `test-native-release-identity.py` で確認した。これは sequenced provider inputの proposed self-transition verified partial sliceであり、
既存の active self-transition/regression・proposed terminal・initial state・terminal reactivation・effective_at・sequence rollback/duplicate以外の完全な transition matrix、payload reducer、provider snapshot regular-file/nonempty/digest/role binding、
MCP semantic、署名/authenticationの完了証拠ではない。live provider API/auth取得・意味検証、current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、
M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/cargo/replay processも変更していないため Linux replay・stage regeneration・full buildは未実行である。blockerの再現 commandは
`current_head="$(git rev-parse --verify HEAD)"; find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'` と
`find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)`。Evidence:
[`decisions-v0.3-provider-lifecycle-proposed-state-self-transition-preflight.md`](docs/adr/decisions-v0.3-provider-lifecycle-proposed-state-self-transition-preflight.md)。

2026-08-02 に明示 review-lifecycle snapshot の sequence lower-bound preflightを追加した。`sequence: 0` の初期 `proposed` eventを
`review lifecycle sequence must be a positive integer` で拒否する RED→GREENを `test-native-release-identity.py` で確認した。これは native provider inputの
sequence positive-integer verified partial sliceであり、sequence field必須化、既存の sequence rollback/duplicate、完全な transition matrix、payload reducer、
effective-time ordering、provider snapshot regular-file/nonempty/digest/role binding、MCP semantic、署名/authenticationの完了証拠ではない。live provider API/auth取得・意味検証、
current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/cargo/replay processも変更していないため Linux replay・stage regeneration・full buildは未実行である。blockerの再現 commandは
`current_head="$(git rev-parse --verify HEAD)"; find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'` と
`find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)`。Evidence:
[`decisions-v0.3-provider-lifecycle-sequence-positive-preflight.md`](docs/adr/decisions-v0.3-provider-lifecycle-sequence-positive-preflight.md)。

2026-08-02 に明示 review-lifecycle snapshot の sequence presence preflightを追加した。`sequence` fieldを欠落させた初期 `proposed` eventを
`review lifecycle sequence is required` で拒否する RED→GREENを `test-native-release-identity.py` で確認した。これは native provider inputの
sequence presence verified partial sliceであり、provider snapshotの取得・認証、完全な transition matrix/reducer、state semantics、MCP semantic、署名/authenticationの完了証拠ではない。
関連する identity preparation、official snapshot、replay-lock、release-smoke、stage0 package の fixtureにも `sequence: 1` を明示した。live provider API/auth取得・意味検証、
current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replay processも変更していないため Linux replay・stage regeneration・full buildは未実行である。blockerの再現 commandは
`current_head="$(git rev-parse --verify HEAD)"; find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'` と
`find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)`。Evidence:
[`decisions-v0.3-provider-lifecycle-sequence-required-preflight.md`](docs/adr/decisions-v0.3-provider-lifecycle-sequence-required-preflight.md)。

2026-08-02 に明示 review-lifecycle snapshot の sequence continuity preflightを追加した。同じ `review_id` の `sequence: 1` の次に `sequence: 3` が現れる
eventを `review lifecycle sequence gap` で拒否する RED→GREENを `test-native-release-identity.py` で確認した。これは native provider inputの
sequence continuity verified partial sliceであり、既存の sequence required/positive/duplicate/rollback、完全な transition matrix/reducer、Rust/native reducer parity（後続 parity ADRで検証）、
provider snapshot semantics、MCP semantic、署名/authenticationの完了証拠ではない。live provider API/auth取得・意味検証、current-source Linux runtime、Mac/Linux両 targetの
packaged provenance/rollback bytes parityは未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay lockが現HEADに一致せず、
別セッション所有のLima/QEMU/replay processも変更していないため Linux replay・stage regeneration・full buildは未実行である。blockerの再現 commandは
`current_head="$(git rev-parse --verify HEAD)"; find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'` と
`find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)`。Evidence:
[`decisions-v0.3-provider-lifecycle-sequence-gap-preflight.md`](docs/adr/decisions-v0.3-provider-lifecycle-sequence-gap-preflight.md)。

2026-08-02 に native provider lifecycle snapshot の `review_id` required preflightを追加した。`review_id` を欠落させた
`sequence: 1` / `proposed` recordを `review lifecycle review_id is required` で拒否する RED→GREENを `test-native-release-identity.py` で確認し、
Rust `ReviewLifecycleEvent::new` の空 `review_id` 拒否も focused testで対応付けた。これは provider inputの required-field verified partial sliceであり、
stable IDの selfhost/MCP全入力 route parity、完全な transition matrix/reducer、live provider API/auth、current-source Linux runtime、Mac/Linux両 targetの packaged
provenance/rollback bytes parityは未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay lockが
現HEADに一致せず、別セッション所有のLima/QEMU/replaydも稼働中のため Linux replay・stage regeneration・full buildは未実行である。Evidence:
[`decisions-v0.3-provider-lifecycle-review-id-required.md`](docs/adr/decisions-v0.3-provider-lifecycle-review-id-required.md)。

2026-08-02 に native provider lifecycle snapshot の stable `review_id` wire-format preflightを追加した。非空だが key segmentが欠けた
`review:checkout` を Rust canonical と native providerの双方で拒否する RED→GREENを確認した。Rust lifecycle 6件、review wire 8件、native
identity 27件と関連 offline harness、Python/Bash syntaxを通過した。これは native provider routeとRust canonical `ReviewId::parse` の
形式 parity verified partial sliceであり、selfhost/MCPの全入力 route、完全な transition matrix/reducer、live provider API/auth、current-source
Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は
`[~]` のまま残す。current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydも稼働中のため Linux
replay・stage regeneration・full buildは未実行である。Evidence:
[`decisions-v0.3-provider-lifecycle-review-id-wire-format.md`](docs/adr/decisions-v0.3-provider-lifecycle-review-id-wire-format.md)。

2026-08-02 に provider-required release identityの auth-context binding preflightを追加した。identityに trust-store / review-lifecycle の digestが
埋め込まれていても、`--require-provider-input` に実ファイル pathが無い呼び出しを `provider auth context is required` で fail-closed に拒否する
RED→GREENを `test-native-release-identity.py` で確認した。これは明示 snapshot input bindingの verified partial sliceであり、provider API/auth取得・
意味検証、current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityの完了証拠ではない。M3-04-N1 / M3-05-N2 /
M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydも
稼働中のため Linux replay・stage regeneration・full buildは未実行である。blockerの再現 commandは
`current_head="$(git rev-parse --verify HEAD)"; find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'` と
`find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)`。Evidence:
[`decisions-v0.3-native-provider-auth-context-binding.md`](docs/adr/decisions-v0.3-native-provider-auth-context-binding.md)。

2026-08-02 に native provider lifecycle snapshotの `effective_at` ordering parityを追加した。strict UTC/calendar shapeだけでなく、同じ reviewの sequence順で
`effective_at` が過去へ戻る eventを `review lifecycle effective_at rollback` で fail-closed に拒否する RED→GREENを native identity testとRust reducer focused testで確認した。
これは effective-at ordering parityの verified partial sliceであり、既存 sequence/state transition、provider API/auth取得・意味検証、current-source Linux runtime、
Mac/Linux両 targetの packaged provenance/rollback bytes parityの完了証拠ではない。M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydも稼働中のため Linux replay・stage regeneration・full buildは未実行である。
blockerの再現 commandは `current_head="$(git rev-parse --verify HEAD)"; find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'` と
`find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)`。Evidence:
[`decisions-v0.3-provider-lifecycle-effective-at-ordering-parity.md`](docs/adr/decisions-v0.3-provider-lifecycle-effective-at-ordering-parity.md)。

2026-08-02 に native selfhost MCP `initialize` の `serverInfo` を Rust canonical route と揃えた。native shim の
`"lsharp-native"` を `"lsharp"` へ変更し、package version `0.1.0` とともに固定する RED→GREEN を focused
route testで確認した。これは MCP JSON-RPC envelope metadata の verified partial parity slice であり、Rust MCP
全tool parity、package-install semantics、live provider API/auth acquisition・意味検証、current-source Linux runtime、
Mac/Linux 両 target の packaged provenance/rollback bytes parity の完了証拠ではない。関連項目は `[~]` のまま残す。
Evidence: [`decisions-v0.3-native-mcp-initialize-server-identity-parity.md`](docs/adr/decisions-v0.3-native-mcp-initialize-server-identity-parity.md)。

2026-08-02 に native selfhost MCP `tools/list` の descriptor order を Rust canonical `list_tools()` と揃えた。同じ
13 routeを LSP-first で返していた native shimを、Rustの check→validate→LSP→offline/package→compile/run→errors/search
順へ explicit canonical tupleで並べ、全配列順を focused test の RED→GREEN で固定した。これは MCP route envelope の
deterministic order verified partial parity sliceであり、Rust MCP 全semantic parity、package-install semantics、live provider
API/auth acquisition・意味検証、current-source Linux runtime、Mac/Linux 両 targetの packaged provenance/rollback bytes parity
の完了証拠ではない。関連項目は `[~]` のまま残す。Evidence: [`decisions-v0.3-native-mcp-tools-list-order-parity.md`](docs/adr/decisions-v0.3-native-mcp-tools-list-order-parity.md)。

2026-08-02 に native selfhost MCP の JSON-RPC request id envelope を Rust canonical transport と揃えた。id field が欠落した
notificationだけを無応答とし、明示 `id: null` は response の `id: null` として保持する RED→GREENを focused route testで
確認した。これは MCP envelope の null/missing distinction verified partial parity sliceであり、MCP 全 error envelope/semantic
parity、package-install semantics、live provider API/auth acquisition・意味検証、current-source Linux runtime、Mac/Linux 両
target の packaged provenance/rollback bytes parity の完了証拠ではない。関連項目は `[~]` のまま残す。Evidence:
[`decisions-v0.3-native-mcp-null-request-id-envelope.md`](docs/adr/decisions-v0.3-native-mcp-null-request-id-envelope.md)。

2026-08-02 に native selfhost MCP `tools/call` の missing-name error envelope を Rust canonical transport と揃えた。`params.name` が
欠落または非文字列の場合も、Rust と同じ result-level `isError: true`・`content[0].text: "tool not found"` を返す RED→GREENを native
82 tests と Rust MCP 89 tests で確認した。明示的な unknown tool name の既存契約は再実装せず、今回は missing-name boundaryだけを固定した
verified partial sliceである。MCP 全 error/semantic parity、package-install semantics、live provider API/auth acquisition・意味検証、
current-source Linux runtime、Mac/Linux 両 target の packaged provenance/rollback bytes parity の完了証拠ではないため、関連項目は `[~]`
のまま残す。Evidence: [`decisions-v0.3-native-mcp-tools-call-missing-name-envelope.md`](docs/adr/decisions-v0.3-native-mcp-tools-call-missing-name-envelope.md)。

2026-08-02 に native selfhost MCP `tools/call` の non-object `params` envelope を Rust canonical transport と揃えた。`params: []` などの
非 object 入力を Rust と同じ empty params として扱い、result-level `isError: true`・`content[0].text: "tool not found"` を返す RED→GREENを
native 83 tests と Rust MCP 90 tests で確認した。これは non-object params boundaryの verified partial sliceであり、MCP 全 error/semantic
parity、package-install semantics、live provider API/auth acquisition・意味検証、current-source Linux runtime、Mac/Linux 両 target の
packaged provenance/rollback bytes parity の完了証拠ではないため、関連項目は `[~]` のまま残す。Evidence:
[`decisions-v0.3-native-mcp-tools-call-non-object-params-envelope.md`](docs/adr/decisions-v0.3-native-mcp-tools-call-non-object-params-envelope.md)。

2026-08-02 に明示 provider lifecycle snapshot の caller-clock freshness preflightを追加した。recordの
`effective_at` が identityの `now` より未来の場合を `review lifecycle effective_at is after identity now` で拒否する
RED→GREENを `test-native-release-identity.py` で確認した。これは既存の snapshot regular-file / nonempty / digest、strict
timestamp shape、同一 review内の effective_at ordering、attestation expiry、auth-context bindingを再実装しない verified
partial sliceである。live provider API/auth取得・署名意味検証、current-source Linux runtime、Mac/Linux両 targetの packaged
provenance/rollback bytes parityは未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydも稼働中のため Linux
replay・stage regeneration・full buildは未実行である。Evidence:
[`decisions-v0.3-provider-lifecycle-future-effective-at.md`](docs/adr/decisions-v0.3-provider-lifecycle-future-effective-at.md)。

2026-08-02 に packaged release-smoke の release archive と rollback compatibility archive の入力を symlink でない
regular file に限定した。anchor checksum と同じ bytes を指す rollback archive symlink を offline fixture の RED
で受け入れることを確認し、`regular file without symlink` で fail-closed に拒否する GREEN を
`test-release-smoke-provider-snapshots.sh` で固定した。これは archive input path の provenance-safe boundaryだけを
閉じる verified partial sliceであり、既存の manifest/anchor/checksum/rollback payload 契約を再実装しない。
live provider API/auth取得・意味検証、current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback
bytes parityは未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current-source
manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydも稼働中のため Linux replay・
stage regeneration・full buildは未実行である。Evidence:
[`decisions-v0.3-packaged-archive-input-regular-file.md`](docs/adr/decisions-v0.3-packaged-archive-input-regular-file.md)。

2026-08-02 に rollback compatibility archive の再帰 smoke へ manifest の `version` を nested `VERSION` として渡し、
rollback executable の `--version` output と manifest version の parity を固定した。manifest/anchor/checksum を
再生成しても `lsharp 9.9.9` を返す rollback fixture を従来実装が受け入れる RED と、`packaged CLI version mismatch` で
fail-closed にする GREEN を `test-release-smoke-provider-snapshots.sh` で確認した。これは rollback executable version の
offline parityだけを閉じる verified partial sliceであり、既存の manifest/anchor/checksum/payload/archive input 契約を
再実装しない。live provider API/auth取得・意味検証、current-source Linux runtime、Mac/Linux両 targetの packaged
provenance/rollback bytes parityは未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydも稼働中のため Linux
replay・stage regeneration・full buildは未実行である。Evidence:
[`decisions-v0.3-packaged-rollback-version-output-parity.md`](docs/adr/decisions-v0.3-packaged-rollback-version-output-parity.md)。

2026-08-02 に残件を細かな packaged edge ではなく、M3-04-N1 / M3-05-N9 の current-source two-target runtime gate
（compile/build → source-bound artifact/manifest → packaged/stage0 → Mac/Linux runtime）として再監査した。監査時の
`HEAD` `f36b51539c1d903d89005f02d4bd9a9fe11770f0` に source commit が一致する manifest と task-owned expected replay lockは
見つからず、Lima hostagent/QEMU/replayd は別セッション所有だったため、`native-official-release-local.sh`、stage
regeneration、Linux replay、full buildは起動していない。これは gate 未実行の blocker evidenceであり、live provider
API/auth取得・意味検証、current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、
M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。再現 command、再開条件、次に一度だけ実行する gate の記録は
[`decisions-v0.3-current-source-two-target-runtime-gate-blocker.md`](docs/adr/decisions-v0.3-current-source-two-target-runtime-gate-blocker.md) に固定した。

2026-08-02 に native release identity verifier の lifecycle snapshotを、Rust `ReviewLifecycleRegistry::from_events` と
selfhost reducerと同じ `(review_id, sequence)` canonical orderで reduceする parityを追加した。`revoked sequence: 2` →
`active sequence: 1` の逆順でも、canonical orderでは有効な `active` → `revoked` になる fixtureを RED→GREENで固定し、
既存の rollback/gap/duplicate/transition/effective_at 診断は維持した。native identity 33件、Rust wire declaration-order 1件、
selfhost lifecycle reducer 1件がGREENである。これは lifecycle producer declaration-order parityの verified partial sliceであり、
live provider/auth取得・署名検証、current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは
未検証のため、EC-M3-01〜05 と M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。Evidence:
[`decisions-v0.3-native-lifecycle-declaration-order-parity.md`](docs/adr/decisions-v0.3-native-lifecycle-declaration-order-parity.md)。

2026-08-02 に native MCP `lsharp_validate --include-manifest` の report/manifest projection parityを追加した。Rust canonical
と同じ graph から投影されるべき report 内 `manifest` と別途 emit された manifest が、どちらも schema-valid でも異なる場合を
`native validate report manifest projection mismatch` で fail-closed に拒否する RED→GREENを fake native harness で固定した。
report に embedded manifest が無い既存 native producerは維持し、receipt/provider identity/schema の個別契約を再実装しない。
これは native MCP の report/manifest semantic parity の verified partial sliceであり、native cryptographic verification、live
provider/auth取得、current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、
EC-M3-01〜05 と M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay lockが
現HEADに一致せず、別セッション所有のLima/QEMU/replaydも稼働中のため Linux replay・stage regeneration・full buildは未実行である。
Evidence: [`decisions-v0.3-native-mcp-report-manifest-projection-parity.md`](docs/adr/decisions-v0.3-native-mcp-report-manifest-projection-parity.md)。

2026-08-02 に native MCP `lsharp_validate` report の `review_attestations[]` source-attestation projection parityを追加した。
Rust canonical reportと同じ14フィールド（named identity、canonical bytes、state、span、expires_at、positive sequence）を native
output schema と postflight validatorへ接続し、valid fixtureを受理しつつ missing/extra/invalid state/byte overflow/span不正を
fail-closed にする RED→GREENを固定した。これは report wire/projection の verified partial sliceであり、署名意味検証、live provider/auth取得、
current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、EC-M3-01〜05 と
M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay lockが現HEADに一致せず、
別セッション所有のLima/QEMU/replaydも稼働中のため Linux replay・stage regeneration・full buildは未実行である。Evidence:
[`decisions-v0.3-native-mcp-review-attestation-report.md`](docs/adr/decisions-v0.3-native-mcp-review-attestation-report.md)。

2026-08-02 に explicit verification receipt と native MCP source-owned `review_attestations[]` の binding parity を追加した。
receipt がある report で同じ `review_id` の attestation projection が欠落・曖昧、`verified` でない、provider/key/algorithm が不一致、または
canonical bytes の SHA-256 が receipt の `attestation_digest` と一致しない場合を、MCP downstream projection 前に
`native validate receipt attestation ...` で fail-closed にする RED→GREENを固定した。これは Rust が明示 trust store で検証済みとした
receipt fact の identity/material handoffであり、native shim に暗号署名検証や provider/auth取得を追加するものではない。
native MCP 95 tests、Rust receipt 4 tests、Rust source-attestation focused test、source-file evidence、official two-target fake gateを確認した。
current-source manifest/expected replay lockは現HEADに一致せず、別セッション所有のLima/QEMU/replaydも稼働中のため Linux replay・stage regeneration・full buildは未実行である。
live provider/auth取得・署名意味検証、current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、
EC-M3-01〜05 と M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。Evidence:
[`decisions-v0.3-native-mcp-review-attestation-receipt-binding.md`](docs/adr/decisions-v0.3-native-mcp-review-attestation-receipt-binding.md)。

2026-08-02 に provider snapshot semantic fail-closed boundaryを native MCP source-attestation projectionまで拡張した。
trust-store/review-lifecycle snapshotを指定した場合、native verifierが意味検証していないため、`review_attestations[]` の
`verified` / `stale` / `revoked` stateも receiptで明示的に束ねられた対象以外は
`provider semantic verification is unavailable` で拒否する RED→GREENを固定した。既存の snapshot regular-file/nonempty/digest、
receipt projection、Rust semantic verifierを再実装せず、native側の暗黙 trustを防ぐ postflight boundaryだけを閉じた。
native MCP 96 tests、native receipt 3 tests、Rust receipt 4 tests、Rust source-attestation focused test、source-file evidence、official two-target fake gateを確認した。
current-source manifest/expected replay lockは現HEADに一致せず、別セッション所有のLima/QEMU/replaydも稼働中のため Linux replay・stage regeneration・full buildは未実行である。
native cryptographic/live provider/auth semantic verification、current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、
EC-M3-01〜05 と M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。Evidence:
[`decisions-v0.3-native-mcp-provider-attestation-semantic-boundary.md`](docs/adr/decisions-v0.3-native-mcp-provider-attestation-semantic-boundary.md)。

2026-08-02 に explicit verification receipt と provider snapshot context の coherency boundaryを追加した。receiptと
trust-store/review-lifecycle snapshotを同時に渡す場合、receiptの `trust_store_digest` が今回の raw trust-store bytesから計算した digest、
または明示された provider digestと一致しなければ native起動前に `receipt trust-store digest mismatch` で fail-closed にする RED→GREENを固定した。
これは receipt→attestation projection、snapshot regular-file/nonempty/digest、provider semantic state拒否とは別の context bindingである。
receipt schemaに lifecycle digestはないため、lifecycle意味検証・live provider/auth取得・native暗号検証は追加せず、Rust receiptのtrust-store identityだけを照合する。
native MCP 97 tests、native receipt 3 tests、Rust receipt 4 tests、Rust source-attestation focused test、source-file evidence、official two-target fake gateを確認した。
current-source manifest/expected replay lockは現HEADに一致せず、別セッション所有のLima/QEMU/replaydも稼働中のため Linux replay・stage regeneration・full buildは未実行である。
native cryptographic/live provider/auth semantic verification、current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、
EC-M3-01〜05 と M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。Evidence:
[`decisions-v0.3-native-mcp-review-receipt-provider-context.md`](docs/adr/decisions-v0.3-native-mcp-review-receipt-provider-context.md)。

2026-08-02 に receipt schemaへ lifecycle digestを追加せず、receiptと lifecycle snapshot/digest の組み合わせを native MCPが
暗黙の lifecycle semantic verification として扱わない fail-closed boundaryを追加した。receiptが指定され、`review_lifecycle` pathまたは
`review_lifecycle_digest`が同時に渡された場合は、canonical receiptがsignature/trust-store factだけを持ち lifecycle identityを持たないため、
native起動前に `native MCP receipt cannot establish lifecycle semantic binding without lifecycle-bound receipt` で拒否する。
explicit trust-store digestだけの receipt pathは従来どおり、Rust verified signature handoffとして受理する。これは receipt/provider trust-store coherency、
receipt→attestation projection、provider semantic state拒否とは別の lifecycle binding contractである。native MCP 98 tests、native receipt 3 tests、
Rust receipt 4 tests、Rust source-attestation focused test、source-file evidence、official two-target fake gateを確認した。
current-source manifest/expected replay lockは現HEADに一致せず、別セッション所有のLima/QEMU/replaydも稼働中のため Linux replay・stage regeneration・full buildは未実行である。
native cryptographic verification、live provider/auth acquisition、current-source Linux runtime、Mac/Linux両 target packaged provenance/rollback bytes parityは未検証のため、
EC-M3-01〜05 と M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。Evidence:
[`decisions-v0.3-native-mcp-review-receipt-lifecycle-boundary.md`](docs/adr/decisions-v0.3-native-mcp-review-receipt-lifecycle-boundary.md)。

2026-08-02 に explicit `review_now` context と `ReviewVerificationReceipt.verification_now` の coherency boundaryを追加した。
receiptが指定され、caller identityが明示される場合は、Rust外部 verifierが作った verified factの検証時刻と現在の review contextが
完全一致しなければ native起動前に `native MCP receipt verification clock mismatch with review context` で拒否する。
一致するreceipt＋identityは従来どおり受理する。receipt schemaへ lifecycle digestを追加せず、lifecycle semantic verification、trust-store digest coherency、
receipt projectionとは別の external verification context contractである。native MCP 99 tests、native receipt 3 tests、Rust receipt 4 tests、
Rust source-attestation focused test、release identity 33 tests、source-file evidence、official two-target fake gateを確認した。
current-source manifest/expected replay lockは現HEADに一致せず、別セッション所有のLima/QEMU/replaydも稼働中のため Linux replay・stage regeneration・full buildは未実行である。
native cryptographic verification、live provider/auth acquisition、current-source Linux runtime、Mac/Linux両 target packaged provenance/rollback bytes parityは未検証のため、
EC-M3-01〜05 と M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。Evidence:
[`decisions-v0.3-native-mcp-review-receipt-verification-clock.md`](docs/adr/decisions-v0.3-native-mcp-review-receipt-verification-clock.md)。

2026-08-02 に native MCP `lsharp_validate` の live provider/auth acquisition external boundaryを明示した。`provider_url`、
`provider_api_url`、provider/auth token、`auth_context` のような network/auth入力は native実行前に
`live provider/auth acquisition is an external boundary; use explicit offline snapshots` で拒否し、provider snapshotの regular-file/nonempty/digest
入力だけを受理する。これは receipt verification clock、trust-store coherency、provider semantic verificationとは別の acquisition boundaryであり、
network helperや暗号検証をnativeへ追加しない。native MCP 100 tests、native receipt 3 tests、Rust receipt 4 tests、Rust source-attestation focused test、
release identity 33 tests、source-file evidence、official two-target fake gateを確認した。current-source manifest/expected replay lockは現HEADに一致せず、
別セッション所有のLima/QEMU/replaydも稼働中のため Linux replay・stage regeneration・full buildは未実行である。
native cryptographic verification、live provider/auth実取得、current-source Linux runtime、Mac/Linux両 target packaged provenance/rollback bytes parityは未検証のため、
EC-M3-01〜05 と M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。Evidence:
[`decisions-v0.3-native-mcp-live-provider-auth-boundary.md`](docs/adr/decisions-v0.3-native-mcp-live-provider-auth-boundary.md)。

2026-08-02 に packaged stage0 release archive の offline round-trip provenance binding を追加した。生成前の
stage0 packageを検証した後も、公開直前に archive を task-owned temporary directoryへ展開し、regular/symlink-free
package、target/source commit、evidence identity、payload checksums、生成前 treeとの完全一致を再検証する。
fake `tar` が archive manifest の source commitを書き換える RED を従来実装が公開してしまうことを確認し、
`archive round-trip provenance validation failed` で拒否し invalid output archiveも残さない GREEN を
`bash scripts/ci/test-native-stage0-release-package.sh` で固定した。これは既存 archive input、manifest、
anchor/checksum、identity、atomic installの再実装ではない、packaged provenanceの verified partial sliceである。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydも稼働中のため
Linux replay・stage regeneration・full buildは未実行である。live provider/auth実取得・native cryptographic verification、
current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、
M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。Evidence:
[`decisions-v0.3-stage0-archive-round-trip-provenance.md`](docs/adr/decisions-v0.3-stage0-archive-round-trip-provenance.md)。

2026-08-02 に rollback compatibility archive の offline payload checksum closure を追加した。従来の required
checksum entry確認に加えて、展開後の全 regular file（`checksums.txt` 自身を除く）が安全・一意な checksum entryを
持つこと、checksum entryの targetが実在する regular fileであることを branch-specific manifest preflight後に検証する。
`unlisted-payload` を rollback archiveへ追加した RED を従来実装が受け入れることを確認し、
`checksums.txt missing payload coverage: unlisted-payload` で executable/rollback smoke前に拒否する GREEN を
`bash scripts/ci/test-release-smoke-provider-snapshots.sh` で固定した。これは stage0 archive round-trip、rollback
anchor/version/checksumの個別契約、provider/auth、native cryptoの再実装ではない、rollback payload provenanceの verified partial sliceである。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydも稼働中のため
Linux replay・stage regeneration・full buildは未実行である。current-source Linux runtime、live provider/auth実取得・意味検証、
Mac/Linux両 target packaged provenance/rollback bytes parityは未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。Evidence:
[`decisions-v0.3-rollback-archive-payload-closure.md`](docs/adr/decisions-v0.3-rollback-archive-payload-closure.md)。

2026-08-02 に source-file smoke evidence writerへ fetched stage0 directoryを明示的に渡し、regular/symlink-free
package内の全 regular payloadを相対path・size・file SHA-256の deterministic digestへ束ねる
`stage0_payload_sha256` projectionを追加した。独立再計算を含む offline RED→GREEN harnessで確認した verified partial sliceであり、
M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。live provider/auth取得・意味検証、native crypto、
current-source Linux runtime、Mac/Linux packaged/rollback bytes parityは未検証である。current-source manifest/expected replay lockは
現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため Linux replay・stage regeneration・full buildは未実行。
Evidence: [`decisions-v0.3-source-smoke-fetched-payload-binding.md`](docs/adr/decisions-v0.3-source-smoke-fetched-payload-binding.md)。

2026-08-02 に official two-target orchestrator の target source-smoke postflightへ、fetched
`${SMOKE_ROOT}/stage0-${target}` と evidence manifestの `stage0_payload_sha256` exact cross-checkを追加した。
fake Mac/Linux matching projectionと、Linux digest改変を `stage0_payload_sha256 mismatch` で拒否する RED→GREENを確認した
verified partial sliceである。M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残し、live provider/auth、native crypto、
current-source Linux runtime、Mac/Linux packaged/rollback bytes parityは未検証である。current-source manifest/expected replay lockは
現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため Linux replay・stage regeneration・full buildは未実行。
Evidence: [`decisions-v0.3-native-official-source-smoke-payload-binding.md`](docs/adr/decisions-v0.3-native-official-source-smoke-payload-binding.md)。

2026-08-02 に official two-target source-smoke postflightへ fetched stage0 manifest/source identity bindingを追加した。
target、current `SOURCE_COMMIT`、fetched `manifest.json` の SHA-256、`stage0_payload_sha256` を target evidenceへ exact compareし、
payloadまたはmanifest digest改変を `source smoke evidence stage0 identity mismatch` で拒否する fake RED→GREENを確認した verified
partial sliceである。M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残し、live provider/auth、native crypto、
current-source Linux runtime、Mac/Linux packaged/rollback bytes parityは未検証である。current-source manifest/expected replay lockは
現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため Linux replay・stage regeneration・full buildは未実行。
Evidence: [`decisions-v0.3-native-official-stage0-identity-binding.md`](docs/adr/decisions-v0.3-native-official-stage0-identity-binding.md)。

2026-08-02 に offline path-package installationの Rust/native destination
boundary parityを追加した。既存の `.lsharp/packages/<name>-<source-hash>` が通常 file/directoryなら、Rust `cmd_install`
も native installer と同じ `refusing to replace non-symlink path package` で拒否し、sentinelを保持する。
既存 symlinkの更新は task-owned temporary symlinkを `rename` で確定し、失敗時に一時物だけを回収する。
同一 path dependency fixtureの RED→GREEN を Rust 5 tests と native install 8 testsで確認した。これは MCPに新しい
install routeを追加せず、offline package-installの mutation safetyに限定した verified partial sliceである。
Git/registry取得は外部/provider boundary、native MCP package projection、current-source Linux runtime、Mac/Linux両 targetの
packaged/rollback parityは未検証のため、EC-M3-05 / M3-05-N9 は `[~]` のまま残す。Evidence:
[`decisions-v0.3-native-package-install-destination-boundary.md`](docs/adr/decisions-v0.3-native-package-install-destination-boundary.md)。

2026-08-02 に Git dependency installの Rust/native managed destination parityを追加した。同一 local git fixtureで、既存の
regular file、manifestなし directory、valid/dangling symlinkを installed package として再利用せず fail-closed にし、fresh cloneは
`.tmp-*` へ作成して `lsharp.toml`確認後に promoteする契約を揃えた。clone/manifest/promotion failureでは temporary pathだけを回収し、
lock.toml と module-index を更新しない。Rust 4 tests と native Git 3 testsを RED→GREEN で確認した。registry/cache parity、
MCP package-install API、複数依存のtransactionality、live provider/auth取得、current-source Linux runtime、Mac/Linux両 targetの
packaged/rollback parityは未検証のため、EC-M3-05 / M3-05-N9 は `[~]` のまま残す。Evidence:
[`decisions-v0.3-native-package-install-git-boundary.md`](docs/adr/decisions-v0.3-native-package-install-git-boundary.md)。

2026-08-02 に cached-version dependency の Rust/native semver lexical parityを追加した。同じ offline cache fixtureで
`math-core = "+1.0.0"` を渡したとき、native installerが既に拒否していた signed componentを Rust `cmd_install`も
ASCII digit-only preflightで拒否し、lock entryを生成しない境界を RED→GREEN で揃えた。これは registry/networkを導入しない
cached-version入力境界の verified partial sliceである。複数依存の atomic promotion/rollback transactionality、registry/provider/auth取得、
native MCP package-install semantics、current-source Linux runtime、Mac/Linux packaged/rollback parityは未検証であり、
EC-M3-05 / M3-05-N9 は `[~]` のまま残す。Evidence:
[`decisions-v0.3-native-package-install-cached-semver-parity.md`](docs/adr/decisions-v0.3-native-package-install-cached-semver-parity.md)。
次の RED は path/Git/cached-versionを含む複数依存で、後続依存の失敗時に先行 final package、lock.toml、module-indexを残さない
task-owned transaction boundaryを同一 Rust/native fixtureで固定すること。current-source manifest/expected replay lockが現HEADに一致せず、
別セッション所有のLima/QEMU/replaydも稼働中のため Linux replay・stage regeneration・full buildは未実行である。
再現 command: `cargo test -p lsharp-driver test_cmd_install_version_dependency_rejects_signed_semver_requirement -- --nocapture` と
`python3 scripts/ci/test-native-selfhost-install.py -k signed_cached_version_requirement`。

2026-08-02 に mixed path + cached-version dependencyの Rust/native dependency-resolution transaction boundaryを追加した。
Rustはnativeと同じ名前順で依存を解決し、pathとfresh Gitは `.install-txn-*` へ staging、cached-versionは既存 cacheだけを参照する。
全依存の解決が成功した後だけ final package promotion、lock.toml生成、module-index再構築を行い、後続 cached missでは
先行 path destination、lock/index、staging residueを残さない RED→GREENを同一 fixtureで確認した。既存 valid installationは
失敗した resolution phaseで置換しない。これは dependency-resolution failure boundaryの verified partial sliceであり、final rename・lock write・
module-index I/O failure時の完全 rollback、registry/provider/auth取得、native MCP package-install semantics、current-source Linux runtime、
Mac/Linux packaged/rollback parityは未検証のため、EC-M3-05 / M3-05-N9 は `[~]` のまま残す。Evidence:
[`decisions-v0.3-native-package-install-mixed-transaction.md`](docs/adr/decisions-v0.3-native-package-install-mixed-transaction.md)。
次の RED は final promotionまたは lock/index I/O failure時にも先行 promotionを残さない rollback/atomic commit boundaryである。

2026-08-02 に final promotion loopの Rust/native rollback boundaryを追加した。同じ local fixtureで既存 path package symlinkと fresh Git packageを
名前順に stagingし、promotion index `1` の test-only failpointを注入した。Rustは `cfg(test)` の atomic failpoint、nativeは明示的な
`LSHARP_TEST_INSTALL_FAILPOINT=promotion:1` を使い、先行 promotionを逆順に除去して旧 destinationを復元する。sentinel lock.tomlと
module-index、fresh Git destination、`.install-txn-*` residueが保持/不在になる RED→GREENを確認した。これは final rename loopの
promotion rollback verified partial sliceであり、lockfile/module-index I/O failure時の完全 rollback、registry/provider/auth取得、native MCP package-install
semantics、current-source Linux runtime、Mac/Linux packaged/rollback parityは未検証のため、EC-M3-05 / M3-05-N9 は `[~]` のまま残す。Evidence:
[`decisions-v0.3-native-package-install-promotion-rollback.md`](docs/adr/decisions-v0.3-native-package-install-promotion-rollback.md)。
次の RED は lock.toml または module-index の I/O failureでも package destinationと旧 metadataを復元する commit boundaryである。

2026-08-02 に lock.toml/module-index metadata commitの Rust/native rollback boundaryを追加した。同じ local path + fresh Git fixtureで
promotion後に `lock` と `index` の test-only failpointをそれぞれ注入し、既存 path destination symlink、sentinel lock/indexを復元し、
fresh Git destination、partial metadata、`.install-txn-*` residueを残さない RED→GREENを確認した。Rustは `cfg(test)` failpoint、nativeは
`LSHARP_TEST_INSTALL_FAILPOINT=lock|index` を使い、通常の CLI/API へ公開しない。これは final promotion + metadata rollbackの verified partial sliceであり、
完全な installer transactionality、filesystem durability、registry/provider/auth取得、native MCP package-install semantics、current-source Linux runtime、
Mac/Linux packaged/rollback parityは未検証のため、EC-M3-05 / M3-05-N9 は `[~]` のまま残す。Evidence:
[`decisions-v0.3-native-package-install-promotion-rollback.md`](docs/adr/decisions-v0.3-native-package-install-promotion-rollback.md)。
次の RED は metadata rollback後の filesystem durability/fsync境界、または外部 registry/provider取得を含む完全 transactionalityである。

2026-08-02 に package installer の filesystem durability orderingを Rust/nativeで揃えた。既存の Rust durable-file helperを lock.tomlへ接続し、
module-indexは temporary directory内の file sync→directory sync→rename→`.lsharp` parent sync、package promotionは staged path sync→rename→
final path/parent syncの順序を固定した。`promotion-before-sync`、`promotion-after-sync`、`lock-sync`、`index-sync` の test-only failpointを
同じ local path + fresh Git fixtureへ注入し、既存 package symlink、sentinel lock/indexの復元、fresh Git destinationと `.install-txn-*` residueの
不在を Rust/native RED→GREENで確認した。これは offline ordering/fail-closed verified partial sliceであり、crash consistency、filesystem journaling、
power-loss durability、cross-device rename、完全な installer transactionality、registry/provider/auth取得、current-source Linux runtime、
Mac/Linux packaged/rollback parityは未検証のため、EC-M3-05 / M3-05-N9 は `[~]` のまま残す。Evidence:
[`decisions-v0.3-native-package-install-promotion-rollback.md`](docs/adr/decisions-v0.3-native-package-install-promotion-rollback.md)。
次の RED は実 filesystem crash/power-loss durability または外部 registry/provider取得を含む完全 transactionalityである。

2026-08-02 に cached registry candidate の provenance/determinism boundaryを Rust/nativeで揃えた。version cacheの matching candidateは regular directoryで、
配下を含め symlink-free、regularでparse可能な `lsharp.toml`、dependency name一致、valid semverであることを事前検証し、matching範囲に
invalid candidateが一つでもあれば暗黙に捨てず fail-closed にする。同じ semantic versionの valid候補は、既存の最高version選択に加えて
cache directory nameの辞書順最大を tie-breakerとして Rust/nativeで一致させた。root/nested symlink、malformed manifest、同version候補を同一
offline fixtureで RED→GREENし、invalid時の既存 lock/indexと `.install-txn-*` residue不在を確認した。これは cached candidate
provenance/determinismの verified partial sliceであり、registry/network取得、filesystem crash consistency、完全な installer transactionality、
current-source Linux runtime、Mac/Linux packaged/rollback parityは未検証のため、EC-M3-05 / M3-05-N9 は `[~]` のまま残す。Evidence:
[`decisions-v0.3-native-package-install-promotion-rollback.md`](docs/adr/decisions-v0.3-native-package-install-promotion-rollback.md)。
次の RED は外部 registry/provider取得を含む完全 transactionality、または実 filesystem crash/power-loss durabilityである。

2026-08-02 に宣言済み local `path` dependency の Rust/native input validation parityを追加した。missing path、通常 file、
`lsharp.toml` 欠落を Rust `cmd_install` と native selfhost installer の同一 fixtureで fail-closed にし、managed `.lsharp` directory、
lock.toml、module-index、transaction stagingを作成しない境界を固定した。これは cached registry candidate provenanceや既存の
destination/promotion/metadata/durability rollbackとは別の local provider input boundaryであり、registry/network取得は追加していない。
Rust installer 22 tests、native installer 18 tests、Rust format、Python syntaxを通過した。live registry/provider取得、crash/power-loss semantics、
完全 transactionality、native MCP package-install semantics、current-source Linux runtime、Mac/Linux packaged/rollback parityは未検証のため、
EC-M3-05 / M3-05-N9 は `[~]` のまま残す。Evidence: [`decisions-v0.3-native-package-install-path-input-fail-closed.md`](docs/adr/decisions-v0.3-native-package-install-path-input-fail-closed.md)。
current-source manifest/expected replay lockは現HEADに一致せず、別セッション所有のLima/QEMU/replaydも稼働中のため、Linux replay・stage regeneration・full buildは未実行である。

2026-08-02 に native component compile/build の semantic validation boundaryを追加した。byte-shape検査を通過した
temporary componentを atomic replace前に明示 `wasm-tools validate` へ渡し、semantic invalid時は validatorの exit/stderrを保持して
既存 outputを変更せず、temporary componentを cleanupする。同一 fake fixtureで `component new` → `validate` の順序、既存 failure、
invalid bytes、stderr/exit、explicit tool、atomic replaceを focused GREENで確認した。これは外部 validatorによる semantic validationの
verified partialであり、component instantiation、source/ftable/import parity、standalone runtime、provider/auth、current-source Linux runtime、
Mac/Linux packaged/rollback parityの証拠ではないため、EC-M3-04 / EC-M3-05 と M3-04-N1 / M3-05-N9 は `[~]` のまま残す。Evidence:
[`decisions-v0.3-native-component-semantic-validation.md`](docs/adr/decisions-v0.3-native-component-semantic-validation.md)。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、Linux replay・stage regeneration・full buildは未実行である。


2026-08-02 の verified partial: native MCP `lsharp_validate` の provider digest-only
contextを explicit snapshot ownershipへ閉じた。`review_trust_store_digest` または
`review_lifecycle_digest` だけを渡す入力は、対応する `trust_store` と `review_lifecycle`
の regular non-symlink snapshot pathも、明示 verification receiptもないため native実行前に
`provider digest requires explicit provider snapshot files` で fail-closed にする。receiptに
束ねた既存の trust-store digest contextは受理し、receiptと一致しない場合は従来どおり拒否する。
同一 fake harnessで digest-only RED→GREEN、explicit snapshotの bytes→digest forwarding、
receipt lifecycle boundaryを確認した。
これは既存の live provider/auth external boundary、trust-store/receipt coherency、native
cryptographic verificationとは重複せず、network/auth clientや暗号検証は追加していない。
current-source Mac/Linux runtime、full Rust/native producer parity、packaged/rollback parity、
live provider/auth acquisitionは未検証のため、EC-M3-01〜05 と M3-04-N1 / M3-05-N2 /
M3-05-N7 / M3-05-N9 は `[~]` のまま維持する。Evidence:
[`decisions-v0.3-native-mcp-provider-digest-requires-snapshot.md`](docs/adr/decisions-v0.3-native-mcp-provider-digest-requires-snapshot.md)。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが
稼働中のため、Linux replay・stage regeneration・full buildは未実行である。

2026-08-02 の verified partial: semantic fixture diff の Rust-oracle/native-stage0
producer observation state parityを追加した。valid fixtureの artifact/runtime が片側だけ
`observed` で他方が `pending` の場合、比較前に `artifact.status` / `runtime.status` の
observable mismatchとして fail-closed にし、同じ `pending` は従来どおり pending、両側が
`observed` の場合は既存の exact payload comparisonを継続する。同じ fake fixtureで
asymmetric stateの RED→GREEN、all-pending、both-observed、既存 mismatchを
`PYTHONDONTWRITEBYTECODE=1 python3 -m unittest -v scripts/ci/test-semantic-fixture-diff.py`
で確認した。report schemaは変更していない。これは full native producer parity、current-source
Mac/Linux runtime、packaged/rollback parity、provider/authの完了証拠ではないため、EC-M3-04 /
EC-M3-05 と M3-04-N1 / M3-05-N9 は `[~]` のまま維持する。Evidence:
[`decisions-v0.3-semantic-observation-state-parity.md`](docs/adr/decisions-v0.3-semantic-observation-state-parity.md)。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが
稼働中のため、Linux replay・stage regeneration・full buildは未実行である。

2026-08-02 の verified partial: native semantic report producerの stage0 manifest→compiler runner identity bindingを追加した。
従来は manifestの kind/target/source_commit と compiler/transport/materializerの relative path shapeだけを検証し、別の `--runner`を実行できた。
manifestの `compiler` を regular executableとして検査し、resolved pathを明示 `--runner` と exact compareすることで、unbound/missing/symlink/non-executable
compilerを fixture実行前に fail-closed にした。同一 fake fixtureで unbound runnerの RED→GREEN、native producer 20 tests、Rust/native diff 9 testsを確認した。
これは source_sha256、ABI、runtime output/exit、batch transaction/cleanup の再実装ではなく、native evidenceが宣言 stage0 payloadを観測するための
manifest-to-runner identity boundaryである。current-source Mac/Linux runtime、full Rust/native producer parity、packaged/rollback parity、provider/authは未検証のため、
EC-M3-04 / EC-M3-05 と M3-04-N1 / M3-05-N9 は `[~]` のまま維持する。Evidence: [`decisions-v0.3-native-report-manifest-runner-binding.md`](docs/adr/decisions-v0.3-native-report-manifest-runner-binding.md)。
再現 command: `python3 scripts/ci/test-semantic-fixture-native-report.py SemanticFixtureNativeReportTest.test_rejects_runner_not_bound_to_stage0_manifest` と
`python3 scripts/ci/test-semantic-fixture-diff.py`。current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、
Linux replay・stage regeneration・full buildは未実行である。

2026-08-02 の verified partial: semantic fixture report producer の stdout/stderr admission を Rust/native で揃えた。
valid fixture の `expected.runtime.stdout` / `stderr` と Wasmtimeの実出力が不一致なら、exit `0` でも report生成前に stream-specific fail-closed とする
同一 fake fixtureを RED→GREEN で確認した。runtime exit admission、receipt、artifact digest、source/ftable/import、batch cleanupの再実装ではない。
実 target runtime、current-source Mac/Linux producer parity、packaged/rollback parity、provider/authは未検証のため EC-M3-04 / EC-M3-05 と
M3-04-N1 / M3-05-N9 は `[~]` のまま残す。Evidence:
[`decisions-v0.3-semantic-runtime-output-admission.md`](docs/adr/decisions-v0.3-semantic-runtime-output-admission.md)。current-source manifest/expected replay lockが現HEADに一致せず、
別セッション所有のLima/QEMU/replaydが稼働中のため Linux replay・stage regeneration・full buildは未実行である。

2026-08-02 の verified partial: semantic fixture report producer の runtime exit admission を Rust/native で揃えた。
valid fixture の `expected.runtime.exit_code` と Wasmtime の exit code が不一致なら report生成前に fail-closed とする同一 fake fixtureを RED→GREEN で確認した。
これは runtime-evidence receipt、artifact digest binding、batch cleanup、component rollback、source/ftable/import projectionの再実装ではなく、runtime observation
の exit admission に限定する。stdout/stderr semantic equality、実 component/target runtime、current-source Mac/Linux producer parity、packaged/rollback parity、
provider/authは未検証のため EC-M3-04 / EC-M3-05 と M3-04-N1 / M3-05-N9 は `[~]` のまま残す。Evidence:
[`decisions-v0.3-semantic-runtime-exit-admission.md`](docs/adr/decisions-v0.3-semantic-runtime-exit-admission.md)。current-source manifest/expected replay lockが現HEADに一致せず、
別セッション所有のLima/QEMU/replaydが稼働中のため Linux replay・stage regeneration・full buildは未実行である。

2026-08-02 の verified partial: semantic fixture report producerの複数 fixture batchをRust oracle/native stage0で
all-or-nothing staging boundaryへ揃えた。後段 fixtureのcompile/runtime failureではreportを書かず、producerが作成した
per-fixture work/runtime directoryだけを逆順cleanupし、caller-owned sentinelと既存rootは保持する同一fake fixtureの
RED→GREENを確認した。これは直前のsource_sha256、artifact/runtime digest、component rollbackとは別のproducer batch
transaction boundaryである。single-fixture root semantics、current-source Mac/Linux producer/runtime、component instantiation、
packaged/rollback parity、provider/auth、crash/power-loss durabilityは未検証のため、EC-M3-04 / EC-M3-05 と M3-04-N1 / M3-05-N9 は
`[~]` のまま維持する。Evidence: [`decisions-v0.3-semantic-report-batch-transaction.md`](docs/adr/decisions-v0.3-semantic-report-batch-transaction.md)。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、Linux replay・stage regeneration・full buildは未実行である。

2026-08-02 の verified partial: native component compile/build の explicit runtime evidence と packaged outputを一つの
bounded local commitへ接続した。runtime evidenceは sibling temporary sidecarへ先に書き、既存 outputを backupした後に componentを
promoteし、最後に sidecarを promoteする。output/evidence の test-only failpointを同一 fake fixtureへ注入し、どちらの失敗でも
既存 outputを復元し、sidecarと component/evidence temporary residueを残さない RED→GREENを確認した。これは既存の component
runtime、artifact digest、source/ftable/import projection の再実装ではなく、runtime→evidence→package の partial-state防止に限定する。
real component instantiation、Rust/native producer parity、current-source Mac/Linux runtime、packaged/rollback parity、provider/auth、
crash/power-loss filesystem semanticsは未検証のため、EC-M3-04 / EC-M3-05 と M3-04-N1 / M3-05-N9 は `[~]` のまま維持する。
Evidence: [`decisions-v0.3-native-component-runtime-evidence-atomic-commit.md`](docs/adr/decisions-v0.3-native-component-runtime-evidence-atomic-commit.md)。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、Linux replay・
stage regeneration・full buildは未実行である。再現 command: `python3 scripts/ci/test-native-selfhost-component.py`。

2026-08-02 に native component compile/build の opt-in runtime postflightを追加した。`--wasmtime PATH` が明示された場合だけ、byte-shape検査と
`wasm-tools validate` の後、atomic replace前に `wasmtime run <temporary-component>` を実行する。成功時の runtime invocationと cleanup、runtime exit
`31` 時の stderr/exit、既存 output保持、temporary cleanupを同一 fake fixtureで RED→GREENにした。既定 compile/buildは Wasmtimeを起動しない。
これは component instantiationの外部 boundaryの verified partialであり、実 target runtime、source/ftable/import parity、provider/auth、Mac/Linux
packaged/rollback parityの証拠ではないため、EC-M3-04 / EC-M3-05 と M3-04-N1 / M3-05-N9 は `[~]` のまま残す。Evidence:
[`decisions-v0.3-native-component-runtime-postflight.md`](docs/adr/decisions-v0.3-native-component-runtime-postflight.md)。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、Linux replay・stage regeneration・full buildは未実行である。

2026-08-02 に semantic fixture の Rust-oracle/native-stage0 reportへ `runtime.artifact_sha256` を追加し、Wasmtimeで実行した artifact digestと
reportの `artifact.sha256` が一致する source→artifact→runtime bindingを固定した。observed runtimeが observed artifactを欠く場合、または digestが
不一致の場合は diff/evidence auditが fail-closed になり、pending/not-run は digest nullのまま残る。同一 fake fixtureで Rust/native report、diffの
negative mismatch、target evidence audit、two-target aggregateを RED→GREENにした。これは packaged runtime evidenceの identity boundaryの
verified partialであり、source/ftable/import producer parity、実 target runtime、current-source stage0、provider/auth、Mac/Linux packaged/rollback
bytes parityの証拠ではないため、EC-M3-04 / EC-M3-05 と M3-04-N1 / M3-05-N9 は `[~]` のまま残す。Evidence:
[`decisions-v0.3-semantic-runtime-artifact-binding.md`](docs/adr/decisions-v0.3-semantic-runtime-artifact-binding.md)。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、Linux replay・stage regeneration・full buildは未実行である。

2026-08-02 に native component compile/build の artifact postflightを追加した。native programの core outputと
wasm-tools component new の packaged outputを、atomic promotion前に non-symlink regular file・non-empty Wasm magic
('\0asm')として検査する。同じ fake component fixtureで、invalid coreは wasm-tools 実行前に拒否し、invalid packaged
bytesは既存 outputを置換せず fail-closedにする RED→GREENを確認した。既存の child failure、stderr/exit forwarding、
temporary cleanup、explicit tool、atomic replace casesも維持する。これは native byte-shape postflightの verified
partialであり、Wasm semantic validation、source/ftable/import parity、standalone runtime、live provider/auth、
current-source Linux runtime、Mac/Linux packaged/rollback parityは未検証のため、EC-M3-04 / EC-M3-05 と
M3-04-N1 / M3-05-N9 は [~] のまま残す。Evidence:
[`decisions-v0.3-native-component-artifact-postflight.md`](docs/adr/decisions-v0.3-native-component-artifact-postflight.md)。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、
Linux replay・stage regeneration・full buildは未実行である。

2026-08-02 に official two-target source-smoke manifest の cross-target projection parityを追加した。Mac/Linux各 targetの
個別 stage0/source/report検証に加え、target固有の `target` / stage0 manifest digest / payload digest 以外の key set と
JSON値を両manifest間で exact compareし、Linuxだけの追加projectionを gate成功へ昇格させない。同一 fake fixtureで正常な
target-specific digest差を許容し、Linux-only fieldを fail-closed にする RED→GREENを確認した。これは official shell
orchestratorの offline/fake evidence shape boundaryであり、Rust canonical verifierや実 native producer/runtimeの証拠ではない。
current-source Mac/Linux runtime、packaged App.Cli/rollback bytes parity、live provider/auth、Rust/native producer parityは未検証のため、
M3-04-N1 / M3-05-N9 は `[~]` のまま残す。Evidence: [`decisions-v0.3-native-official-cross-target-source-smoke-projection.md`](docs/adr/decisions-v0.3-native-official-cross-target-source-smoke-projection.md)。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydも稼働中のため、Linux replay・stage regeneration・full buildは未実行である。

2026-08-02 の verified partial: semantic fixture report producerの invalid compiler exit admissionをRust/nativeで揃えた。
invalid fixtureは従来から non-zero compiler exit と診断 code/span を要求していたが、fixtureが宣言する `expected.exit_code` と異なる
non-zero exitでも report を生成できたため、同じ `invalid/type-undefined-value` fake fixtureで exit `2` を注入する REDを追加した。
Rust oracle/native stage0とも compile exit mismatch を report生成・Wasmtime実行前に fail-closed とし、既存の diagnostic parser、source digest、
no-artifact、invalid report schemaを維持した。Rust 18 tests、native 19 testsをGREENにした。これは diagnostic span parity、runtime exit/output admission、
runtime-evidence receipt、source/ftable/import、batch cleanupの再実装ではなく、invalid compiler outcomeの exact admissionに限定する。
実 target Mac/Linux runtime、current-source Rust/native producer parity、packaged/rollback parity、provider/authは未検証のため、EC-M3-04 / EC-M3-05 と
M3-04-N1 / M3-05-N9 は `[~]` のまま残す。Evidence: [`decisions-v0.3-semantic-invalid-compile-exit-admission.md`](docs/adr/decisions-v0.3-semantic-invalid-compile-exit-admission.md)。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、Linux replay・stage regeneration・full buildは未実行である。
RED/GREEN再現 command: `python3 scripts/ci/test-semantic-fixture-rust-report.py SemanticFixtureRustReportTest.test_rejects_invalid_fixture_with_unexpected_compile_exit_before_report` と
`python3 scripts/ci/test-semantic-fixture-native-report.py SemanticFixtureNativeReportTest.test_rejects_invalid_fixture_with_unexpected_compile_exit_before_report`。
重い gate再開前の blocker再現は `ps -axo pid=,command= | rg 'lsharp-linux-x86|replayd'` と
`find . -path './target' -prune -o -type f \( -name manifest.json -o -name '*replay*lock*' -o -name 'expected-lock*' \) -print` で行う。

2026-08-02 の verified partial: official two-target release orchestratorの target runtime/evidence admissionを追加した。
App.Cli/stage0 archiveとchecksumを既存の最終 `DIST_DIR` へ直接書かず、task-owned `SMOKE_ROOT/release-dist`へ stagingし、Mac/Linuxの
fetched stage0 runtime postflightと cross-target evidence projection が全て成功した後だけ final `DIST_DIR` へ promoteする。同一 fake harnessで
Linux側の shared-field mismatch を late failureとして注入し、既存 final outputを保持し新規 archiveを公開しない RED→GREENを確認した。これは
source-commit admission、runtime-evidence receipt、component digest binding、diagnostic span parityとは別の target runtime→package publication
boundaryである。最終 per-file promotionの crash consistency、current-source Mac/Linux runtime、packaged/rollback bytes parity、provider/authは未検証のため、
EC-M3-04 / EC-M3-05 と M3-04-N1 / M3-05-N9 は `[~]` のまま維持する。current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有の
Lima/QEMU/replaydも稼働中のため Linux replay・stage regeneration・full buildは未実行である。Evidence:
[`decisions-v0.3-native-official-release-output-admission.md`](docs/adr/decisions-v0.3-native-official-release-output-admission.md)。

2026-08-02 の verified partial: Rust/native semantic report producerの診断 span grammar parityを追加した。
native側が Rust側と同じ compact byte rangeと multiline structured `Span { start, end }`（diagnostic gutter marker付き）を受け付け、既存の
diagnostic codeとbyte offsetから同じ line/column JSONへ正規化する。同一 `LS3001` fixtureでRust/nativeのstructured span、invalid code/span、
reversed/UTF-8 boundaryのfail-closed semanticsを focused testsで確認した。これは source-commit admission、runtime-evidence receipt、component digest binding、
static source/ftable/import projectionとは別の producer diagnostic parityであり、report schemaは変更していない。current-source Mac/Linux producer parity、
component instantiation、target runtime、packaged/rollback parity、provider/authは未検証のため、EC-M3-04 / EC-M3-05 と M3-04-N1 / M3-05-N9 は `[~]` のまま残す。
Evidence: [`decisions-v0.3-semantic-native-diagnostic-span-parity.md`](docs/adr/decisions-v0.3-semantic-native-diagnostic-span-parity.md)。

2026-08-02 の verified partial: `v4-m1-07` static source→artifact projectionに current-source commit admissionを追加した。
projection開始時に explicit `--source-commit` と `root` の現HEADを exact compareし、不一致・取得不能・不正なcommitなら `wasm-tools print`、artifact読込、sidecar生成を行わず fail-closed にする。同一 fake fixtureで current commitのprojection GREENと stale commitの外部tool未起動/no-evidence RED→GREENを確認した。
これは直前の runtime-evidence receipt、component runtime→packaged digest binding、既存のsource/ftable/import shape projectionとは別の producer provenance admission boundaryであり、report schemaは変更していない。current-source Rust/native producer parity、component instantiation、Mac/Linux runtime、packaged/rollback parity、provider/authは未検証のため、EC-M3-04 / EC-M3-05 と M3-04-N1 / M3-05-N9 は `[~]` のまま残す。Evidence:
[`decisions-v0.3-semantic-source-commit-admission.md`](docs/adr/decisions-v0.3-semantic-source-commit-admission.md)。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、Linux replay・stage regeneration・full buildは未実行である。

2026-08-02 の verified partial: explicit component runtimeの実行結果を source/artifactへ結合した。
`--runtime-evidence PATH` を `--wasmtime PATH` と組み合わせた場合だけ、semantic validation後の source digest、temporary component digest、runtime exit/stdout/stderrを
atomic sidecarへ記録し、receipt生成成功後に限り packaged promotionする。fake native→component new→validate→wasmtime fixtureで16 testsがGREENとなり、
output identity、runtime mutation、既存 output保持、temporary cleanupを確認した。これは直前の static source/ftable/import projectionや bytes mutation guardとは別の
component instantiation/runtime evidence boundaryであり、report schemaは追加していない。real component instantiation、Rust/native producer parity、
current-source Mac/Linux runtime、packaged/rollback parity、provider/authは未検証のため、EC-M3-04 / EC-M3-05 と M3-04-N1 / M3-05-N9 は `[~]` のまま残す。
Evidence: [`decisions-v0.3-native-component-runtime-evidence.md`](docs/adr/decisions-v0.3-native-component-runtime-evidence.md)。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、Linux replay・stage regeneration・full buildは未実行である。

2026-08-02 の verified partial: native component の explicit `wasmtime run` と packaged promotionの artifact identityを接続した。
`wasm-tools validate` 後の temporary component digestを runtime前に固定し、runtimeが成功しても bytesを変更していた場合は `os.replace` 前に
fail-closed にする。同一 fake native→component new→validate→wasmtime fixtureで、成功経路、runtime mutation拒否、既存 output保持、temporary
cleanupを確認した。これは直前の static source/ftable/import projectionとは異なる component lifecycle boundaryであり、report schemaは追加していない。
fake runtime evidenceの verified partialに留まり、real component instantiation、Rust/native producer parity、current-source Mac/Linux runtime、packaged/rollback parity、provider/authは未検証のため、
EC-M3-04 / EC-M3-05 と M3-04-N1 / M3-05-N9 は `[~]` のまま残す。Evidence:
[`decisions-v0.3-native-component-runtime-artifact-identity.md`](docs/adr/decisions-v0.3-native-component-runtime-artifact-identity.md)。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、Linux replay・stage regeneration・full buildは未実行である。

2026-08-02 に package installer の stale transaction ownership boundaryを Rust/nativeで揃えた。管理対象 packages directoryに
`.install-txn-*` が残っている場合、unknown ownerの stagingを暗黙に再利用せず、
`install transaction staging already exists; refusing to reuse unknown owner` で promotion前に fail-closed にする。同一 fixtureで
stale owner sentinel、既存 package destination、lock.toml、module-indexを無変更保持し、host fallbackを呼ばない RED→GREENを確認した。
Rust installer 24 tests、native installer 20 testsがGREENである。これは直前の registry external-boundary、promotion/metadata/durability rollbackとは別の
stale transaction ownership/provider state preservationであり、stagingの自動復旧・削除や crash/power-loss semantics は追加していない。
live registry/provider retrieval/auth、完全 transactionality、native MCP package-install semantics、current-source Linux runtime、Mac/Linux packaged/rollback parityは
未検証のため、EC-M3-05 / M3-05-N9 は `[~]` のまま残す。Evidence:
[`decisions-v0.3-native-package-install-stale-transaction-boundary.md`](docs/adr/decisions-v0.3-native-package-install-stale-transaction-boundary.md)。
current-source manifest/expected replay lockは現HEADに一致せず、別セッション所有のLima/QEMU/replaydも稼働中のため、Linux replay・stage regeneration・full buildは未実行である。

2026-08-02 に version dependency の Rust/native registry external boundaryを追加した。version spec は明示された offline
`.lsharp/packages` cacheからのみ解決し、cache missを live registry取得へ暗黙に委譲せず、
`registry provider acquisition is an external boundary` として managed `.lsharp`、lock.toml、module-index、transaction stagingの作成前に
fail-closed にする同一 fixtureを固定した。valid cached candidate、path/Git install、promotion/metadata/sync rollbackを含む Rust installer
23 tests、native installer 19 testsがGREENである。これは直前の local path-provider input boundaryや cached candidate provenanceの再実装ではなく、
registry/network/auth取得を追加しない external boundaryである。live registry/provider retrieval/auth、完全 transactionality、crash/power-loss semantics、
native MCP package-install semantics、current-source Linux runtime、Mac/Linux packaged/rollback parityは未検証のため、EC-M3-05 / M3-05-N9 は `[~]` のまま残す。
Evidence: [`decisions-v0.3-native-package-install-registry-external-boundary.md`](docs/adr/decisions-v0.3-native-package-install-registry-external-boundary.md)。
current-source manifest/expected replay lockは現HEADに一致せず、別セッション所有のLima/QEMU/replaydも稼働中のため、Linux replay・stage regeneration・full buildは未実行である。

2026-08-02 の verified partial: semantic fixture の source→Wasm artifact projection→runtime/evidence bindingを追加した。
`valid/nested-record-pattern` が宣言する `ftable` / `imports` を対象に、明示 `wasm-tools print` の ordered imports、table shape、exportsと
source/artifact digestを別 sidecar (`v4-m1-07`)へ投影し、Rust-oracle/native-stage0 sidecarを exact compareする。既存 reportを渡した場合は
sidecarの artifact digestが report artifact、および observed runtime artifact digestと一致しないと fail-closed になる。同一 fake fixtureで
projection success、helper failure前の no-evidence、Rust/native table mismatch、既存 runtime reportへの digest bindingを RED→GREENで確認した。
これは report schemaを拡張しない offline static artifact/evidence boundaryの verified partialであり、現行 Rust/native producerが実 targetで同じ
source/ftable/import bytesを出す証拠、component instantiation、Mac/Linux current-source runtime、packaged/rollback parity、provider/authの証拠ではない。
EC-M3-04 / EC-M3-05 と M3-04-N1 / M3-05-N9 は `[~]` のまま残す。Evidence:
[`decisions-v0.3-semantic-source-artifact-projection.md`](docs/adr/decisions-v0.3-semantic-source-artifact-projection.md)。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydも稼働中のため、Linux replay・stage regeneration・full buildは未実行である。
2026-08-02 の verified partial: `v4-m1-07` static source→artifact projectionの non-empty-only確認を、fixture-level exact ABI contractへ拡張した。
`semantic-fixture-artifact-expectations.json`で `valid/nested-record-pattern` の ordered imports、static table/ftable shape、exportsを固定し、fake artifactの
shape driftは projection sidecar生成前に fail-closed にする。同一 fake fixtureで既存 projection success、Rust/native sidecar diff、report/runtime digest bindingと
table shape drift/no-evidenceを RED→GREENで確認した。これは report schemaや既存 runtime receiptを変更しない offline/fake verified partialであり、current-source
Rust/native producer parity、component instantiation、Mac/Linux runtime、packaged/rollback parity、provider/authの証拠ではない。
EC-M3-04 / EC-M3-05 と M3-04-N1 / M3-05-N9 は `[~]` のまま維持する。Evidence:
[`decisions-v0.3-semantic-source-artifact-exact-abi.md`](docs/adr/decisions-v0.3-semantic-source-artifact-exact-abi.md)。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、Linux replay・stage regeneration・full buildは未実行である。
2026-08-02 の verified partial: semantic fixture reportへ `source_sha256` を追加し、source bytes→Rust-oracle/native-stage0 producer→current fixtureの exact bindingを固定した。
compile前の digestと観測後の再計算が一致しない source mutation、Rust/native間の digest mismatch、current fixtureと異なる stale digestは report/evidence前に fail-closedにする。
同じ fake fixtureで Rust report 14、native report 15、diff 9、evidence audit 12、two-target aggregate 7をGREENにした。これは直前の source-commit admission、exact ABI binding、
runtime-evidence receiptとは別の compiler-input provenance verified partialであり、current-source Rust/native producer parity、component instantiation、Mac/Linux runtime、
packaged/rollback parity、provider/authの証拠ではない。EC-M3-04 / EC-M3-05 と M3-04-N1 / M3-05-N9 は `[~]` のまま維持する。Evidence:
[`decisions-v0.3-semantic-source-fingerprint-binding.md`](docs/adr/decisions-v0.3-semantic-source-fingerprint-binding.md)。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、Linux replay・stage regeneration・full buildは未実行である。
2026-08-02 の verified partial: semantic fixture report producerの compiler/runner workspace ownershipをRust/nativeで揃えた。両producerは compile subprocessの `cwd` を per-fixtureのtask-owned directoryへ固定し、相対パスの runner 出力が checkout rootへ漏れないことを同一 fake fixtureで RED→GREEN にした。Rust producer 19 tests、native producer 21 testsと既存 focused batchがGREENである。これは source mutation、manifest→runner identity、runtime/report admissionとは別の runner workspace safety contractであり、current-source Mac/Linux runtime、full native producer parity、component instantiation、packaged/rollback parity、provider/authは未検証のため、EC-M3-04 / EC-M3-05 と M3-04-N1 / M3-05-N9 は `[~]` のまま維持する。Evidence: [`decisions-v0.3-semantic-producer-runner-workspace.md`](docs/adr/decisions-v0.3-semantic-producer-runner-workspace.md)。current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、Linux replay・stage regeneration・full buildは未実行である。
2026-08-02 の verified partial: native semantic report producerの stage0 manifest payload closureを追加した。compilerだけでなく、manifestが宣言する
`transport_driver` と `materializer` も、source fixtureのcopy、runner起動、report/evidence生成前に regular executable file として検査する。
欠落または symlink payload の fake fixture 4ケースを native report 22 testsで RED→GREENにし、既存の compiler→runner identity、source digest、
runtime/report admission、batch cleanup semanticsは変更していない。これは runner identity/cwd isolation とは別の stage0 package closed-shape
boundaryである。Rust oracleはstage0 manifestを入力としないためこの preflight の対象外であり、current-source Mac/Linux runtime、full native producer
parity、component instantiation、packaged/rollback parity、provider/authは未検証のため、EC-M3-04 / EC-M3-05 と M3-04-N1 / M3-05-N9 は `[~]` のまま維持する。
Evidence: [`decisions-v0.3-native-report-stage0-payload-closure.md`](docs/adr/decisions-v0.3-native-report-stage0-payload-closure.md)。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、Linux replay・stage regeneration・full buildは未実行である。
2026-08-02 の verified partial: semantic report producerの child environment isolationをRust/nativeで揃えた。compiler、native runner、wasm-tools、Wasmtimeへ渡す環境から
ambientな `LSHARP_*` を全て除去し、Rust oracleだけが明示 `LSHARP_DISABLE_EMBEDDED_COMPONENT=1` を再設定する。`LSHARP_PATH`、provider URL、install
failpointを親環境へ注入する fake fixtureで、Rust childが explicit guardだけ、native childがゼロの `LSHARP_*` になる RED→GREENを確認した。
これは manifest→runner identity、runner cwd isolation、stage0 payload closure、source/ABI/runtime/report admissionとは別の environment isolation
contractであり、live provider/auth取得や暗号意味検証を追加していない。Rust producer 20 tests、native producer 23 testsと fixture/differential/
evidence/aggregate/docs/whitespaceの focused batchがGREENである。current-source Mac/Linux runtime、full native producer parity、packaged/rollback parity、
provider/authは未検証のため、EC-M3-04 / EC-M3-05 と M3-04-N1 / M3-05-N9 は `[~]` のまま維持する。Evidence:
[`decisions-v0.3-semantic-producer-environment-isolation.md`](docs/adr/decisions-v0.3-semantic-producer-environment-isolation.md)。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、Linux replay・stage regeneration・full buildは未実行である。

2026-08-02 の verified partial: semantic fixture two-target aggregateへ target-independent observation parityを接続した。両 target indexの
fixture scopeと各 target内の Rust/native parityを再監査した後、producerごとに `source_sha256`、diagnostics、compiler `exit_code`を Mac/Linux間で
exact compareし、両 targetの runtimeが observed の場合だけ runtime exit/stdout/stderrも比較する。不一致は `cross-target semantic parity mismatch` で
aggregate pass前に fail-closed にする。target-specific artifact bytes/digestと pending runtimeは比較対象から除外し、report/aggregate schemaは変更していない。
同じ fake fixtureで target内は一貫したまま Linux側だけの source digest mismatchを注入する RED→GREENを
`python3 scripts/ci/test-semantic-fixture-evidence-aggregate.py -v` で確認した。これは直前の producer environment isolation、source-commit admission、
official source-smoke projectionとは別の semantic evidence cross-target relationであり、current-source Mac/Linux runtime、full native producer parity、
packaged/rollback parity、provider/authは未検証のため EC-M3-04 / EC-M3-05 と M3-04-N1 / M3-05-N9 は `[~]` のまま維持する。Evidence:
[`decisions-v0.3-semantic-two-target-observation-parity.md`](docs/adr/decisions-v0.3-semantic-two-target-observation-parity.md)。current-source
manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため Linux replay・stage regeneration・full buildは未実行である。

2026-08-02 の verified partial: official two-target release orchestratorの final `DIST_DIR` promotionを bounded transactionへ強化した。staged regular
fileの managed destinationを task-owned backupへ退避してから順に promoteし、test-only failpointまたは promotion failureでは移動済みの新規ファイルを
除去して旧 archive/checksumsを復元する。unrelated sentinelは保持し、symlink/non-regular managed destinationは fail-closed、成功・rollbackとも transaction
residueを残さない。同一 fake two-target harnessで通常 successと1ファイル後の RED→GREEN、既存 managed outputs復元を確認した。これは直前の
staging-before-evidence admissionとは別の final publication transaction boundaryであり、stale managed file removal、power-loss/`fsync` durability、実 Mac/Linux
runtime、packaged/rollback parity、full native producer parity、provider/authは未検証である。EC-M3-04 / EC-M3-05 と M3-04-N1 / M3-05-N9 は `[~]` のまま維持する。
Evidence: [`decisions-v0.3-native-official-release-final-promotion-transaction.md`](docs/adr/decisions-v0.3-native-official-release-final-promotion-transaction.md)。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、Linux replay・stage regeneration・full buildは未実行である。

2026-08-02 の verified partial: 外部暗号検証 handoff の receipt identity gate を追加した。`verify-native-review-verification-receipt.py RECEIPT_JSON --trust-store TRUST_STORE_JSON` は既存 trust-store validator の active key selection を再利用し、receipt の `(provider, key_id, algorithm)` が active identity に一意に一致しない場合、invalid・inactive・別 provider/key を fail-closed にする。matching active identity、identity mismatch、invalid trust-store の offline fixtureを RED→GREEN で確認した。これは native MCP が署名を検証する変更ではなく、明示 receiptを semantic verified として受け渡す前の外部 provider/crypto identity boundaryであり、receipt schema、digest-only snapshot guard、lifecycle binding、live provider/auth は変更していない。current-source Mac/Linux runtime、full native producer parity、packaged/rollback parity、実 Ed25519 verification は未検証のため、EC-M3-01〜05 と M3-04-N1 / M3-05-N9 は `[~]` のまま維持する。Evidence: [`decisions-v0.3-native-review-verification-receipt-trust-identity.md`](docs/adr/decisions-v0.3-native-review-verification-receipt-trust-identity.md)。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、Linux replay・stage regeneration・full buildは未実行である。

2026-08-02 の verified partial: semantic fixture diff の Rust-oracle/native-stage0 producer parityに、producer-independentな canonical observation bytes を接続した。validated reportの `id`、`source_sha256`、`diagnostics`、`exit_code`、`artifact`、`runtime`だけを fixture ID順・compact/sorted-key UTF-8 JSONへ正規化し、両producerの canonical SHA-256 を comparisonへ記録する。producer/target/source commitのmetadataやJSON/fixture emission orderはbytesへ影響せず、観測値が変われば `canonical_observation_bytes` mismatchとして pass前に fail-closed にする。同じ fake fixtureで order/metadata independence、observation mutation、pending/observed、evidence audit、two-target aggregateの再計算をRED→GREENで確認した。これは report input schemaを拡張せず、直前の structured observation state parityとは別の byte-level producer parity verified partialである。current-source Mac/Linux runtime、full native producer parity、component instantiation、packaged/rollback parity、provider/auth、実 Ed25519 verificationは未検証のため、EC-M3-01〜05 と M3-04-N1 / M3-05-N9 は `[~]` のまま維持する。Evidence: [`decisions-v0.3-semantic-producer-canonical-observation-bytes.md`](docs/adr/decisions-v0.3-semantic-producer-canonical-observation-bytes.md)。
focused command: `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest -v scripts/ci/test-semantic-fixture-diff.py scripts/ci/test-semantic-fixture-evidence-audit.py scripts/ci/test-semantic-fixture-evidence-aggregate.py`。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、Linux replay・stage regeneration・full buildは未実行である。再開条件はcurrent HEAD一致のmanifest/expected replay lockとVM ownershipの明示である。

2026-08-02 の verified partial: native MCP `lsharp_project_context` の offline dependency source projectionを Rust/native で closed-world に揃えた。依存宣言は非空の registry version 文字列、または `path` / `git` の一方だけを持つ source tableに限定し、`path` と `git` の同時指定、pathへの `branch` / `tag`、未知属性、空の source/selector を MCP実行前に fail-closed とする。Python fallback TOML readerも未知属性を捨てずに検査へ渡す。同一fixtureの Rust project-context 5 tests と native project-context 3 testsで、valid projection、4つの invalid source cases、fake native program no-executionを確認した。これは installer transaction、registry/provider acquisition、provider digest/receipt、MCP report projectionとは別の read-only input semanticsである。MCP package-install API、live provider/auth取得・意味検証、実Ed25519、current-source Mac/Linux runtime、Mac/Linux packaged/rollback parityは未検証のため、EC-M3-05 / M3-05-N9 は `[~]` のまま維持する。Evidence: [`decisions-v0.3-native-mcp-project-context-dependency-closed-world.md`](docs/adr/decisions-v0.3-native-mcp-project-context-dependency-closed-world.md)。current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、Linux replay・stage regeneration・full buildは未実行である。再開条件はcurrent HEAD一致manifest/expected replay lockとVM ownershipの明示である。
2026-08-02 の verified partial: 同日、Git dependency URL の authority に埋め込まれた credential を `lsharp_project_context` が
MCP 応答へ投影しない境界を Rust/native で揃えた。live Git/provider/auth取得、package install、
current-source Mac/Linux runtime は未接続のため `EC-M3-05` / `M3-05-N9` は `[~]` のまま維持する。
ADR: [`decisions-v0.3-native-mcp-project-context-git-credential-boundary.md`](docs/adr/decisions-v0.3-native-mcp-project-context-git-credential-boundary.md)。
2026-08-02 の verified partial: installed package の既存 `docs/api.json` を regular non-symlink file に限定し、
package directory 外の JSON を symlink 経由で package-owned metadata として投影しない Rust/native 境界を追加した。
package install、registry/provider取得、current-source Mac/Linux runtime は未接続のため `EC-M3-05` /
`M3-05-N9` は `[~]` のまま維持する。ADR:
[`decisions-v0.3-native-mcp-package-api-regular-file-boundary.md`](docs/adr/decisions-v0.3-native-mcp-package-api-regular-file-boundary.md)。

2026-08-02 の verified partial: `.lsharp/packages/<entry>` 自体を regular non-symlink directory に限定した。
同一の外部 package-root symlink fixtureについて、enumeration surfaceの search/project-context は既存 non-directory
entryと同様に無視し、explicit package-api lookupは既存 not-found で fail closed にする Rust/native parityを固定した。
これは `docs/api.json` final entryの regular-file検査と重複しない package discovery ownership boundaryである。
nested package tree、installer、registry/provider取得、current-source Mac/Linux runtime、packaged/rollback parity は
未検証のため `EC-M3-05` / `M3-05-N9` は `[~]` のまま維持する。ADR:
[`decisions-v0.3-native-mcp-installed-package-directory-ownership.md`](docs/adr/decisions-v0.3-native-mcp-installed-package-directory-ownership.md)。

2026-08-02 の verified partial: regular installed package directory内の`lsharp.toml` symlinkをdiscovery対象外とし、外部manifest identityをsearch/project-context/package-apiへ投影しないRust/native境界を追加した。`src/`、個別source、`docs/`を含むnested tree全体、installer、provider、current-source Mac/Linux runtimeは未検証のため、`EC-M3-05` / `M3-05-N9`は`[~]`のまま維持する。ADR: [`decisions-v0.3-native-mcp-package-manifest-symlink-boundary.md`](docs/adr/decisions-v0.3-native-mcp-package-manifest-symlink-boundary.md)。
2026-08-02 の verified partial: native public `compile` / `build` の Rust-only target/option admission を stage0 bootstrap より前へ移動した。`web-wasm`、`native`、`--emit-ir` は stage0 manifest読込、stage directory作成、transport/materializer、native helper、output routingの前に既存診断で fail-closed となる。同じ fake stage0 fixtureで unsupported target/option の RED→GREEN、既存 output sentinel保持、stage/helper no-mutation、supported component/Preview1 routing、host fallback不使用を確認した。これは公開 native runnerの明示拒否境界であり、Rust `EmbeddedCli` の実装変更、current-source Mac/Linux runtime、full native producer parity、live provider/auth、実Ed25519、packaged/rollback bytes parityの証拠ではない。`EC-M3-03` / `EC-M3-04` / `EC-M3-05` と `M3-04-N1` / `M3-05-N9` は `[~]` のまま維持する。Evidence: [`decisions-v0.3-native-public-unsupported-preflight.md`](docs/adr/decisions-v0.3-native-public-unsupported-preflight.md)。focused command: `bash scripts/ci/test-native-selfhost-dev.sh`; shell syntax: `bash -n scripts/native-selfhost-dev.sh scripts/ci/test-native-selfhost-dev.sh`; current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、Linux replay・stage regeneration・full build・実target runtimeは未実行である。再開条件はcurrent HEAD一致manifest/expected replay lockとVM ownershipの明示である。
2026-08-02 の verified partial: Rust/native MCPへ `lsharp_install` の explicit external boundaryを追加した。`name` 必須・`project_dir` 任意の同一 closed input schemaを `tools/list` と `tools/call` に投影し、valid requestでも `native MCP package installation requires an explicit external provider adapter` を返して native program、registry/network、auth provider、installerを呼ばず、`lsharp.toml`、`.lsharp/lock.toml`、`.lsharp/module-index.json` と既存 package bytesを変更しない。同じ sentinel fixtureで native MCP 103 tests、Rust MCP 92 testsの RED→GREEN、Rust/native exact descriptor/error/no-mutation parityを確認した。これは provider adapterを実装した証拠ではなく、MCP package-installの明示 external boundaryである。live provider/auth、完全 package transactionality、実Ed25519、current-source Mac/Linux runtime、packaged/rollback parityは未検証のため `EC-M3-03` / `EC-M3-04` / `EC-M3-05` と `M3-04-N1` / `M3-05-N9` は `[~]` のまま維持する。Evidence: [`decisions-v0.3-native-mcp-package-install-external-boundary.md`](docs/adr/decisions-v0.3-native-mcp-package-install-external-boundary.md)。focused commands: `PYTHONDONTWRITEBYTECODE=1 python3 scripts/ci/test-native-selfhost-mcp.py`; `cargo test -p lsharp-driver mcp_server::tests -- --nocapture`; `rustfmt --edition 2024 --check crates/lsharp-driver/src/mcp_server.rs crates/lsharp-driver/src/mcp_schema.rs crates/lsharp-driver/src/mcp_tests.rs`; current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、Linux replay・stage regeneration・full build・実target runtimeは未実行である。再開条件はcurrent HEAD一致manifest/expected replay lockとVM ownershipの明示である。
2026-08-02 の verified partial: Rust/native `lsharp_validate` の live provider/auth external boundaryを揃えた。`provider_url`、`provider_api_url`、`provider_auth_token`、`provider_token`、`auth_token`、`auth_context` のいずれかを含む入力を、source/manifest解析、snapshot読込、native実行、report生成より前に `live provider/auth acquisition is an external boundary; use explicit offline snapshots` で fail-closed にする。同じ6名のfixtureで Rust MCP 93 tests、native MCP 103 testsをGREENにし、native側の既存no-execution/error parityも再確認した。これは install boundary、provider digest/receipt trust identity、live acquisition実装、実Ed25519検証、current-source Mac/Linux runtime、packaged/rollback parityではない。未検証境界のため `EC-M3-01`〜`EC-M3-05` と `M3-04-N1` / `M3-05-N9` は `[~]` のまま維持する。Evidence: [`decisions-v0.3-native-mcp-provider-auth-external-boundary.md`](docs/adr/decisions-v0.3-native-mcp-provider-auth-external-boundary.md)。focused commands: `PYTHONDONTWRITEBYTECODE=1 python3 scripts/ci/test-native-selfhost-mcp.py`; `CARGO_BUILD_JOBS=2 cargo test -p lsharp-driver mcp_server::tests -- --nocapture`; `rustfmt --edition 2024 --check crates/lsharp-driver/src/mcp_validation.rs crates/lsharp-driver/src/mcp_tests.rs`。current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replaydが稼働中のため、Linux replay・stage regeneration・full build・実target runtimeは未実行である。再開条件はcurrent HEAD一致manifest/expected replay lockとVM ownershipの明示である。
