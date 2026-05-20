export default function ForgeLogo({ size = 22 }: { size?: number }) {
  const s = size;
  const hex = (r: number) => {
    const pts: string[] = [];
    for (let i = 0; i < 6; i++) {
      const a = (Math.PI / 180) * (60 * i - 90);
      pts.push(`${s / 2 + r * Math.cos(a)},${s / 2 + r * Math.sin(a)}`);
    }
    return pts.join(" ");
  };
  const r1 = s * 0.41;
  const r2 = s * 0.22;
  const r3 = s * 0.088;
  return (
    <svg width={s} height={s} viewBox={`0 0 ${s} ${s}`} style={{ flexShrink: 0 }}>
      <polygon points={hex(r1)} fill="none" stroke="var(--ink-2)" strokeWidth="1.6" />
      <polygon points={hex(r2)} fill="none" stroke="var(--ink-2)" strokeWidth="1.2" />
      <polygon points={hex(r3)} fill="var(--amber)" />
    </svg>
  );
}
