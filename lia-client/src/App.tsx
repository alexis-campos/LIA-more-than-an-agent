// App.tsx
// Componente raiz del HUD de Lia.
// Orquesta el estado de la maquina, escucha eventos de Tauri,
// y renderiza el orbe, el texto streaming, y la barra de contexto.

import { useState, useEffect, useCallback } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import StatusOrb, { type LiaState } from './components/StatusOrb';
import StreamingText from './components/StreamingText';
import ContextBar from './components/ContextBar';
import './App.css';

interface ContextInfo {
  fileName: string;
  language: string;
  cursorLine: number;
}

function App() {
  const [state, setState] = useState<LiaState>('IDLE');
  const [streamText, setStreamText] = useState('');
  const [isProcessing, setIsProcessing] = useState(false);
  const [context, setContext] = useState<ContextInfo>({
    fileName: '',
    language: '',
    cursorLine: 0,
  });

  useEffect(() => {
    const unlistenState = listen<string>('lia://state-change', (event) => {
      const newState = event.payload as LiaState;
      setState(newState);
      setIsProcessing(newState !== 'IDLE');
    });

    const unlistenChunk = listen<string>('lia://stream-chunk', (event) => {
      setStreamText((prev) => prev + event.payload);
    });

    const unlistenClear = listen('lia://stream-clear', () => {
      setStreamText('');
    });

    const unlistenEnd = listen('lia://stream-end', () => {
      setState('IDLE');
      setIsProcessing(false);
    });

    const unlistenContext = listen<ContextInfo>('lia://context-update', (event) => {
      setContext(event.payload);
    });

    return () => {
      unlistenState.then((fn) => fn());
      unlistenChunk.then((fn) => fn());
      unlistenClear.then((fn) => fn());
      unlistenEnd.then((fn) => fn());
      unlistenContext.then((fn) => fn());
    };
  }, []);

  // Disparar inferencia via Tauri command
  const handleAskLia = useCallback(async () => {
    if (isProcessing) return;
    setIsProcessing(true);
    try {
      await invoke('ask_lia');
    } catch (e) {
      console.error('Error invocando ask_lia:', e);
      setIsProcessing(false);
    }
  }, [isProcessing]);

  const handleClose = () => {
    getCurrentWindow().close();
  };

  return (
    <div className="hud-container">
      {/* Barra superior draggable */}
      <div className="hud-titlebar" data-tauri-drag-region>
        <span className="hud-title">Lia</span>
        <button className="hud-close" onClick={handleClose} aria-label="Cerrar" />
      </div>

      {/* Area principal */}
      <div className="hud-main">
        <StatusOrb state={state} />
        <StreamingText text={streamText} state={state} />

        {/* Boton de accion */}
        <button
          className={`ask-button ${isProcessing ? 'ask-button--disabled' : ''}`}
          onClick={handleAskLia}
          disabled={isProcessing}
        >
          {isProcessing ? 'Procesando...' : 'Preguntar a Lia'}
        </button>
      </div>

      {/* Barra de contexto */}
      <ContextBar
        fileName={context.fileName}
        language={context.language}
        cursorLine={context.cursorLine}
      />
    </div>
  );
}

export default App;
