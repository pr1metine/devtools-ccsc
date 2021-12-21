import * as path from 'path';
import { window, ExtensionContext } from 'vscode';

import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: ExtensionContext) {
    window.showInformationMessage('CCS C LSP active!');
    let executable = context.asAbsolutePath(path.join('..', 'target', 'release', 'lsp-ccs-c'));

    let serverOptions: ServerOptions = {
        run: {
            command: executable
        },
        debug: {
            command: executable
        }
    };

    let clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'ccsc' }],
    };

    client = new LanguageClient(
        'lsp-ccs-c',
        'CCS C Language Server',
        serverOptions,
        clientOptions
    );

    client.start();
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
