// Viewport — the scene rendered as a paper schematic.
// Editorial: hairline grid, axis labels, entity bounds as outlined boxes with handwritten-feel labels.

const vpStyles = {
  root: { position: 'relative', height: '100%', background: 'var(--paper)', overflow: 'hidden' },
  topbar: {
    display: 'flex', alignItems: 'center',
    height: 36,
    borderBottom: '1px solid var(--rule)',
    padding: '0 16px',
    gap: 14,
    background: 'var(--paper)',
    position: 'relative',
    zIndex: 2,
  },
  tab: (active) => ({
    fontSize: 10.5, letterSpacing: '0.16em', textTransform: 'uppercase',
    paddingBottom: 8, paddingTop: 8,
    color: active ? 'var(--ink)' : 'var(--ink-3)',
    cursor: 'pointer',
    borderBottom: active ? '2px solid var(--ink)' : '2px solid transparent',
    marginBottom: -1,
    fontWeight: 500,
  }),
  meta: { marginLeft: 'auto', display: 'flex', gap: 16, alignItems: 'center', color: 'var(--ink-3)', fontFamily: 'var(--f-mono)', fontSize: 10.5 },
  stage: { position: 'absolute', inset: '36px 0 0 0', overflow: 'hidden' },
  grid: { position: 'absolute', inset: 0, pointerEvents: 'none' },
  axisLabel: {
    position: 'absolute', fontFamily: 'var(--f-mono)', fontSize: 9, color: 'var(--ink-4)',
  },
  cameraFrame: {
    position: 'absolute',
    border: '1px solid var(--navy)',
    background: 'rgba(43, 74, 111, 0.04)',
    pointerEvents: 'none',
  },
  cameraLabel: {
    position: 'absolute', top: -22, left: 0,
    fontFamily: 'var(--f-mono)', fontSize: 10, color: 'var(--navy)',
    letterSpacing: '0.04em',
  },
  cornerTick: (which) => {
    const s = { position: 'absolute', width: 14, height: 14, borderColor: 'var(--navy)', borderStyle: 'solid', borderWidth: 0 };
    if (which.includes('t')) s.top = -1; if (which.includes('b')) s.bottom = -1;
    if (which.includes('l')) s.left = -1; if (which.includes('r')) s.right = -1;
    if (which.includes('t')) s.borderTopWidth = 2; if (which.includes('b')) s.borderBottomWidth = 2;
    if (which.includes('l')) s.borderLeftWidth = 2; if (which.includes('r')) s.borderRightWidth = 2;
    return s;
  },
  entityBox: (color, selected, ghost) => ({
    position: 'absolute',
    border: ghost
      ? `1.5px dashed var(--orange)`
      : `1.5px solid ${color === 'orange' ? 'var(--orange)' : color === 'navy' ? 'var(--navy)' : 'var(--ink-2)'}`,
    background: ghost ? 'rgba(212,84,30,0.06)' :
      color === 'orange' ? 'rgba(212,84,30,0.08)' :
      color === 'navy' ? 'rgba(43,74,111,0.10)' :
      'rgba(26,26,26,0.06)',
    cursor: 'pointer',
    transformOrigin: 'center',
    boxShadow: selected ? 'inset 0 0 0 2px var(--ink)' : 'none',
  }),
  entityLabel: (color) => ({
    position: 'absolute',
    top: -22, left: 0,
    fontFamily: 'var(--f-mono)',
    fontSize: 10,
    color: color === 'orange' ? 'var(--orange)' : color === 'navy' ? 'var(--navy)' : 'var(--ink-2)',
    whiteSpace: 'nowrap',
    letterSpacing: '0.02em',
  }),
  selectionHandles: (selected) => ({
    position: 'absolute', inset: -4,
    pointerEvents: 'none',
    display: selected ? 'block' : 'none',
  }),
  bottomMeta: {
    position: 'absolute',
    right: 12, bottom: 8,
    display: 'flex', gap: 14,
    fontFamily: 'var(--f-mono)', fontSize: 10, color: 'var(--ink-3)',
  },
  topRightOverlay: {
    position: 'absolute', top: 48, right: 16,
    display: 'flex', flexDirection: 'column', gap: 8,
    alignItems: 'flex-end',
    pointerEvents: 'none',
  },
  pill: {
    fontFamily: 'var(--f-mono)', fontSize: 10,
    color: 'var(--ink-3)',
    background: 'var(--paper)',
    border: '1px solid var(--rule)',
    padding: '3px 8px',
    letterSpacing: '0.02em',
  },
};

