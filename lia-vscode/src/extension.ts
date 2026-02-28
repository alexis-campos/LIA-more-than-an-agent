// lia-vscode/src/extension.ts
// Fase 2 + Fase 7: Extraccion de Contexto Dinamico (Contrato A)
// con Exponential Backoff y Dynamic Port Discovery.

import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import WebSocket from 'ws';

// Configuracion
const DEFAULT_PORT = 3333;
const DEBOUNCE_MS = 1000;
const CONTEXT_RADIUS = 50;

// Exponential Backoff config
const BACKOFF_INITIAL_MS = 1000;
const BACKOFF_MAX_MS = 30000;
const BACKOFF_MULTIPLIER = 2;
const BACKOFF_JITTER = 0.2;       // ±20%

let ws: WebSocket | null = null;
let debounceTimer: ReturnType<typeof setTimeout> | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let isConnected = false;
let currentBackoff = BACKOFF_INITIAL_MS;

/**
 * Lee el puerto del archivo ~/.lia/port.
 * Si no existe, usa el puerto por defecto 3333.
 */
function discoverPort(): number {
    try {
        const portFile = path.join(os.homedir(), '.lia', 'port');
        const content = fs.readFileSync(portFile, 'utf-8').trim();
        const port = parseInt(content, 10);
        if (!isNaN(port) && port > 0 && port < 65536) {
            console.log(`Puerto descubierto desde ~/.lia/port: ${port}`);
            return port;
        }
    } catch {
        // Archivo no existe, usar default
    }
    console.log(`Usando puerto por defecto: ${DEFAULT_PORT}`);
    return DEFAULT_PORT;
}

/**
 * Calcula el delay con jitter aleatorio para evitar thundering herd.
 */
function getBackoffDelay(): number {
    const jitter = 1 + (Math.random() * 2 - 1) * BACKOFF_JITTER;
    return Math.min(currentBackoff * jitter, BACKOFF_MAX_MS);
}

/**
 * Construye el payload del Contrato A.
 */
function buildContextPayload(): object | null {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        return null;
    }

    const doc = editor.document;
    const cursorLine = editor.selection.active.line;

    const totalLines = doc.lineCount;
    const startLine = Math.max(0, cursorLine - CONTEXT_RADIUS);
    const endLine = Math.min(totalLines - 1, cursorLine + CONTEXT_RADIUS);

    const range = new vscode.Range(startLine, 0, endLine, doc.lineAt(endLine).text.length);
    const contentWindow = doc.getText(range);

    const workspaceName = vscode.workspace.workspaceFolders?.[0]?.name ?? 'unknown';

    return {
        event_type: 'context_update',
        timestamp: Math.floor(Date.now() / 1000),
        workspace_name: workspaceName,
        file_context: {
            file_name: doc.fileName.split('/').pop() ?? doc.fileName,
            file_path: doc.fileName,
            language: doc.languageId,
            cursor_line: cursorLine + 1,
            content_window: contentWindow,
        },
    };
}

/**
 * Envia el contexto con debounce de 1 segundo.
 */
function sendContextDebounced(): void {
    if (debounceTimer) {
        clearTimeout(debounceTimer);
    }

    debounceTimer = setTimeout(() => {
        if (!ws || !isConnected) {
            return;
        }

        const payload = buildContextPayload();
        if (payload) {
            ws.send(JSON.stringify(payload));
        }
    }, DEBOUNCE_MS);
}

/**
 * Conexion WebSocket con Exponential Backoff y Dynamic Port Discovery.
 */
function connectWebSocket(context: vscode.ExtensionContext): void {
    if (reconnectTimer) {
        clearTimeout(reconnectTimer);
        reconnectTimer = null;
    }

    const port = discoverPort();
    const wsUrl = `ws://127.0.0.1:${port}/ws`;

    ws = new WebSocket(wsUrl);

    ws.on('open', () => {
        isConnected = true;
        currentBackoff = BACKOFF_INITIAL_MS; // Reset backoff on success
        vscode.window.showInformationMessage('Lia conectada al editor!');
        console.log('Conectado a Lia en', wsUrl);

        const payload = buildContextPayload();
        if (payload) {
            ws!.send(JSON.stringify(payload));
        }
    });

    ws.on('error', (error) => {
        console.error('Error conectando con Lia Desktop:', error.message);
    });

    ws.on('close', () => {
        isConnected = false;
        const delay = getBackoffDelay();
        console.log(`Conexion cerrada. Reintentando en ${Math.round(delay)}ms (backoff: ${currentBackoff}ms)`);

        reconnectTimer = setTimeout(() => {
            connectWebSocket(context);
        }, delay);

        // Incrementar backoff para el siguiente intento
        currentBackoff = Math.min(currentBackoff * BACKOFF_MULTIPLIER, BACKOFF_MAX_MS);
    });
}

export function activate(context: vscode.ExtensionContext) {
    console.log('La extension de Lia esta activa. Conectando...');

    connectWebSocket(context);

    const onEditorChange = vscode.window.onDidChangeActiveTextEditor(() => {
        sendContextDebounced();
    });

    const onDocChange = vscode.workspace.onDidChangeTextDocument((e) => {
        const activeDoc = vscode.window.activeTextEditor?.document;
        if (activeDoc && e.document === activeDoc) {
            sendContextDebounced();
        }
    });

    const onSelectionChange = vscode.window.onDidChangeTextEditorSelection(() => {
        sendContextDebounced();
    });

    context.subscriptions.push(
        onEditorChange,
        onDocChange,
        onSelectionChange,
        {
            dispose: () => {
                if (debounceTimer) { clearTimeout(debounceTimer); }
                if (reconnectTimer) { clearTimeout(reconnectTimer); }
                if (ws) { ws.close(); }
            },
        }
    );
}

export function deactivate() { }