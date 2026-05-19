// v2 Viewport — quieter grid, simpler entity boxes, no inner glyphs.

const v3vp = {
  root: { position: 'relative', height: '100%', background: V3.PAPER, overflow: 'hidden' },
  topbar: {
    display: 'flex', alignItems: 'center',
    height: 42,
    padding: '0 16px',
    borderBottom: `1px solid ${V3.RULE}`,
    gap: 20,
    position: 'relative', zIndex: 2,
    background: V3.PAPER,
  },
  tab: (active) => ({
    paddingTop: 10, paddingBottom: 10,
    fontSize: 12.5,
    color: active ? V3.INK : V3.INK3,
    fontWeight: active ? 500 : 400,
    borderBottom: active ? `2px solid ${V3.INK}` : '2px solid transparent',
    marginBottom: -1,
    cursor: 'pointer',
  }),
  meta: { marginLeft: 'auto', display: 'flex', gap: 14, color: V3.INK3, fontFamily: V3.F_MONO, fontSize: 11 },
  stage: { position: 'absolute', inset: '42px 0 0 0', overflow: 'hidden' },
  grid: { position: 'absolute', inset: 0, pointerEvents: 'none' },

  cameraFrame: (l, t, w, h) => ({
    position: 'absolute',
    left: l, top: t, width: w, height: h,
    border: `1px solid ${V3.NAVY}`,
    background: 'rgba(109,188,219,0.06)',
    pointerEvents: 'none',
  }),
  cameraLabel: {
    position: 'absolute', top: -22, left: 0,
    fontFamily: V3.F_SANS, fontSize: 11, color: V3.NAVY,
  },

  ebox: (color, sel, ghost) => ({
    position: 'absolute',
    border: ghost
      ? `1.5px dashed ${V3.ORANGE}`
      : sel
        ? `2px solid ${V3.INK}`
        : `1px solid ${color === 'orange' ? V3.ORANGE : color === 'navy' ? V3.NAVY : V3.INK3}`,
    background: ghost ? 'rgba(240,192,80,0.10)'
      : color === 'orange' ? 'rgba(240,192,80,0.13)'
      : color === 'navy' ? 'rgba(109,188,219,0.10)'
      : 'rgba(255,255,255,0.04)',
    cursor: 'pointer',
  }),
  elabel: (color, ghost) => ({
    position: 'absolute', top: -20, left: 0,
    fontFamily: V3.F_SANS, fontSize: 11,
    color: ghost ? V3.ORANGE : color === 'orange' ? V3.ORANGE : color === 'navy' ? V3.NAVY : V3.INK2,
    whiteSpace: 'nowrap',
  }),
  bottomMeta: {
    position: 'absolute', right: 14, bottom: 10,
    display: 'flex', gap: 14,
    fontFamily: V3.F_MONO, fontSize: 10.5, color: V3.INK4,
  },
};

