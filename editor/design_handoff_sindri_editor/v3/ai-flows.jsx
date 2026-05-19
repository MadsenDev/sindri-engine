// v2 AI flows — calmer proposals lane + scene composer.

const v3pl = {
  root: { display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden', fontFamily: V3.F_SANS, fontSize: 13 },
  hdr: { padding: '22px 22px 18px', borderBottom: `1px solid ${V3.RULE}`, background: V3.PAPER },
  hdrLabel: { fontSize: 11.5, color: V3.ORANGE, display: 'flex', alignItems: 'center', gap: 6 },
  hdrPrompt: { fontFamily: V3.F_DISPLAY, fontSize: 22, lineHeight: 1.2, color: V3.INK, marginTop: 8 },
  hdrMeta: { marginTop: 12, fontSize: 12, color: V3.INK3, lineHeight: 1.5 },
  hdrActions: { marginTop: 16, display: 'flex', gap: 8 },
  btn: (variant) => ({
    fontFamily: V3.F_SANS, fontSize: 12, fontWeight: 500,
    padding: '8px 14px',
    border: variant === 'primary' ? `1px solid ${V3.INK}` : `1px solid ${V3.RULE2}`,
    background: variant === 'primary' ? V3.INK : 'transparent',
    color: variant === 'primary' ? V3.PAPER : V3.INK2,
    cursor: 'pointer',
    display: 'inline-flex', alignItems: 'center', gap: 6,
  }),
  countLine: { marginTop: 14, fontFamily: V3.F_MONO, fontSize: 11, color: V3.INK4 },

  scroll: { flex: 1, overflowY: 'auto' },
  change: { borderBottom: `1px solid ${V3.RULE}`, padding: '18px 22px' },
  chHead: { display: 'flex', alignItems: 'flex-start', gap: 12, marginBottom: 10 },
  chKind: (kind) => ({
    fontSize: 11, fontWeight: 500,
    color: kind === 'add-component' ? V3.MOSS
      : kind === 'modify-script' ? V3.NAVY
      : kind === 'add-asset' ? V3.ORANGE : V3.INK3,
    padding: '3px 8px',
    border: `1px solid currentColor`,
    flex: 'none',
  }),
  chTitle: { fontSize: 14, color: V3.INK, lineHeight: 1.3, fontWeight: 500 },
  chMeta: { fontFamily: V3.F_MONO, fontSize: 11, color: V3.INK4, marginTop: 4 },

  diffBlock: { fontFamily: V3.F_MONO, fontSize: 11, lineHeight: 1.55, background: V3.PAPER2, padding: '8px 0' },
  diffLine: (kind) => ({
    padding: '0 14px',
    background: kind === 'add' ? 'rgba(155,176,112,0.15)' : kind === 'del' ? 'rgba(240,192,80,0.13)' : 'transparent',
    color: kind === 'del' ? V3.INK4 : V3.INK,
    whiteSpace: 'pre',
  }),
  kv: { fontFamily: V3.F_MONO, fontSize: 11.5, display: 'grid', gridTemplateColumns: '70px 1fr', rowGap: 4, columnGap: 10 },
  actions: { marginTop: 12, display: 'flex', gap: 6 },
  mini: (state) => ({
    fontFamily: V3.F_SANS, fontSize: 11.5, fontWeight: 500,
    padding: '5px 10px',
    border: `1px solid ${state === 'accept' ? V3.MOSS : state === 'reject' ? V3.INK4 : V3.RULE2}`,
    color: state === 'accept' ? V3.MOSS : state === 'reject' ? V3.INK4 : V3.INK2,
    background: 'transparent', cursor: 'pointer',
  }),
};

function V2ProposalsLane({ diff, statuses, onSet, onAcceptAll, onRejectAll, onHover }) {
  const acc = Object.values(statuses).filter(s => s === 'accept').length;
  const rej = Object.values(statuses).filter(s => s === 'reject').length;
  return (
    <div style={v3pl.root}>
      <div style={v3pl.hdr}>
        <div style={v3pl.hdrLabel}>
          <IconSparkle size={11} stroke={V3.ORANGE}/> AI proposal · awaiting review
        </div>
        <div style={v3pl.hdrPrompt}>"{diff.prompt}"</div>
        <div style={v3pl.hdrMeta}>{diff.summary}</div>
        <div style={v3pl.hdrActions}>
          <button style={v3pl.btn('primary')} onClick={onAcceptAll}>
            <IconCheck size={11} stroke={V3.PAPER}/> Accept all
          </button>
          <button style={v3pl.btn()} onClick={onRejectAll}>Reject all</button>
          <button style={{ ...v3pl.btn(), marginLeft: 'auto' }}>
            <IconChat size={11} stroke={V3.INK2}/> Follow-up
          </button>
        </div>
        <div style={v3pl.countLine}>{acc} accepted · {rej} rejected · {diff.changes.length - acc - rej} pending</div>
      </div>

      <div style={v3pl.scroll}>
        {diff.changes.map((ch) => (
          <div key={ch.id} style={v3pl.change}
               onMouseEnter={() => onHover && onHover(ch.id)}
               onMouseLeave={() => onHover && onHover(null)}>
            <div style={v3pl.chHead}>
              <span style={v3pl.chKind(ch.kind)}>
                {ch.kind === 'add-component' ? 'component' : ch.kind === 'modify-script' ? 'script' : 'asset'}
              </span>
              <div style={{ flex: 1 }}>
                <div style={v3pl.chTitle}>{ch.title}</div>
                <div style={v3pl.chMeta}>
                  {ch.entity}{ch.file ? ` · ${ch.file}` : ''}
                </div>
              </div>
            </div>

            {ch.kind === 'modify-script' && (
              <div style={v3pl.diffBlock}>
                {ch.removed.map((r, i) => (
                  <div key={'r' + i} style={v3pl.diffLine('del')}>
                    <span style={{ color: V3.ORANGE, marginRight: 6 }}>−</span>{r.t || ' '}
                  </div>
                ))}
                {ch.added.map((a, i) => (
                  <div key={'a' + i} style={v3pl.diffLine('add')}>
                    <span style={{ color: V3.MOSS, marginRight: 6 }}>+</span>{a.t || ' '}
                  </div>
                ))}
              </div>
            )}
            {ch.kind !== 'modify-script' && ch.details && (
              <div style={v3pl.kv}>
                {ch.details.map((d, i) => (
                  <React.Fragment key={i}>
                    <span style={{ color: V3.INK4 }}>{d.k}</span>
                    <span style={{ color: V3.INK2 }}>{d.v}</span>
                  </React.Fragment>
                ))}
              </div>
            )}

            <div style={v3pl.actions}>
              <button style={v3pl.mini(statuses[ch.id] === 'accept' ? 'accept' : null)}
                onClick={() => onSet(ch.id, statuses[ch.id] === 'accept' ? null : 'accept')}>
                {statuses[ch.id] === 'accept' ? '✓ accepted' : 'accept'}
              </button>
              <button style={v3pl.mini(statuses[ch.id] === 'reject' ? 'reject' : null)}
                onClick={() => onSet(ch.id, statuses[ch.id] === 'reject' ? null : 'reject')}>
                {statuses[ch.id] === 'reject' ? '✕ rejected' : 'reject'}
              </button>
              <button style={v3pl.mini()}>refine…</button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

// ── Scene composer (v2)
const v3sc = {
  scrim: { position: 'absolute', inset: 0, background: 'rgba(0,0,0,0.65)', zIndex: 60, display: 'flex', alignItems: 'center', justifyContent: 'center' },
  panel: { width: 1080, height: 640, background: V3.PAPER, border: `1px solid ${V3.RULE2}`, boxShadow: '0 30px 80px rgba(0,0,0,0.28)', display: 'grid', gridTemplateColumns: '420px 1fr', overflow: 'hidden' },
  left: { padding: '32px 32px 24px', borderRight: `1px solid ${V3.RULE}`, display: 'flex', flexDirection: 'column' },
  label: { fontSize: 11.5, color: V3.ORANGE },
  title: { fontFamily: V3.F_DISPLAY, fontSize: 42, lineHeight: 1.05, marginTop: 8, color: V3.INK },
  blurb: { color: V3.INK3, fontSize: 12.5, lineHeight: 1.55, marginTop: 12 },
  promptBox: {
    marginTop: 20,
    border: `1px solid ${V3.RULE2}`,
    padding: '16px',
    fontFamily: V3.F_DISPLAY, fontSize: 18, lineHeight: 1.4, color: V3.INK,
    background: V3.PAPER2, flex: 1,
  },
  presetLbl: { marginTop: 14, fontSize: 11.5, color: V3.INK4 },
  presets: { marginTop: 8, display: 'flex', flexWrap: 'wrap', gap: 6 },
  preset: { fontFamily: V3.F_MONO, fontSize: 11, padding: '5px 10px', border: `1px solid ${V3.RULE2}`, color: V3.INK3, cursor: 'pointer' },
  actions: { marginTop: 18, display: 'flex', gap: 8 },

  right: { padding: '24px 32px', display: 'flex', flexDirection: 'column', overflow: 'hidden' },
  rightHead: { display: 'flex', alignItems: 'baseline', justifyContent: 'space-between' },
  preview: { marginTop: 16, flex: 1, border: `1px solid ${V3.RULE}`, background: V3.PAPER, position: 'relative', overflow: 'hidden' },
  ents: { marginTop: 16, fontSize: 12.5 },
  row: { display: 'grid', gridTemplateColumns: '14px 140px 1fr 60px', padding: '9px 4px', borderBottom: `1px solid ${V3.RULE}`, alignItems: 'baseline', fontFamily: V3.F_MONO, fontSize: 11.5 },
};

function V2SceneComposer({ open, onClose, onCompose, isComposing }) {
  if (!open) return null;
  const SC = window.SINDRI_DATA.COMPOSED_SCENE;
  return (
    <div style={v3sc.scrim} onClick={onClose}>
      <div style={v3sc.panel} onClick={(e) => e.stopPropagation()}>
        <div style={v3sc.left}>
          <div style={v3sc.label}>✦ Compose a scene</div>
          <div style={v3sc.title}>Describe it. <span style={{ }}>Sindri builds it.</span></div>
          <div style={v3sc.blurb}>
            The assistant drafts entities, components, and starter scripts. Nothing is committed — you'll see a full diff before anything lands in the project.
          </div>
          <div style={v3sc.promptBox}>"{SC.prompt}"<span style={v3cb.caret}/></div>
          <div style={v3sc.presetLbl}>Or start from a preset</div>
          <div style={v3sc.presets}>
            {['top-down arena', 'platformer level', 'dialogue scene', 'menu', 'puzzle room'].map(p =>
              <span key={p} style={v3sc.preset}>{p}</span>)}
          </div>
          <div style={v3sc.actions}>
            <button style={v3pl.btn('primary')} onClick={() => !isComposing && onCompose && onCompose()} disabled={isComposing}>
              {isComposing ? (
                <React.Fragment>
                  <span style={{ width: 6, height: 6, background: V3.PAPER, animation: 'v3dot 900ms ease-in-out infinite' }}/>
                  Composing…
                </React.Fragment>
              ) : (
                <React.Fragment>
                  <IconSparkle size={11} stroke={V3.PAPER}/> Compose
                </React.Fragment>
              )}
            </button>
            <button style={v3pl.btn()} onClick={onClose}>Cancel</button>
          </div>
        </div>

        <div style={v3sc.right}>
          <div style={v3sc.rightHead}>
            <div>
              <div style={v3sc.label}>Preview</div>
              <div style={{ fontFamily: V3.F_DISPLAY, fontSize: 24, marginTop: 4, }}>What you'll get.</div>
            </div>
            <div style={{ fontFamily: V3.F_MONO, fontSize: 11, color: V3.INK4 }}>6 entities · 3 scripts · 12 tiles</div>
          </div>
          <div style={v3sc.preview}>
            <V2ComposePreview/>
            {isComposing && (
              <div style={{
                position: 'absolute', inset: 0,
                background: 'rgba(13,17,23,0.78)',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                flexDirection: 'column', gap: 14,
              }}>
                <div style={{ fontFamily: V3.F_DISPLAY, fontSize: 26, color: V3.INK }}>
                  Composing<span style={{ color: V3.ORANGE }}>…</span>
                </div>
                <div style={{
                  display: 'flex', gap: 8, alignItems: 'center',
                  fontFamily: V3.F_MONO, fontSize: 11, color: V3.INK3,
                }}>
                  <span style={{ width: 6, height: 6, background: V3.ORANGE, animation: 'v3dot 900ms 0ms ease-in-out infinite' }}/>
                  <span style={{ width: 6, height: 6, background: V3.ORANGE, animation: 'v3dot 900ms 140ms ease-in-out infinite' }}/>
                  <span style={{ width: 6, height: 6, background: V3.ORANGE, animation: 'v3dot 900ms 280ms ease-in-out infinite' }}/>
                  <span style={{ marginLeft: 10 }}>drafting 6 entities · 3 scripts</span>
                </div>
              </div>
            )}
          </div>
          <div style={v3sc.ents}>
            {SC.entities.map((e, i) => (
              <div key={i} style={v3sc.row}>
                <span style={{ width: 7, height: 7, background:
                  e.kind === 'actor' ? V3.ORANGE :
                  e.kind === 'camera' ? V3.NAVY :
                  e.kind === 'light' ? V3.MOSS : V3.INK4 }}/>
                <span style={{ color: V3.INK }}>{e.name}{e.note && <span style={{ color: V3.INK4, marginLeft: 6 }}>{e.note}</span>}</span>
                <span style={{ color: V3.INK3 }}>{e.components.join(' · ')}</span>
                <span style={{ color: V3.INK4, textAlign: 'right' }}>{e.kind}</span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

function V2ComposePreview() {
  return (
    <svg viewBox="0 0 600 280" width="100%" height="100%" preserveAspectRatio="xMidYMid meet">
      <defs>
        <pattern id="v2cp" width="24" height="24" patternUnits="userSpaceOnUse">
          <path d="M 24 0 L 0 0 0 24" stroke={V3.INK} strokeOpacity="0.05" strokeWidth="0.6" fill="none"/>
        </pattern>
      </defs>
      <rect width="600" height="280" fill="url(#v2cp)"/>
      {[40, 100, 160, 220, 360, 420, 480, 540].map((x, i) => (
        <g key={i} transform={`translate(${x}, 70)`}>
          <path d="M0 60 L10 0 L20 60 Z" fill="none" stroke={V3.MOSS} strokeWidth="1.2"/>
          <line x1="10" y1="60" x2="10" y2="72" stroke={V3.MOSS} strokeWidth="1.2"/>
        </g>
      ))}
      <g transform="translate(280, 165)">
        <rect x="-12" y="-12" width="24" height="24" fill="rgba(240,192,80,0.13)" stroke={V3.ORANGE} strokeWidth="1.2" strokeDasharray="3 2"/>
        <text x="-22" y="-18" fontFamily={V3.F_MONO} fontSize="10" fill={V3.ORANGE}>Player</text>
      </g>
      <g transform="translate(450, 200)">
        <rect x="-16" y="-10" width="32" height="20" fill="rgba(109,188,219,0.10)" stroke={V3.NAVY} strokeWidth="1.2" strokeDasharray="3 2"/>
        <text x="-22" y="-14" fontFamily={V3.F_MONO} fontSize="10" fill={V3.NAVY}>Wolf</text>
      </g>
      {[[150, 200], [320, 215], [380, 175]].map(([x, y], i) => (
        <g key={i} transform={`translate(${x}, ${y})`}>
          <rect x="-7" y="-7" width="14" height="14" fill="rgba(155,176,112,0.15)" stroke={V3.MOSS} strokeWidth="1" strokeDasharray="2 2"/>
          {i === 0 && <text x="-12" y="-10" fontFamily={V3.F_MONO} fontSize="9" fill={V3.MOSS}>Mushroom × 3</text>}
        </g>
      ))}
      <rect x="60" y="60" width="480" height="180" stroke={V3.NAVY} strokeWidth="1" fill="none" strokeDasharray="2 3"/>
      <text x="64" y="56" fontFamily={V3.F_MONO} fontSize="10" fill={V3.NAVY}>Main Camera</text>
    </svg>
  );
}

Object.assign(window, { V2ProposalsLane, V2SceneComposer });
