// lia-vscode/src/extension.ts
import * as vscode from 'vscode';
import WebSocket from 'ws';

export function activate(context: vscode.ExtensionContext) {
    console.log('La extensión de Lia está activa. Intentando conectar...');

    // Conectamos al servidor local de Rust
    const ws = new WebSocket('ws://127.0.0.1:3333/ws');

    ws.on('open', () => {
        vscode.window.showInformationMessage('¡Lia conectada al editor!');
        
        // Enviamos nuestro primer payload de contexto
        const payload = {
            event: "saludo",
            editor: "VS Code",
            message: "Hola desde las entrañas del código"
        };
        
        ws.send(JSON.stringify(payload));
    });

    ws.on('error', (error) => {
        console.error('Error conectando con Lia Desktop:', error);
    });

    ws.on('close', () => {
        console.log('Conexión con Lia Desktop cerrada.');
    });

    // Esto asegura que si desactivas la extensión, el socket se cierre limpio
    context.subscriptions.push({
        dispose: () => ws.close()
    });
}

export function deactivate() {}