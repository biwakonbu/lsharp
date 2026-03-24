// L# VSCode 拡張 - 最小シェル
//
// 機能:
// 1. TextMate grammar によるシンタックスハイライト (syntaxes/lsharp.tmLanguage.json)
// 2. 言語設定 (括弧の自動閉じ、コメント設定)
// 3. 将来的な LSP クライアント統合の基盤

import * as vscode from 'vscode';

export function activate(context: vscode.ExtensionContext) {
    console.log('L# language extension activated');

    // ステータスバーに L# 表示
    const statusBarItem = vscode.window.createStatusBarItem(
        vscode.StatusBarAlignment.Right,
        100
    );
    statusBarItem.text = 'L#';
    statusBarItem.tooltip = 'L# Language Support';

    // .ls ファイルが開かれたときにステータスバーを表示
    vscode.window.onDidChangeActiveTextEditor((editor) => {
        if (editor && editor.document.languageId === 'lsharp') {
            statusBarItem.show();
        } else {
            statusBarItem.hide();
        }
    }, null, context.subscriptions);

    // 現在のエディタが .ls ファイルの場合、即座に表示
    if (vscode.window.activeTextEditor?.document.languageId === 'lsharp') {
        statusBarItem.show();
    }

    context.subscriptions.push(statusBarItem);
}

export function deactivate() {
    console.log('L# language extension deactivated');
}
