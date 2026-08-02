# ADR: v0.3 native MCP project-context Git credential boundary

## 状態

Verified partial slice（2026-08-02）。

## 背景

`lsharp_project_context` は `lsharp.toml` の Git 依存元を read-only metadata として返すが、
URL authority に埋め込まれた credential もそのまま MCP 応答へ投影していた。これでは offline
projection が secret を応答やログへ漏らし、未実装の provider/auth 境界を暗黙に迂回できる。

## 決定

Rust/native の両経路で、`scheme://authority/path` 形式の Git URL の authority に `@` が含まれる場合、
`dependencies.<name>.git に credentials を含められません` として projection 前に拒否する。
通常の HTTPS URL は引き続き受理する。registry 取得、Git clone、認証、network access は追加しない。

## 証跡

- 同一 `https://token@example.invalid/bad.git` fixture を Rust/native focused test に追加した。
- RED では credential を含む URL が dependency metadata として返された。
- GREEN では両経路が同じ診断で拒否し、native program は実行されない。
- Linux replay、stage regeneration、実 provider/auth、実 Git access は実行していない。

## 影響

offline project-context が URL credential を公開する経路を閉じたが、live provider/auth、package install、
current-source Mac/Linux runtime、packaged/rollback parity は未検証である。`EC-M3-05` と
`M3-05-N9` は `[~]` のまま維持する。