function Viewport({ entities, selectedId, onSelect, ghostEntities = [], viewTab = 'scene', onViewTabChange, hoverProposal, isComposing }) {
  return (
    <div style={vpStyles.root}>
      <div style={vpStyles.topbar}>
        <div style={vpStyles.tab(viewTab === 'scene')} onClick={() => onViewTabChange && onViewTabChange('scene')}>Scene</div>
        <div style={vpStyles.tab(viewTab === 'game')}  onClick={() => onViewTabChange && onViewTabChange('game')}>Game</div>
        <div style={vpStyles.tab(viewTab === 'split')} onClick={() => onViewTabChange && onViewTabChange('split')}>Split</div>
        <div style={vpStyles.meta}>
          <span>1280 × 720</span>
          <span style={{ color: 'var(--ink-4)' }}>·</span>
          <span>0.47×</span>
        </div>
      </div>
      <div style={vpStyles.stage}>
        <Grid />

        {/* Camera frame */}
        <div style={{ ...vpStyles.cameraFrame, left: 220, top: 180, width: 600, height: 340 }}>
          <div style={vpStyles.cameraLabel}>Main Camera · <span style={{ fontStyle: 'italic', fontFamily: 'var(--f-display)', fontSize: 13 }}>active</span></div>
          <div style={vpStyles.cornerTick('tl')} />
          <div style={vpStyles.cornerTick('tr')} />
          <div style={vpStyles.cornerTick('bl')} />
          <div style={vpStyles.cornerTick('br')} />
        </div>

        {/* Entities (skip camera which is the frame itself) */}
        {entities.filter(e => e.pos).map((e) => {
          const sel = selectedId === e.id;
          return (
            <div
              key={e.id}
              style={{
                ...vpStyles.entityBox(e.color, sel, false),
                left: e.pos.x, top: e.pos.y, width: e.pos.w, height: e.pos.h,
                transform: `rotate(${e.pos.rot || 0}deg)`,
              }}
              onClick={() => onSelect(e.id)}
            >
              <div style={vpStyles.entityLabel(e.color)}>
                {e.name}
                <span style={{ color: 'var(--ink-4)', marginLeft: 6 }}>·</span>
                <span style={{ color: 'var(--ink-3)', marginLeft: 6 }}>{Math.round(e.pos.w)}×{Math.round(e.pos.h)}</span>
              </div>
              <EntityGlyph kind={e.kind} color={e.color} />
            </div>
          );
        })}

        {/* Ghost entities — AI proposals */}
        {ghostEntities.map((e, i) => (
          <div
            key={'ghost-vp-' + i}
            style={{
              ...vpStyles.entityBox('orange', false, true),
              left: e.x, top: e.y, width: e.w, height: e.h,
              transform: `rotate(${e.rot || 0}deg)`,
            }}
          >
            <div style={{ ...vpStyles.entityLabel('orange'), fontStyle: 'italic', fontFamily: 'var(--f-display)', fontSize: 13 }}>
              {e.name} <span style={{ color: 'var(--ink-3)', fontSize: 9.5, letterSpacing: '0.14em', textTransform: 'uppercase', marginLeft: 4, fontStyle: 'normal', fontFamily: 'var(--f-sans)' }}>proposed</span>
            </div>
          </div>
        ))}

        {/* hover proposal connection — drone -> beacon arrow */}
        {hoverProposal && (
          <svg style={{ position: 'absolute', inset: 0, pointerEvents: 'none' }}>
            <defs>
              <marker id="arr" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto">
                <path d="M0,0 L10,5 L0,10 z" fill="var(--orange)" />
              </marker>
            </defs>
            <path d="M706,341 Q620,260 576,356" stroke="var(--orange)" strokeWidth="1.4" fill="none" strokeDasharray="4 3" markerEnd="url(#arr)" />
            <text x="640" y="290" fill="var(--orange)" fontFamily="var(--f-display)" fontStyle="italic" fontSize="14">chases →</text>
          </svg>
        )}

        {/* Scene composing overlay — wash the viewport with a thinking state */}
        {isComposing && <ComposingWash />}

        <div style={vpStyles.topRightOverlay}>
          <div style={vpStyles.pill}>grid · 32px</div>
          <div style={vpStyles.pill}>snap · on</div>
        </div>

        <div style={vpStyles.bottomMeta}>
          <span>x 524</span>
          <span>y 312</span>
          <span style={{ color: 'var(--ink-4)' }}>·</span>
          <span>0.47×</span>
        </div>
      </div>
    </div>
  );
}

