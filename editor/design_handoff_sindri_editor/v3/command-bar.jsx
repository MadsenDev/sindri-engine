// v2 Command bar — paper card, clean lines, no small-caps.

const v3cb = {
  scrim: {
    position: 'absolute', inset: 0,
    background: 'rgba(0,0,0,0.55)',
    zIndex: 50,
    display: 'flex', alignItems: 'flex-start', justifyContent: 'center',
    paddingTop: 140,
  },
  panel: {
    width: 720,
    background: V3.PAPER,
    border: `1px solid ${V3.RULE2}`,
    boxShadow: '0 24px 60px rgba(0,0,0,0.18)',
  },
  head: {
    padding: '18px 22px 16px',
    borderBottom: `1px solid ${V3.RULE}`,
    display: 'flex', alignItems: 'center', gap: 12,
  },
  prompt: {
    fontFamily: V3.F_DISPLAY, fontSize: 26, lineHeight: 1.1, color: V3.INK,
    flex: 1, },
  caret: { display: 'inline-block', width: 2, height: 26, background: V3.ORANGE, animation: 'v3caret 1s steps(1) infinite' },
  ctxbar: {
    padding: '10px 22px',
    borderBottom: `1px solid ${V3.RULE}`,
    display: 'flex', gap: 8, alignItems: 'center', flexWrap: 'wrap',
  },
  ctxchip: (kind) => ({
    fontSize: 11, fontFamily: V3.F_MONO,
    padding: '3px 8px',
    border: `1px solid ${kind === 'entity' ? V3.NAVY : V3.RULE2}`,
    color: kind === 'entity' ? V3.NAVY : V3.INK3,
    background: kind === 'entity' ? 'rgba(109,188,219,0.10)' : 'transparent',
    display: 'inline-flex', gap: 6, alignItems: 'center',
  }),
  body: { padding: '6px 0 8px', maxHeight: 380, overflowY: 'auto' },
  group: { padding: '12px 22px 4px' },
  groupLabel: { fontSize: 11.5, color: V3.INK3, marginBottom: 6 },
  item: (active) => ({
    display: 'flex', alignItems: 'center', gap: 12,
    padding: '8px 22px',
    background: active ? V3.PAPER2 : 'transparent',
    fontSize: 13, color: V3.INK,
    cursor: 'pointer', whiteSpace: 'nowrap',
    borderLeft: active ? `2px solid ${V3.ORANGE}` : '2px solid transparent',
  }),
  glyph: (kind) => ({
    width: 16, display: 'inline-flex', justifyContent: 'center',
    color: kind === '✦' ? V3.ORANGE : V3.INK4,
    fontFamily: kind === '✦' ? V3.F_DISPLAY : V3.F_MONO,
    fontSize: kind === '✦' ? 14 : 12,
  }),
  hint: { marginLeft: 'auto', fontFamily: V3.F_MONO, fontSize: 11, color: V3.INK4 },
  foot: {
    borderTop: `1px solid ${V3.RULE}`,
    padding: '10px 22px',
    display: 'flex', alignItems: 'center', justifyContent: 'space-between',
    fontFamily: V3.F_MONO, fontSize: 11, color: V3.INK4,
  },
};

function V2CommandBar({ open, onClose, prompt = '', ctx = [] }) {
  if (!open) return null;
  const G = window.SINDRI_DATA.COMMAND_SUGGESTIONS;
  return (
    <div style={v3cb.scrim} onClick={onClose}>
      <div style={v3cb.panel} onClick={(e) => e.stopPropagation()}>
        <div style={v3cb.head}>
          <IconSparkle size={18} stroke={V3.ORANGE}/>
          <div style={v3cb.prompt}>
            {prompt || <span style={{ color: V3.INK4 }}>What should we build…</span>}
            <span style={v3cb.caret}/>
          </div>
          <V2Key>esc</V2Key>
        </div>

        <div style={v3cb.ctxbar}>
          <span style={{ fontSize: 11.5, color: V3.INK4 }}>Context:</span>
          {ctx.map((c, i) => (
            <span key={i} style={v3cb.ctxchip(c.kind)}>
              {c.kind === 'entity' && <span style={{ width: 5, height: 5, background: V3.NAVY }}/>}
              <span>{c.text}</span>
              <IconX size={9} stroke="currentColor"/>
            </span>
          ))}
          <span style={{ fontFamily: V3.F_MONO, fontSize: 11, color: V3.INK4 }}>+ type @ to add</span>
        </div>

        <div style={v3cb.body}>
          {G.map((g, gi) => (
            <div key={gi} style={v3cb.group}>
              <div style={v3cb.groupLabel}>{g.group}</div>
              {g.items.map((it, ii) => (
                <div key={ii} style={v3cb.item(gi === 0 && ii === 2)}>
                  <span style={v3cb.glyph(it.glyph)}>{it.glyph}</span>
                  <span style={{ flex: 1, overflow: 'hidden', textOverflow: 'ellipsis' }}>{it.label}</span>
                  {it.hint && <span style={v3cb.hint}>{it.hint}</span>}
                </div>
              ))}
            </div>
          ))}
        </div>

        <div style={v3cb.foot}>
          <div style={{ display: 'flex', gap: 16 }}>
            <span><V2Key>↑↓</V2Key> navigate</span>
            <span><V2Key>↵</V2Key> apply</span>
            <span><V2Key>⌘↵</V2Key> preview diff</span>
          </div>
          <div style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
            <span style={{ width: 6, height: 6, background: V3.MOSS }}/>
            <span>qwen3:14b · local</span>
          </div>
        </div>
      </div>
    </div>
  );
}

const V3_CB_S = document.createElement('style');
V3_CB_S.innerHTML = '@keyframes v3caret { 50% { opacity: 0; } }';
document.head.appendChild(V3_CB_S);

Object.assign(window, { V2CommandBar });
