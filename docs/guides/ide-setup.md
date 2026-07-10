# IDE and LSP Setup

L# の editor integration は `lsharp lsp` を入口にします。公開 CLI では `parse` / `check` / `fmt` の直接利用より、LSP や MCP を経由する使い方を優先します。

## LSP Server

```bash
lsharp lsp
```

LSP は diagnostics、hover、completion、definition、references、rename、formatting を提供します。エディタ側は `lsharp` binary を PATH から見つけるか、明示した path で起動します。

## VS Code, Cursor, Windsurf

VS Code 系の拡張は `editors/vscode/` にあります。

```bash
bash scripts/install-vscode-ext.sh
```

`lsharp` が PATH にない場合は VS Code 設定の `lsharp.lspPath` に binary path を指定します。

## Neovim

Neovim 用の設定は `editors/neovim/` にあります。`nvim-lspconfig` を使う場合は `cmd = { 'lsharp', 'lsp' }` を指定します。

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.lsharp then
  configs.lsharp = {
    default_config = {
      cmd = { 'lsharp', 'lsp' },
      filetypes = { 'lsharp' },
      root_dir = lspconfig.util.root_pattern('lsharp.toml', 'Cargo.toml', '.git'),
      settings = {},
    },
  }
end

lspconfig.lsharp.setup{}
vim.filetype.add({ extension = { ls = 'lsharp' } })
```

## JetBrains

JetBrains 向けの現状メモは `editors/jetbrains/README.md` を参照します。LSP 接続時の基本方針は同じで、server command は `lsharp lsp` です。

## AI Tools

AI 連携には MCP server を使います。

```bash
lsharp mcp-server
lsharp claude-plugin
lsharp language-guide
```

`lsharp claude-plugin` は Claude Code に MCP server 設定と L# language guide skill を配置します。`lsharp language-guide` は同じ guide Markdown を標準出力へ出します。

## Known Limits

- Windows は通常サポート対象として案内しません。
- PATH 解決や editor extension の install はエディタごとの設定に依存します。
- LSP の細かい内部実装は book の LSP 章で扱います。利用者向けには `lsharp lsp` を入口にします。