function Grid() {
  // hairline cross-hatch
  return (
    <svg style={vpStyles.grid} preserveAspectRatio="none" width="100%" height="100%">
      <defs>
        <pattern id="g32" width="32" height="32" patternUnits="userSpaceOnUse">
          <path d="M 32 0 L 0 0 0 32" stroke="var(--ink)" strokeOpacity="0.05" strokeWidth="0.8" fill="none" />
        </pattern>
        <pattern id="g160" width="160" height="160" patternUnits="userSpaceOnUse">
          <path d="M 160 0 L 0 0 0 160" stroke="var(--ink)" strokeOpacity="0.10" strokeWidth="0.8" fill="none" />
        </pattern>
      </defs>
      <rect width="100%" height="100%" fill="url(#g32)" />
      <rect width="100%" height="100%" fill="url(#g160)" />
      {/* origin crosshair */}
      <g transform="translate(520, 360)" stroke="var(--ink)" strokeOpacity="0.4" strokeWidth="0.8">
        <line x1="-8" y1="0" x2="8" y2="0" />
        <line x1="0" y1="-8" x2="0" y2="8" />
      </g>
      <text x="528" y="358" fill="var(--ink-4)" fontFamily="var(--f-mono)" fontSize="9">0,0</text>
    </svg>
  );
}

function EntityGlyph({ kind, color }) {
  // a small representational glyph at the center
  const stroke = color === 'orange' ? 'var(--orange)' : color === 'navy' ? 'var(--navy)' : 'var(--ink-2)';
  return (
    <div style={{
      position: 'absolute',
      inset: 0,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
    }}>
      {kind === 'actor' && (
        <svg width="22" height="22" viewBox="0 0 22 22" fill="none" stroke={stroke} strokeWidth="1.4">
          <path d="M11 3 L19 11 L11 19 L3 11 Z"/>
          <circle cx="11" cy="11" r="2.6"/>
        </svg>
      )}
      {kind === 'static' && (
        <svg width="40" height="6" viewBox="0 0 40 6" fill="none" stroke={stroke} strokeWidth="1.4">
          <line x1="0" y1="3" x2="40" y2="3"/>
        </svg>
      )}
    </div>
  );
}

function ComposingWash() {
  return (
    <div style={{
      position: 'absolute', inset: 0,
      background: 'rgba(246,244,239,0.85)',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      flexDirection: 'column', gap: 14,
      zIndex: 5,
    }}>
      <div style={{
        fontFamily: 'var(--f-display)',
        fontStyle: 'italic',
        fontSize: 28,
        color: 'var(--ink)',
        maxWidth: 540,
        textAlign: 'center',
        lineHeight: 1.25,
      }}>
        Composing your scene <span style={{ color: 'var(--orange)' }}>·</span> sketching entities and scripts <span style={{ color: 'var(--orange)' }}>·</span> wiring them together
      </div>
      <div style={{
        display: 'flex', gap: 8,
        fontFamily: 'var(--f-mono)', fontSize: 11,
        color: 'var(--ink-3)', letterSpacing: '0.04em',
      }}>
        <ThinkingDot />
        <ThinkingDot delay={140}/>
        <ThinkingDot delay={280}/>
        <span style={{ marginLeft: 10 }}>3 entities · 2 scripts</span>
      </div>
    </div>
  );
}

function ThinkingDot({ delay = 0 }) {
  return (
    <span style={{
      width: 6, height: 6,
      background: 'var(--orange)',
      display: 'inline-block',
      animation: `dot 900ms ${delay}ms ease-in-out infinite`,
    }}/>
  );
}

const VP_STYLE = document.createElement('style');
VP_STYLE.innerHTML = `
  @keyframes dot { 0%, 100% { opacity: 0.25; transform: translateY(0); } 50% { opacity: 1; transform: translateY(-3px); } }
`;
document.head.appendChild(VP_STYLE);

Object.assign(window, { Viewport });
