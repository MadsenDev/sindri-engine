// v3 logo overrides — uppercase pixel wordmark, no italic.
// Re-exposes window.Wordmark + window.LogoLockup with v3 styling.

const WordmarkV3 = ({ size = 18, color = "var(--ink)", accent = "var(--orange)" }) => (
  <span style={{
    fontFamily: 'Pixelify Sans, monospace',
    fontWeight: 700,
    fontSize: size,
    letterSpacing: '0.14em',
    color,
    lineHeight: 1,
    display: 'inline-flex',
    alignItems: 'center',
    textTransform: 'uppercase',
  }}>
    <span>S</span>
    <span style={{ color: accent }}>I</span>
    <span>NDRI</span>
  </span>
);

const LogoLockupV3 = ({ which = "forge", size = 18 }) => {
  const Mark = which === "forge" ? window.LogoForge
             : which === "rune"  ? window.LogoRune
             : window.LogoAnvil;
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
      <Mark size={size + 8} color="var(--ink)" accent="var(--orange)"/>
      <WordmarkV3 size={size}/>
    </div>
  );
};

// Override globals so the editor picks these up.
window.Wordmark   = WordmarkV3;
window.LogoLockup = LogoLockupV3;
