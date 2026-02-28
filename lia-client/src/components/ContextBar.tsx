// ContextBar.tsx
// Barra inferior compacta que muestra que archivo esta observando Lia.

interface ContextBarProps {
    fileName: string;
    language: string;
    cursorLine: number;
}

export default function ContextBar({ fileName, language, cursorLine }: ContextBarProps) {
    const hasContext = fileName !== '';

    return (
        <div className="context-bar">
            <span
                className="context-dot"
                style={{ background: hasContext ? 'var(--color-accent-responding)' : 'var(--color-text-dim)' }}
            />
            <span className="context-text">
                {hasContext
                    ? `${fileName}:${cursorLine} (${language})`
                    : 'Sin archivo activo'
                }
            </span>
        </div>
    );
}
