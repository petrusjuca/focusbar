import type { ReactNode } from "react";

// Anel de progresso estilo timer do Google (Rev4 #3): círculo que esvazia
// conforme o tempo passa, com os dígitos grandes no centro. O arco mostra o
// tempo RESTANTE; no overtime/pós-pausa fica cheio e quem chama pisca o card.
export function TimerRing({
  fraction, // 0..1 = fatia restante do tempo
  size = 168,
  stroke = 7,
  children,
}: {
  fraction: number;
  size?: number;
  stroke?: number;
  children: ReactNode;
}) {
  const f = Math.min(1, Math.max(0, fraction));
  const r = (size - stroke) / 2;
  const c = 2 * Math.PI * r;
  return (
    <div className="timer-ring" style={{ width: size, height: size }}>
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`}>
        {/* trilha */}
        <circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          fill="none"
          stroke="var(--border)"
          strokeWidth={stroke}
        />
        {/* arco do tempo restante — gira -90° pra começar no topo, como o Google */}
        <circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          fill="none"
          stroke="currentColor"
          strokeWidth={stroke}
          strokeLinecap="round"
          strokeDasharray={c}
          strokeDashoffset={c * (1 - f)}
          transform={`rotate(-90 ${size / 2} ${size / 2})`}
          style={{ transition: "stroke-dashoffset 0.9s linear" }}
        />
      </svg>
      <div className="timer-ring-center">{children}</div>
    </div>
  );
}
