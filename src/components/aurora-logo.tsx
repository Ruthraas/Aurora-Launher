/** Símbolo da marca: um cubo isométrico com uma faixa de "aurora"
 * atravessando, conforme docs/design/logo (screens/Logo e marca.png).
 * Só o topo usa o gradiente accent→#818cf8 — é o único lugar do app
 * fora das barras de progresso que tem permissão pra gradiente. */
export function AuroraLogo({ size = 24, className }: { size?: number; className?: string }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 32 32"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      role="img"
      aria-label="Aurora Launcher"
    >
      <defs>
        <linearGradient id="aurora-logo-gradient" x1="4" y1="4" x2="28" y2="16" gradientUnits="userSpaceOnUse">
          <stop stopColor="var(--aurora-from, #2dd4bf)" />
          <stop offset="1" stopColor="var(--aurora-to, #818cf8)" />
        </linearGradient>
      </defs>
      <path d="M16 3 L28 9.5 L16 16 L4 9.5 Z" fill="url(#aurora-logo-gradient)" />
      <path d="M4 9.5 L16 16 L16 29 L4 22.5 Z" fill="currentColor" opacity="0.16" />
      <path d="M28 9.5 L16 16 L16 29 L28 22.5 Z" fill="currentColor" opacity="0.28" />
      <path
        d="M4 22.5 L16 29 L28 22.5"
        stroke="url(#aurora-logo-gradient)"
        strokeWidth="1.2"
        strokeLinejoin="round"
        fill="none"
        opacity="0.6"
      />
    </svg>
  );
}
