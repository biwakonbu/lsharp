# L# Neovim 設定

Neovim (nvim-lspconfig) 向けの L# LSP 統合。

## 対応プラットフォーム

macOS / Linux のみ。Windows は動作保証の対象外。

## 前提条件

- Neovim 0.9+
- [nvim-lspconfig](https://github.com/neovim/nvim-lspconfig)
- `lsharp` バイナリが PATH に存在すること

## セットアップ

### 方法 1: ファイルコピー

`lsharp.lua` を Neovim の Lua パスにコピー:

```bash
cp editors/neovim/lsharp.lua ~/.config/nvim/lua/lsharp.lua
```

`init.lua` に追加:

```lua
require('lsharp')
```

### 方法 2: インライン設定

`init.lua` に直接記述:

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.lsharp then
  configs.lsharp = {
    default_config = {
      cmd = { 'lsharp', 'lsp' },
      filetypes = { 'lsharp' },
      root_dir = lspconfig.util.root_pattern('Cargo.toml', '.git'),
      settings = {},
    },
  }
end

lspconfig.lsharp.setup{}

vim.filetype.add({ extension = { ls = 'lsharp' } })
```

## 提供機能

- Diagnostics (構文エラー、型エラー)
- Hover (`K` キー)
- Completion (補完プラグインと連携)
- Go to Definition (`gd`)
- Find References (`gr`)
- Rename (`<leader>rn`)
- Document Formatting (`<leader>f`)

## シンタックスハイライト

Tree-sitter grammar は未提供。基本的なハイライトには以下を `init.lua` に追加:

```lua
vim.api.nvim_create_autocmd('FileType', {
  pattern = 'lsharp',
  callback = function()
    vim.bo.commentstring = '; %s'
  end,
})
```
