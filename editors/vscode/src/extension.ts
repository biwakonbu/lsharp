// L# VSCode 拡張 - LSP クライアント統合
//
// 機能:
// 1. TextMate grammar によるシンタックスハイライト
// 2. 言語設定 (括弧の自動閉じ、コメント設定)
// 3. LSP クライアント (`lsharp lsp` バイナリを spawn)
//    - diagnostics, goto_definition, references, rename, formatting

import * as vscode from 'vscode';
import * as child_process from 'child_process';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

// lsharp バイナリのパスを検出する
// 優先順: 設定パス → PATH 検索
function findLsharpBinary(): string | null {
    // 設定からパスを取得 (AC-215)
    const configPath = vscode.workspace
        .getConfiguration('lsharp')
        .get<string>('lspPath');
    if (configPath && configPath.trim() !== '') {
        return configPath.trim();
    }

    // PATH から検索
    try {
        const result = child_process
            .execSync('which lsharp', { encoding: 'utf-8' })
            .trim();
        if (result) {
            return result;
        }
    } catch {
        // which が失敗した場合は見つからない
    }

    return null;
}

export function activate(context: vscode.ExtensionContext) {
    console.log('L# language extension activated');

    // ステータスバーに L# 表示
    const statusBarItem = vscode.window.createStatusBarItem(
        vscode.StatusBarAlignment.Right,
        100
    );
    statusBarItem.text = 'L#';
    statusBarItem.tooltip = 'L# Language Support';

    vscode.window.onDidChangeActiveTextEditor(
        (editor) => {
            if (editor && editor.document.languageId === 'lsharp') {
                statusBarItem.show();
            } else {
                statusBarItem.hide();
            }
        },
        null,
        context.subscriptions
    );

    if (vscode.window.activeTextEditor?.document.languageId === 'lsharp') {
        statusBarItem.show();
    }

    context.subscriptions.push(statusBarItem);

    // LSP クライアント起動
    const binaryPath = findLsharpBinary();

    if (!binaryPath) {
        // バイナリが見つからない場合のエラー通知 (AC-214)
        statusBarItem.text = 'L# (no LSP)';
        statusBarItem.tooltip =
            'lsharp バイナリが見つかりません。PATH に追加するか lsharp.lspPath 設定を確認してください。';
        statusBarItem.backgroundColor = new vscode.ThemeColor(
            'statusBarItem.warningBackground'
        );
        vscode.window.showErrorMessage(
            'L#: lsharp バイナリが見つかりません。' +
                'PATH に lsharp を追加するか、設定 lsharp.lspPath でパスを指定してください。'
        );
        return;
    }

    // サーバーオプション: lsharp lsp を spawn (AC-212)
    const serverOptions: ServerOptions = {
        command: binaryPath,
        args: ['lsp'],
    };

    // クライアントオプション
    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'lsharp' }],
    };

    client = new LanguageClient(
        'lsharp',
        'L# Language Server',
        serverOptions,
        clientOptions
    );

    // LSP クライアント開始
    client.start();
    statusBarItem.text = 'L#';
    statusBarItem.tooltip = `L# Language Server (${binaryPath})`;
}

export function deactivate(): Thenable<void> | undefined {
    if (client) {
        return client.stop();
    }
    return undefined;
}
