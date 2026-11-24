interface LogoProps {
  size?: number;
  class?: string;
}

/**
 * Brand mark: a TV with a signal in its corner. Same geometry as assets/app-icon.svg,
 * without the tile. The screen follows `currentColor`, the signal uses the accent.
 */
export function Logo(props: LogoProps) {
  const size = () => props.size ?? 24;
  return (
    <svg
      class={props.class}
      width={size()}
      height={size()}
      viewBox="150 216 724 634"
      fill="none"
      aria-hidden="true"
    >
      <rect x="196" y="262" width="632" height="430" rx="84" stroke="currentColor" stroke-width="64" />
      <path d="M404 788H620" stroke="currentColor" stroke-width="64" stroke-linecap="round" />
      <circle cx="318" cy="574" r="38" fill="var(--accent)" />
      <path d="M318 456A118 118 0 0 1 436 574" stroke="var(--accent)" stroke-width="56" stroke-linecap="round" />
      <path d="M318 368A206 206 0 0 1 524 574" stroke="var(--accent)" stroke-width="56" stroke-linecap="round" />
    </svg>
  );
}
