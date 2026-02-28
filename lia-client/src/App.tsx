// App.tsx
// Componente raiz del HUD de Lia.
// Orquesta el estado de la maquina, escucha eventos de Tauri,
// y renderiza el orbe, el texto streaming, y la barra de contexto.

import { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
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
  const [context, setContext] = useState<ContextInfo>({
    fileName: '',
    language: '',
    cursorLine: 0,
  });

  useEffect(() => {
    // Escuchar cambios de estado desde Rust
    const unlistenState = listen<string>('lia://state-change', (event) => {
      setState(event.payload as LiaState);
    });

    // Escuchar chunks de texto streaming desde Gemini
    const unlistenChunk = listen<string>('lia://stream-chunk', (event) => {
      setStreamText((prev) => prev + event.payload);
    });

    // Escuchar fin del stream
    const unlistenEnd = listen('lia://stream-end', () => {
      setState('IDLE');
    });

    // Escuchar actualizaciones de contexto (archivo/linea)
    const unlistenContext = listen<ContextInfo>('lia://context-update', (event) => {
      setContext(event.payload);
    });

    return () => {
      unlistenState.then((fn) => fn());
      unlistenChunk.then((fn) => fn());
      unlistenEnd.then((fn) => fn());
      unlistenContext.then((fn) => fn());
    };
  }, []);

  // Cerrar la ventana
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
