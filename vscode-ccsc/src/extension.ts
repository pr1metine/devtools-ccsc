import * as path from 'path';
import { window, ExtensionContext, commands, workspace } from 'vscode';
import * as cp from 'child_process';

import {
    Executable,
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: ExtensionContext) {
    let executable = context.asAbsolutePath(path.join('..', 'target', 'debug', 'ls-ccsc'));

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
        'ls-ccsc',
        'CCS C Language Server',
        serverOptions,
        clientOptions
    );

    client.start();

    if (workspace.workspaceFolders === undefined) {
        window.showErrorMessage("No workspace folders found! Exiting...");
        return;
    }

    let compileCommand: Executable = {
        command: 'ccsc.exe'.replace(/(["\s'$`\\])/g,'\\$1'),
        args: ['+FM', `${path.join(workspace.workspaceFolders[0].uri.fsPath, "main.c")}`, '+DF', '+LN', '+T', '+A', '+M', '+Z', '+Y=9', '+EA'].map((s) => s.replace(/(["\s'$`\\])/g,'\\$1'))
    };

    let disposable = commands.registerCommand('vscode-ccsc.compile', () => {
        window.showInformationMessage(`Compiling ${workspace.rootPath}...`);
        cp.exec(`${compileCommand.command} ${compileCommand.args?.join(" ")}`);
        window.showInformationMessage('Done compiling!');
    });

    context.subscriptions.push(disposable);

    window.showInformationMessage('CCS C LSP Extension active!');
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
