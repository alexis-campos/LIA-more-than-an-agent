// lia-vscode/src/extension.ts
// Fase 2: Extracción de Contexto Dinámico (Contrato A: context_update)

import * as vscode from 'vscode';
import WebSocket from 'ws';

// Configuración
const WS_URL = 'ws://127.0.0.1:3333/ws';
const DEBOUNCE_MS = 1000;       // 1 segundo de inactividad antes de enviar
const RECONNECT_MS = 2000;      // Reintentar conexión cada 2 segundos
const CONTEXT_RADIUS = 50;      // ±50 líneas alrededor del cursor

let ws: WebSocket | null = null;
let debounceTimer: ReturnType<typeof setTimeout> | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let isConnected = false;

/**
 * Construye el payload del Contrato A a partir del estado actual del editor.
 * Extrae: nombre del archivo, ruta, lenguaje, línea del cursor,
 * y una ventana de ±50 líneas alrededor del cursor.
 */
function buildContextPayload(): object | null {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        return null;
    }

    const doc = editor.document;
    const cursorLine = editor.selection.active.line; // 0-indexed

    // Calculamos los límites de la ventana de contexto (±50 líneas)
    const totalLines = doc.lineCount;
    const startLine = Math.max(0, cursorLine - CONTEXT_RADIUS);
    const endLine = Math.min(totalLines - 1, cursorLine + CONTEXT_RADIUS);

    // Extraemos las líneas del rango
    const range = new vscode.Range(startLine, 0, endLine, doc.lineAt(endLine).text.length);
    const contentWindow = doc.getText(range);

    // Nombre del workspace activo
    const workspaceName = vscode.workspace.workspaceFolders?.[0]?.name ?? 'unknown';

    return {
        event_type: 'context_update',
        timestamp: Math.floor(Date.now() / 1000),
        workspace_name: workspaceName,
        file_context: {
            file_name: doc.fileName.split('/').pop() ?? doc.fileName,
            file_path: doc.fileName,
            language: doc.languageId,
            cursor_line: cursorLine + 1, // Convertimos a 1-indexed para el humano/Rust
            content_window: contentWindow,
        },
    };
}

/**
 * Envía el contexto actual al servidor de Rust, con debounce de 1 segundo.
 * Si el usuario edita o mueve el cursor rápidamente, solo se envía
 * un mensaje después de que se "calme" durante 1 segundo.
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
 * Establece la conexión WebSocket con Lia Client (Rust/Tauri).
 * Incluye lógica de reconexión básica para comodidad en desarrollo.
 */
function connectWebSocket(context: vscode.ExtensionContext): void {
    // Limpiar reconexión previa si existe
    if (reconnectTimer) {
        clearTimeout(reconnectTimer);
        reconnectTimer = null;
    }

    ws = new WebSocket(WS_URL);

    ws.on('open', () => {
        isConnected = true;
        vscode.window.showInformationMessage('¡Lia conectada al editor!');
        console.log('Conectado al servidor de Lia en', WS_URL);

        // Enviamos el contexto inicial inmediatamente al conectar
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
        console.log('Conexión con Lia Desktop cerrada. Reintentando en', RECONNECT_MS, 'ms...');

        // Reconexión básica con intervalo fijo (Fase 7 tendrá backoff exponencial)
        reconnectTimer = setTimeout(() => {
            connectWebSocket(context);
        }, RECONNECT_MS);
    });
}

export function activate(context: vscode.ExtensionContext) {
    console.log('La extensión de Lia está activa. Conectando...');

    // 1. Establecer conexión WebSocket
    connectWebSocket(context);

    // 2. Registrar los listeners del editor (Paso 2.1)

    // Cuando el usuario cambia de pestaña/archivo
    const onEditorChange = vscode.window.onDidChangeActiveTextEditor(() => {
        sendContextDebounced();
    });

    // Cuando el usuario edita el contenido del archivo
    const onDocChange = vscode.workspace.onDidChangeTextDocument((e) => {
        // Solo nos interesa el documento del editor activo
        const activeDoc = vscode.window.activeTextEditor?.document;
        if (activeDoc && e.document === activeDoc) {
            sendContextDebounced();
        }
    });

    // Cuando el usuario mueve el cursor o selecciona texto
    const onSelectionChange = vscode.window.onDidChangeTextEditorSelection(() => {
        sendContextDebounced();
    });

    // 3. Registrar los disposables para limpieza
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