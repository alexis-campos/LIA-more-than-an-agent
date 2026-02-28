// StatusOrb.tsx
// Orbe animado que indica el estado actual de Lia.
// Cambia de color y animacion segun el estado de la maquina.

import { motion, type TargetAndTransition } from 'framer-motion';

export type LiaState = 'IDLE' | 'LISTENING' | 'THINKING' | 'RESPONDING';

interface StatusOrbProps {
    state: LiaState;
}

const stateConfig: Record<LiaState, { color: string; label: string }> = {
    IDLE: { color: 'var(--color-accent-idle)', label: 'Lista' },
    LISTENING: { color: 'var(--color-accent-listening)', label: 'Escuchando...' },
    THINKING: { color: 'var(--color-accent-thinking)', label: 'Pensando...' },
    RESPONDING: { color: 'var(--color-accent-responding)', label: 'Respondiendo...' },
};

const orbVariants: Record<LiaState, TargetAndTransition> = {
    IDLE: {
        scale: [1, 1.08, 1],
        opacity: [0.7, 1, 0.7],
        transition: { duration: 3, repeat: Infinity, ease: 'easeInOut' },
    },
    LISTENING: {
        scale: [1, 1.15, 1],
        opacity: [0.8, 1, 0.8],
        transition: { duration: 0.8, repeat: Infinity, ease: 'easeInOut' },
    },
    THINKING: {
        rotate: [0, 360],
        scale: [1, 1.05, 1],
        transition: {
            rotate: { duration: 2, repeat: Infinity, ease: 'linear' },
            scale: { duration: 1, repeat: Infinity },
        },
    },
    RESPONDING: {
        scale: 1.05,
        opacity: 1,
        transition: { duration: 0.3 },
    },
};

export default function StatusOrb({ state }: StatusOrbProps) {
    const config = stateConfig[state];

    return (
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '8px' }}>
            <div style={{ position: 'relative', width: '64px', height: '64px' }}>
                {/* Glow difuso detras */}
                <motion.div
                    animate={orbVariants[state]}
                    style={{
                        position: 'absolute',
                        inset: '-8px',
                        borderRadius: '50%',
                        background: config.color,
                        filter: 'blur(16px)',
                        opacity: 0.35,
                    }}
                />
                {/* Orbe principal */}
                <motion.div
                    animate={orbVariants[state]}
                    style={{
                        width: '64px',
                        height: '64px',
                        borderRadius: '50%',
                        background: `radial-gradient(circle at 35% 35%, ${config.color}, rgba(0,0,0,0.4))`,
                        border: '1px solid rgba(255,255,255,0.15)',
                        position: 'relative',
                    }}
                />
            </div>
            <motion.span
                key={state}
                initial={{ opacity: 0, y: 4 }}
                animate={{ opacity: 1, y: 0 }}
                style={{
                    fontSize: '11px',
                    fontWeight: 500,
                    color: config.color,
                    letterSpacing: '0.5px',
                }}
            >
                {config.label}
            </motion.span>
        </div>
    );
}
