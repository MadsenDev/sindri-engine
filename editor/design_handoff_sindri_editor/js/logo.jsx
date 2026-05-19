// Three logo concepts. Pure geometry, no fills except orange accents.
// All built to render at any size; the editor uses 28px in the topbar.

// ── Logo 1 ─ ANVIL: a clean editorial mark. Negative-space anvil + ember dot.
const LogoAnvil = ({ size = 28, color = "var(--ink)", accent = "var(--orange)" }) => (
  <svg width={size} height={size} viewBox="0 0 32 32" fill="none">
    {/* Anvil body */}
    <path
      d="M4 12 H28 L26 16 H22 V20 H28 V24 H4 V20 H10 V16 H6 Z"
      fill={color}
    />
    {/* Ember */}
    <circle cx="16" cy="7" r="2.4" fill={accent} />
    {/* Spark trace */}
    <path d="M16 4 V2 M18.5 5.5 L19.8 4.2 M13.5 5.5 L12.2 4.2"
      stroke={accent} strokeWidth="1.6" strokeLinecap="square" />
  </svg>
);

// ── Logo 2 ─ FORGE HEX: refined version of the current hex.
// Single outer ring + inner hex + ember hex at center.
const LogoForge = ({ size = 28, color = "var(--ink)", accent = "var(--orange)" }) => {
  const hex = (r, cx = 16, cy = 16) => {
    const pts = [];
    for (let i = 0; i < 6; i++) {
      const a = (Math.PI / 3) * i - Math.PI / 2;
      pts.push((cx + r * Math.cos(a)).toFixed(2) + "," + (cy + r * Math.sin(a)).toFixed(2));
    }
    return pts.join(" ");
  };
  return (
    <svg width={size} height={size} viewBox="0 0 32 32" fill="none">
      <polygon points={hex(13)} stroke={color} strokeWidth="1.6" fill="none" />
      <polygon points={hex(7)}  stroke={color} strokeWidth="1.2" fill="none" />
      <polygon points={hex(2.8)} fill={accent} />
    </svg>
  );
};

// ── Logo 3 ─ RUNE 'S': a Norse-inspired stroke monogram.
// Two opposing diagonals form an angular 'S'; a small ember interrupts the stroke.
const LogoRune = ({ size = 28, color = "var(--ink)", accent = "var(--orange)" }) => (
  <svg width={size} height={size} viewBox="0 0 32 32" fill="none">
    {/* Top stroke */}
    <path d="M6 7 H22 L14 15" stroke={color} strokeWidth="2.4" strokeLinecap="square" />
    {/* Bottom stroke */}
    <path d="M10 17 L18 17 L26 25 H10" stroke={color} strokeWidth="2.4" strokeLinecap="square" />
    {/* Ember between strokes */}
    <circle cx="16" cy="16" r="1.8" fill={accent} />
  </svg>
);

// Wordmark — same Instrument Serif across all three with subtle italic on 'i'
const Wordmark = ({ size = 22, color = "var(--ink)", accent = "var(--orange)" }) => (
  <span style={{
    fontFamily: 'var(--f-display)',
    fontSize: size,
    fontWeight: 400,
    letterSpacing: '0.04em',
    color,
    lineHeight: 1,
    display: 'inline-flex',
    alignItems: 'baseline',
  }}>
    <span>S</span>
    <span style={{ fontStyle: 'italic', color: accent }}>i</span>
    <span>ndri</span>
  </span>
);

// Logo + wordmark lockup for the editor topbar
const LogoLockup = ({ which = "anvil", size = 22 }) => {
  const Mark = which === "forge" ? LogoForge : which === "rune" ? LogoRune : LogoAnvil;
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
      <Mark size={size + 6} />
      <Wordmark size={size} />
    </div>
  );
};

Object.assign(window, { LogoAnvil, LogoForge, LogoRune, Wordmark, LogoLockup });
