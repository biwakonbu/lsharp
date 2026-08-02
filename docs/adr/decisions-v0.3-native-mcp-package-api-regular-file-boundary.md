# ADR: v0.3 native MCP package API regular-file boundary

## 状態

Verified partial slice（2026-08-02）。

## 背景

`lsharp_package_api` は installed package の `docs/api.json` を offline に投影するが、既存 path が
symlink の場合も target を読み取っていた。package directory 外の JSON を package-owned metadata として
公開できるため、直前に追加した package identity 検査だけでは provenance ownership を保証できない。

## 決定

既存 `docs/api.json` は regular non-symlink file の場合だけ読み取る。symlink、directory、その他の
non-regular entry は `api.json must be a regular non-symlink file` として、JSON 読込や native program
実行より前に拒否する。`api.json` が存在しない場合の source からの in-memory 生成は維持する。

## 証跡

- RED: package directory 外の identity-valid JSON を指す symlink fixture は Rust/native とも受理されていた。
- GREEN: 同一 fixture を両経路が同じ診断で拒否し、native program を実行しない。
- native focused test は既存 identity mismatch fixture と合わせて成功した。
- Linux replay、stage regeneration、live provider/auth、実 target runtime は実行していない。

## 影響

これは package API metadata の local ownership boundary に限る verified partial である。package install、
registry/provider 取得、current-source Mac/Linux runtime、packaged/rollback parity は未検証のため、
`EC-M3-05` と `M3-05-N9` は `[~]` のまま維持する。