function V2Viewport({ entities, selectedId, onSelect, ghostEntities = [], viewTab, onViewTabChange, hoverProposal, isComposing }) {
  return (
    <div style={v3vp.root}>
      <div style={v3vp.topbar}>
        <div style={v3vp.tab(viewTab === 'scene')} onClick={() => onViewTabChange('scene')}>Scene</div>
        <div style={v3vp.tab(viewTab === 'game')}  onClick={() => onViewTabChange('game')}>Game</div>
        <div style={v3vp.tab(viewTab === 'split')} onClick={() => onViewTabChange('split')}>Split</div>
        <div style={v3vp.meta}>
          <span>1280 × 720</span>
          <span style={{ color: V3.INK4 }}>·</span>
          <span>0.47×</span>
        </div>
      </div>

      <div style={v3vp.stage}>
        <V2Grid/>
        <div style={v3vp.cameraFrame(220, 120, 600, 340)}>
          <div style={v3vp.cameraLabel}>Main Camera</div>
        </div>

        {entities.filter(e => e.pos).map((e) => {
          const sel = selectedId === e.id;
          return (
            <div
              key={e.id}
              style={{
                ...v3vp.ebox(e.color, sel, false),
                left: e.pos.x, top: e.pos.y, width: e.pos.w, height: e.pos.h,
                transform: `rotate(${e.pos.rot || 0}deg)`,
              }}
              onClick={() => onSelect(e.id)}
            >
              <div style={v3vp.elabel(e.color)}>{e.name}</div>
            </div>
          );
        })}

        {ghostEntities.map((e, i) => (
          <div key={'g' + i}
            style={{
              ...v3vp.ebox('orange', false, true),
              left: e.x, top: e.y, width: e.w, height: e.h,
              transform: `rotate(${e.rot || 0}deg)`,
            }}>
            <div style={v3vp.elabel('orange', true)}>{e.name}</div>
          </div>
        ))}

        {hoverProposal && (
          <svg style={{ position: 'absolute', inset: 0, pointerEvents: 'none' }}>
            <defs>
              <marker id="arr2" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto">
                <path d="M0,0 L10,5 L0,10 z" fill={V3.ORANGE} />
              </marker>
            </defs>
            <path d="M706,281 Q620,200 576,296" stroke={V3.ORANGE} strokeWidth="1.4" fill="none" strokeDasharray="4 3" markerEnd="url(#arr2)" />
            <text x="640" y="230" fill={V3.ORANGE} fontFamily={V3.F_SANS} fontSize="12">chases →</text>
          </svg>
        )}

        {isComposing && <V2ComposingWash/>}

        <div style={v3vp.bottomMeta}>
          <span>x 524</span><span>y 312</span>
        </div>
      </div>
    </div>
  );
}

function V2Grid() {
  return (
    <svg style={v3vp.grid} preserveAspectRatio="none" width="100%" height="100%">
      <defs>
        <pattern id="vg32" width="40" height="40" patternUnits="userSpaceOnUse">
          <path d="M 40 0 L 0 0 0 40" stroke={V3.INK} strokeOpacity="0.04" strokeWidth="0.6" fill="none"/>
        </pattern>
        <pattern id="vg200" width="200" height="200" patternUnits="userSpaceOnUse">
          <path d="M 200 0 L 0 0 0 200" stroke={V3.INK} strokeOpacity="0.08" strokeWidth="0.6" fill="none"/>
        </pattern>
      </defs>
      <rect width="100%" height="100%" fill="url(#vg32)"/>
      <rect width="100%" height="100%" fill="url(#vg200)"/>
    </svg>
  );
}

function V2ComposingWash() {
  return (
    <div style={{
      position: 'absolute', inset: 0,
      background: 'rgba(13,17,23,0.92)',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      flexDirection: 'column', gap: 18,
      zIndex: 5,
    }}>
      <div style={{
        fontFamily: V3.F_DISPLAY, fontSize: 32, color: V3.INK,
        textAlign: 'center', maxWidth: 580, lineHeight: 1.2,
      }}>
        Composing your scene<span style={{ color: V3.ORANGE }}>…</span>
      </div>
      <div style={{
        display: 'flex', gap: 8, alignItems: 'center',
        fontFamily: V3.F_MONO, fontSize: 11, color: V3.INK3,
      }}>
        <span style={{ width: 6, height: 6, background: V3.ORANGE, animation: 'v3dot 900ms 0ms ease-in-out infinite' }}/>
        <span style={{ width: 6, height: 6, background: V3.ORANGE, animation: 'v3dot 900ms 140ms ease-in-out infinite' }}/>
        <span style={{ width: 6, height: 6, background: V3.ORANGE, animation: 'v3dot 900ms 280ms ease-in-out infinite' }}/>
        <span style={{ marginLeft: 12 }}>drafting 6 entities · 3 scripts</span>
      </div>
    </div>
  );
}

const V3_VP_S = document.createElement('style');
V3_VP_S.innerHTML = `@keyframes v3dot { 0%, 100% { opacity: 0.25; transform: translateY(0); } 50% { opacity: 1; transform: translateY(-3px); } }`;
document.head.appendChild(V3_VP_S);

Object.assign(window, { V2Viewport });
