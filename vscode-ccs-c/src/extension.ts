import * as path from 'path';
import { window, ExtensionContext } from 'vscode';

import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: ExtensionContext) {
    window.showInformationMessage('Hello World!');
    let executable = context.asAbsolutePath(path.join('..', 'target', 'debug', 'lsp-ccs-c'));

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

    // Create the language client and start the client.
    client = new LanguageClient(
        'lsp-ccs-c',
        'CCS C Language Server',
        serverOptions,
        clientOptions
    );

    // Start the client. This will also launch the server
    client.start();
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
