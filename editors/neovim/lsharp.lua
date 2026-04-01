-- L# LSP 設定 (nvim-lspconfig)
--
-- 使い方:
--   1. nvim-lspconfig をインストール
--   2. init.lua に以下を追加:
--      require('lsharp')  -- このファイルを runtimepath に配置
--
-- 前提: `lsharp` バイナリが PATH に存在すること

local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

-- L# 言語サーバー定義
if not configs.lsharp then
  configs.lsharp = {
    default_config = {
      cmd = { 'lsharp', 'lsp' },
      filetypes = { 'lsharp' },
      root_dir = lspconfig.util.root_pattern('.git', 'project.toml'),
      settings = {},
    },
  }
end

lspconfig.lsharp.setup{}

-- .ls ファイルを lsharp として認識
vim.filetype.add({
  extension = {
    ls = 'lsharp',
  },
})

-- コメント文字設定
vim.api.nvim_create_autocmd('FileType', {
  pattern = 'lsharp',
  callback = function()
    vim.bo.commentstring = '; %s'
  end,
})
