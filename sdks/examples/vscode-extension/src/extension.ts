/**
 * Overseer VS Code Extension Example
 *
 * This example shows how to use the overseer-client TypeScript SDK
 * inside a VS Code extension to integrate AI agent capabilities.
 *
 * Features:
 * - List conversations in the VS Code sidebar
 * - Run agent orchestration on selected text
 * - Create new conversations from VS Code commands
 */

import * as vscode from 'vscode';
import { OverseerClient } from 'overseer-client';

let client: OverseerClient | null = null;

/**
 * Get or create the Overseer client from VS Code settings.
 */
function getClient(): OverseerClient {
    if (!client) {
        const config = vscode.workspace.getConfiguration('overseer');
        const baseUrl = config.get<string>('baseUrl', 'http://localhost:8080');
        const token = config.get<string>('token', '');
        client = new OverseerClient({ baseUrl, token });
    }
    return client;
}

/**
 * Activate the extension.
 */
export function activate(context: vscode.ExtensionContext) {
    console.log('Overseer extension activating...');

    // Command: List conversations in a quick pick
    const listConversations = vscode.commands.registerCommand(
        'overseer.listConversations',
        async () => {
            try {
                const c = getClient();
                const conversations = await c.conversations.list();
                const items = conversations.map(conv => ({
                    label: `#${conv.id}`,
                    description: conv.title,
                    detail: new Date(conv.created_at).toLocaleString(),
                }));
                const selected = await vscode.window.showQuickPick(items, {
                    placeHolder: 'Select a conversation',
                });
                if (selected) {
                    vscode.window.showInformationMessage(
                        `Selected conversation #${selected.label}: ${selected.description}`
                    );
                }
            } catch (err) {
                vscode.window.showErrorMessage(`Overseer error: ${err}`);
            }
        }
    );

    // Command: Orchestrate on selected text
    const orchestrateSelection = vscode.commands.registerCommand(
        'overseer.orchestrateSelection',
        async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showWarningMessage('No active editor');
                return;
            }
            const selection = editor.document.getText(editor.selection);
            if (!selection) {
                vscode.window.showWarningMessage('No text selected');
                return;
            }

            try {
                const c = getClient();
                // Use conversation 1, or let user pick
                const conversations = await c.conversations.list();
                const convId = conversations.length > 0 ? conversations[0].id : await c.conversations.create('VS Code');

                vscode.window.withProgress(
                    { location: vscode.ProgressLocation.Notification, title: 'Overseer processing...' },
                    async () => {
                        const result = await c.orchestrate(convId, selection);
                        // Show result in a new document
                        const doc = await vscode.workspace.openTextDocument({
                            content: result,
                            language: 'markdown',
                        });
                        await vscode.window.showTextDocument(doc, { preview: false });
                    }
                );
            } catch (err) {
                vscode.window.showErrorMessage(`Overseer error: ${err}`);
            }
        }
    );

    // Command: Create conversation
    const createConversation = vscode.commands.registerCommand(
        'overseer.createConversation',
        async () => {
            const title = await vscode.window.showInputBox({
                placeHolder: 'Conversation title',
                prompt: 'Enter a title for the new conversation',
            });
            if (!title) return;

            try {
                const c = getClient();
                const id = await c.conversations.create(title);
                vscode.window.showInformationMessage(
                    `Created conversation #${id}: ${title}`
                );
            } catch (err) {
                vscode.window.showErrorMessage(`Overseer error: ${err}`);
            }
        }
    );

    context.subscriptions.push(listConversations, orchestrateSelection, createConversation);
}

/**
 * Deactivate the extension.
 */
export function deactivate() {
    client = null;
}
