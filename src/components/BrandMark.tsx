import "./BrandMark.css";

interface BrandMarkProps {
  size?: number;
  withWordmark?: boolean;
}

/** Inline, theme-aware rendering of the Foldown "#" badge (see icon.svg / icon_inverse.svg). */
export function BrandMark({ size = 48, withWordmark = false }: BrandMarkProps) {
  return (
    <span className="brand-mark">
      <svg
        width={size}
        height={size}
        viewBox="0 0 256 256"
        className="brand-mark__badge"
        role="img"
        aria-label="Foldown"
      >
        <rect x="10" y="10" width="236" height="236" rx="52" className="brand-mark__fill" />
        <rect x="82" y="60" width="28" height="136" rx="6" className="brand-mark__glyph" />
        <rect x="146" y="60" width="28" height="136" rx="6" className="brand-mark__glyph" />
        <rect x="60" y="82" width="136" height="28" rx="6" className="brand-mark__glyph" />
        <rect x="60" y="146" width="136" height="28" rx="6" className="brand-mark__glyph" />
      </svg>
      {withWordmark && <span className="brand-mark__wordmark">Foldown</span>}
    </span>
  );
}
