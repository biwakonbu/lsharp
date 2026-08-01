# ADR: native source-file smoke evidence export

- Date: 2026-07-31
- Status: Accepted (verified partial slice)
- Scope: `M3-05-N9` / `EC-M3-05`

## Context

`native-selfhost-dev-source-file-smoke.sh` は各コマンドの stdout/stderr、生成した Wasm、入力 fixture
を一時 work directory に保存してから、成功・失敗を問わず削除する。これは通常の smoke では安全だが、
N9 が要求する current-source target gate の実 bytes、stdout/stderr、exit code の比較証拠を後から監査できない。
過去の stage0 artifact や summary を証拠として再利用することもできない。

## Decision

`NATIVE_SELFHOST_SOURCE_SMOKE_EVIDENCE_DIR` を指定した場合だけ、source-file smoke の終了時に証跡を
外部 directory へ保存する。

1. 出力先は絶対パスの non-root かつ未使用の leaf directory に限定し、既存証跡を上書きしない。
2. `write-native-source-smoke-evidence.py` が一時 staging directory を作り、work directory 全体、stage0
   manifest、終了コードをコピーしてから atomic rename する。
3. `manifest.json` に target、stage0 の source commit、stage0 manifest digest、exit code、Wasm の size/digest、
   stdout/stderr の相対パスを記録する。
4. stage0 manifest の `source_commit` は小文字の40桁 hexadecimal に限定し、uppercase や別形式は証跡を
   作成する前に fail-closed で拒否する。
5. work directory 内の symlink は保存前に fail-closed で拒否し、既存の evidence directory や外部 path を
   symlink 経由で参照する証跡を作らない。
6. 環境変数が未指定の場合の smoke 出力と cleanup は従来どおりで、証跡を自動で `/tmp` や release directoryへ
   残さない。
7. 証跡の書き込みに失敗した成功 run は失敗へ反転し、元の非 zero exit code は保持する。

## Evidence

- RED: `bash scripts/ci/test-native-selfhost-source-file-smoke-evidence.sh` は writer 未実装で失敗した。
- GREEN: 同じ test が exit code、stdout、Wasm bytes、stage0 manifest、manifest の digest/size metadata と
  既存 directory の上書き拒否を確認して通過した。
- `bash -n scripts/ci/native-selfhost-dev-source-file-smoke.sh scripts/ci/test-native-selfhost-source-file-smoke-evidence.sh`
- `python3 -m py_compile scripts/ci/write-native-source-smoke-evidence.py`
- uppercase `source_commit` を渡した writer は証跡 directory を作成せず拒否し、canonical lowercase
  40 桁入力は従来どおり保存する。
- work directory 内の symlink を渡した writer は evidence directory を作成せず拒否し、staging 側の
  symlink 検査も持つ。
- current `3e1b26901aef8191a47382b06bf87fe62c9fb9ad` の Mac stage0 を fresh evidence directoryへ渡し、
  `aarch64-apple-darwin native selfhost source-file smoke passed` を確認した。保存した
  `manifest.json` は stage0 source commit、stage0 manifest digest、`compile.wasm` / `build.wasm` の
  同一 size/digest、stdout/stderr の相対 path、exit code `0` を記録し、期限付き/期限なし
  `expires_at`、`unverified`、directive span、invalid source の error code `8` を含む fixture outputを保持した。

これは証跡保存の contract evidence であり、実 Mac Apple Silicon / Linux x86_64 の current-source producer、
packaged release、runtime parity を完了した証拠ではない。N9 の実 target gate と provider/rollback/Wasm byte
比較は `TODO.md` の `[~]` に残す。

## Consequences

実 target gate の operator は、fresh directory を指定して run 後に同じ bytes/stdout/stderr/exit を再監査できる。
証跡 directory は task-owned output として検証後に回収し、共有 worktree や別セッションの `/tmp` を削除しない。
