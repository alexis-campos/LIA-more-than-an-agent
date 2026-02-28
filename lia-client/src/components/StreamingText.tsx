// StreamingText.tsx
// Panel que muestra la respuesta de Gemini con efecto streaming.
// Auto-scroll y cursor parpadeante.

import { useEffect, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import type { LiaState } from './StatusOrb';

interface StreamingTextProps {
    text: string;
    state: LiaState;
}

export default function StreamingText({ text, state }: StreamingTextProps) {
    const containerRef = useRef<HTMLDivElement>(null);

    // Auto-scroll cuando llega nuevo texto
    useEffect(() => {
        if (containerRef.current) {
            containerRef.current.scrollTop = containerRef.current.scrollHeight;
        }
    }, [text]);

    const isStreaming = state === 'RESPONDING';
    const isEmpty = !text;

    return (
        <div className="streaming-panel" ref={containerRef}>
            <AnimatePresence mode="wait">
                {isEmpty ? (
                    <motion.div
                        key="placeholder"
                        className="streaming-placeholder"
                        initial={{ opacity: 0 }}
                        animate={{ opacity: 1 }}
                        exit={{ opacity: 0 }}
                    >
                        {state === 'IDLE' && 'Lia esta lista para ayudarte.'}
                        {state === 'LISTENING' && 'Escuchando tu voz...'}
                        {state === 'THINKING' && 'Analizando tu codigo...'}
                    </motion.div>
                ) : (
                    <motion.div
                        key="content"
                        initial={{ opacity: 0 }}
                        animate={{ opacity: 1 }}
                    >
                        <span className="streaming-text">{text}</span>
                        {isStreaming && <span className="streaming-cursor" />}
                    </motion.div>
                )}
            </AnimatePresence>
        </div>
    );
}
