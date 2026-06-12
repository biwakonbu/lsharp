# Documentation Freshness

このページは L# 自身の `.lsharp-doc-status` 運用手順です。`lsharp review` / `lsharp doc-ack` / `lsharp doc-check` を repo 内 docs の鮮度確認にも使い、ドキュメント追跡機構を dogfood します。

## Scope

初期運用では `examples/metadata.ls` の `abs` を代表 fixture として追跡します。これは metadata の `:doc` / `:params` / `:returns` / `:example` / `:invariant` を含み、`lsharp test` と `lsharp doc` の両方に使う最小サンプルです。

`docs/guides/error-reference.md` は DOC-06 / imp-02 の `LS####` 体系導入後に追加します。

## Status File

`.lsharp-doc-status` は repo root に置きます。entry は関数名を key にし、AST hash、doc hash、reviewer、review timestamp、freshness を保持します。

現在の初回 ack:

- entry: `abs`
- source: `examples/metadata.ls`
- reviewer: `docs-maintainers`
- freshness: `Fresh`

## Local Commands

```bash
lsharp review examples/metadata.ls
lsharp doc-check examples/metadata.ls --emit-trailers
bash scripts/ci/doc-status-check.sh
```

`lsharp doc-check --emit-trailers` は `.lsharp-doc-status` を読み、CI や PR comment に載せる `Doc-Review-Status` / `Doc-Reviewed-By` を出力します。現行 CI の正本 gate は `scripts/ci/doc-status-check.sh` で、この command が `docs-maintainers` reviewer を返すことを確認します。

`lsharp review` は metadata の確認に使えます。default path の ownership は embedded guest と host tooling の移行状況に依存するため、CI では status trailer を返せる `doc-check --emit-trailers` を使います。

## Update Flow

1. metadata を持つ source を変更する。
2. `lsharp review <file>` で freshness と metadata diagnostics を確認する。
3. 内容を確認したら `lsharp doc-ack <name> --reviewer <name>` で ack する。
4. `.lsharp-doc-status` の差分を review し、`bash scripts/ci/doc-status-check.sh` を通す。

現状の CI gate は代表 fixture の dogfooding を固定するための lightweight check です。追跡対象を増やす場合は、このページと `.lsharp-doc-status` の entry を同じ変更で更新します。
