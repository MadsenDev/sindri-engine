// AI flows: Proposals lane (diff review) and Scene Composer overlay

// ── Proposals lane: right-side replacement for the inspector when AI has staged changes.
const plStyles = {
  root: { display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden', fontFamily: 'var(--f-sans)' },
  hdr: {
    padding: '14px 18px 14px',
    borderBottom: '1px solid var(--rule)',
    background: 'var(--paper-2)',
  },
  hdrLabel: { fontSize: 10, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--orange)', fontWeight: 500, display: 'flex', alignItems: 'center', gap: 6 },
  hdrPrompt: {
    fontFamily: 'var(--f-display)', fontSize: 21, lineHeight: 1.2, color: 'var(--ink)',
    marginTop: 6,
  },
  hdrMeta: {
    marginTop: 8, fontSize: 11, color: 'var(--ink-3)',
  },
  hdrActions: {
    marginTop: 12, display: 'flex', gap: 8,
  },
  btn: (variant) => ({
    fontFamily: 'var(--f-sans)',
    fontSize: 10, letterSpacing: '0.14em', textTransform: 'uppercase', fontWeight: 500,
    padding: '7px 12px',
    border: variant === 'primary' ? '1px solid var(--ink)' : variant === 'danger' ? '1px solid var(--ink-3)' : '1px solid var(--ink-3)',
    background: variant === 'primary' ? 'var(--ink)' : 'transparent',
    color: variant === 'primary' ? 'var(--paper)' : 'var(--ink-2)',
    cursor: 'pointer',
    display: 'inline-flex', alignItems: 'center', gap: 6,
  }),
  scroll: { flex: 1, overflowY: 'auto' },
  change: { borderBottom: '1px solid var(--paper-3)' },
  changeHead: {
    padding: '14px 18px 6px',
    display: 'flex', alignItems: 'flex-start', gap: 10,
  },
  changeBody: { padding: '0 18px 14px' },
  changeKind: (kind) => ({
    fontSize: 9.5, letterSpacing: '0.16em', textTransform: 'uppercase', fontWeight: 500,
    color: kind === 'add-component' ? 'var(--moss)' :
      kind === 'modify-script' ? 'var(--navy)' :
      kind === 'add-asset' ? 'var(--orange)' : 'var(--ink-3)',
    display: 'inline-flex', alignItems: 'center', gap: 6,
    border: '1px solid currentColor',
    padding: '2px 6px',
  }),
  changeTitle: {
    fontFamily: 'var(--f-display)', fontSize: 16, color: 'var(--ink)', lineHeight: 1.25,
  },
  changeMeta: {
    fontFamily: 'var(--f-mono)', fontSize: 10.5, color: 'var(--ink-3)', marginTop: 4,
  },
  miniBtn: (accepted, rejected) => ({
    fontFamily: 'var(--f-sans)', fontSize: 9.5, letterSpacing: '0.14em', textTransform: 'uppercase',
    fontWeight: 500,
    padding: '4px 8px',
    border: '1px solid ' + (accepted ? 'var(--moss)' : rejected ? 'var(--ink-4)' : 'var(--ink-3)'),
    color: accepted ? 'var(--moss)' : rejected ? 'var(--ink-4)' : 'var(--ink-2)',
    background: 'transparent',
    cursor: 'pointer',
  }),
  diffBlock: {
    marginTop: 8,
    fontFamily: 'var(--f-mono)', fontSize: 11, lineHeight: 1.5,
    border: '1px solid var(--paper-3)',
    background: 'var(--paper)',
  },
  diffLine: (kind) => ({
    padding: '1px 12px 1px 8px',
    background: kind === 'add' ? 'rgba(94,107,58,0.08)'
      : kind === 'del' ? 'rgba(212,84,30,0.06)'
      : 'transparent',
    borderLeft: '2px solid ' + (kind === 'add' ? 'var(--moss)' : kind === 'del' ? 'var(--orange)' : 'transparent'),
    color: kind === 'del' ? 'var(--ink-3)' : 'var(--ink)',
    whiteSpace: 'pre',
  }),
  detailsKV: {
    marginTop: 6, fontFamily: 'var(--f-mono)', fontSize: 11,
    display: 'grid', gridTemplateColumns: '70px 1fr', rowGap: 3, columnGap: 10,
  },
  asideHead: {
    padding: '10px 18px',
    borderBottom: '1px solid var(--rule)',
    background: 'var(--paper)',
    display: 'flex', alignItems: 'center', justifyContent: 'space-between',
  },
};

function ProposalsLane({ diff, statuses, onSet, onAcceptAll, onRejectAll, onHover }) {
  const accCount = Object.values(statuses).filter(s => s === 'accept').length;
  const rejCount = Object.values(statuses).filter(s => s === 'reject').length;

  return (
    <div style={plStyles.root}>
      <div style={plStyles.hdr}>
        <div style={plStyles.hdrLabel}>
          <IconSparkle size={11} stroke="var(--orange)"/> AI proposal · awaiting review
        </div>
        <div style={plStyles.hdrPrompt}>"{diff.prompt}"</div>
        <div style={plStyles.hdrMeta}>{diff.summary}</div>
        <div style={plStyles.hdrActions}>
          <button style={plStyles.btn('primary')} onClick={onAcceptAll}>
            <IconCheck size={11} stroke="var(--paper)"/> Accept all
          </button>
          <button style={plStyles.btn()} onClick={onRejectAll}>Reject all</button>
          <button style={{ marginLeft: 'auto', ...plStyles.btn() }}>
            <IconChat size={11} stroke="var(--ink-2)"/> Ask follow-up
          </button>
        </div>
        <div style={{ marginTop: 10, fontFamily: 'var(--f-mono)', fontSize: 10.5, color: 'var(--ink-3)' }}>
          {accCount} accepted · {rejCount} rejected · {diff.changes.length - accCount - rejCount} pending
        </div>
      </div>

      <div style={plStyles.scroll}>
        {diff.changes.map((ch) => (
          <div key={ch.id} style={plStyles.change}
               onMouseEnter={() => onHover && onHover(ch.id)}
               onMouseLeave={() => onHover && onHover(null)}>
            <div style={plStyles.changeHead}>
              <span style={plStyles.changeKind(ch.kind)}>
                {ch.kind === 'add-component' ? '+ component' :
                 ch.kind === 'modify-script' ? '↻ script' :
                 ch.kind === 'add-asset' ? '✦ asset' : ch.kind}
              </span>
              <div style={{ flex: 1 }}>
                <div style={plStyles.changeTitle}>{ch.title}</div>
                <div style={plStyles.changeMeta}>
                  on <span style={{ color: 'var(--ink-2)' }}>{ch.entity}</span>
                  {ch.file && <React.Fragment><span style={{ color: 'var(--ink-4)' }}> · </span><span>{ch.file}</span></React.Fragment>}
                </div>
              </div>
            </div>
            <div style={plStyles.changeBody}>
              {ch.kind === 'modify-script' && (
                <div style={plStyles.diffBlock}>
                  {ch.removed.map((r, i) => (
                    <div key={'r' + i} style={plStyles.diffLine('del')}>
                      <span style={{ color: 'var(--orange)', marginRight: 8 }}>−</span>{r.t || ' '}
                    </div>
                  ))}
                  {ch.added.map((a, i) => (
                    <div key={'a' + i} style={plStyles.diffLine('add')}>
                      <span style={{ color: 'var(--moss)', marginRight: 8 }}>+</span>{a.t || ' '}
                    </div>
                  ))}
                </div>
              )}
              {ch.kind !== 'modify-script' && ch.details && (
                <div style={plStyles.detailsKV}>
                  {ch.details.map((d, i) => (
                    <React.Fragment key={i}>
                      <span style={{ color: 'var(--ink-3)' }}>{d.k}</span>
                      <span>{d.v}</span>
                    </React.Fragment>
                  ))}
                </div>
              )}
              <div style={{ marginTop: 10, display: 'flex', gap: 6 }}>
                <button
                  style={plStyles.miniBtn(statuses[ch.id] === 'accept')}
                  onClick={() => onSet(ch.id, statuses[ch.id] === 'accept' ? null : 'accept')}
                >
                  {statuses[ch.id] === 'accept' ? '✓ accepted' : 'accept'}
                </button>
                <button
                  style={plStyles.miniBtn(false, statuses[ch.id] === 'reject')}
                  onClick={() => onSet(ch.id, statuses[ch.id] === 'reject' ? null : 'reject')}
                >
                  {statuses[ch.id] === 'reject' ? '✕ rejected' : 'reject'}
                </button>
                <button style={plStyles.miniBtn()}>refine…</button>
              </div>
            </div>
          </div>
        ))}
        <div style={{ padding: 18, fontFamily: 'var(--f-display)', fontStyle: 'italic', color: 'var(--ink-3)', fontSize: 14 }}>
          The model can keep working on this — accept what you like, ask for changes, or start over.
        </div>
      </div>
    </div>
  );
}

// ── Scene composer modal — full overlay
const scStyles = {
  scrim: {
    position: 'absolute', inset: 0,
    background: 'rgba(10,10,10,0.5)',
    backdropFilter: 'blur(3px)',
    zIndex: 60,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
  },
  panel: {
    width: 1080,
    height: 640,
    background: 'var(--paper)',
    border: '1px solid var(--ink)',
    boxShadow: '12px 12px 0 var(--ink)',
    display: 'grid',
    gridTemplateColumns: '420px 1fr',
    overflow: 'hidden',
  },
  left: {
    padding: '28px 28px 20px',
    borderRight: '1px solid var(--rule)',
    display: 'flex',
    flexDirection: 'column',
  },
  topLabel: { fontSize: 10, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--orange)', fontWeight: 500 },
  title: { fontFamily: 'var(--f-display)', fontSize: 40, lineHeight: 1.05, marginTop: 8, color: 'var(--ink)' },
  subtitle: { color: 'var(--ink-3)', fontSize: 12.5, lineHeight: 1.5, marginTop: 10 },
  promptBox: {
    marginTop: 18,
    border: '1px solid var(--ink)',
    padding: '14px 14px',
    fontFamily: 'var(--f-display)',
    fontStyle: 'italic',
    fontSize: 18,
    lineHeight: 1.35,
    color: 'var(--ink)',
    background: 'var(--paper-2)',
    flex: 1,
  },
  presetLabel: {
    marginTop: 14,
    fontSize: 9.5, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--ink-3)', fontWeight: 500,
  },
  presets: {
    marginTop: 8, display: 'flex', flexWrap: 'wrap', gap: 6,
  },
  preset: {
    fontFamily: 'var(--f-mono)', fontSize: 10.5,
    padding: '5px 9px', border: '1px solid var(--ink-3)', color: 'var(--ink-2)',
    cursor: 'pointer',
  },
  actions: { marginTop: 16, display: 'flex', gap: 8 },
  right: {
    padding: '20px 28px 20px',
    display: 'flex', flexDirection: 'column',
    overflow: 'hidden',
  },
  rightHead: {
    display: 'flex', alignItems: 'center', justifyContent: 'space-between',
  },
  preview: {
    marginTop: 14,
    flex: 1,
    border: '1px solid var(--rule)',
    background: 'var(--paper-2)',
    position: 'relative',
    overflow: 'hidden',
  },
  ents: {
    marginTop: 14,
    fontSize: 12.5,
  },
  entRow: {
    display: 'grid', gridTemplateColumns: '20px 130px 1fr 60px',
    padding: '8px 4px',
    borderBottom: '1px solid var(--paper-3)',
    alignItems: 'baseline',
    fontFamily: 'var(--f-mono)', fontSize: 11,
  },
};

function SceneComposer({ open, onClose, onCompose }) {
  if (!open) return null;
  const SC = window.SINDRI_DATA.COMPOSED_SCENE;
  return (
    <div style={scStyles.scrim} onClick={onClose}>
      <div style={scStyles.panel} onClick={(e) => e.stopPropagation()}>
        <div style={scStyles.left}>
          <div style={scStyles.topLabel}>✦ Compose a scene</div>
          <div style={scStyles.title}>Describe a scene. <span style={{ fontStyle: 'italic' }}>Sindri builds it.</span></div>
          <div style={scStyles.subtitle}>
            The assistant drafts entities, components, and starter scripts. Nothing is committed — you'll see a full diff before anything lands in the project.
          </div>
          <div style={scStyles.promptBox}>
            "{SC.prompt}"<span style={cbStyles.caret}/>
          </div>
          <div style={scStyles.presetLabel}>Or start from a preset</div>
          <div style={scStyles.presets}>
            <span style={scStyles.preset}>top-down arena</span>
            <span style={scStyles.preset}>platformer level</span>
            <span style={scStyles.preset}>dialogue scene</span>
            <span style={scStyles.preset}>menu</span>
            <span style={scStyles.preset}>puzzle room</span>
          </div>
          <div style={scStyles.actions}>
            <button style={plStyles.btn('primary')} onClick={() => onCompose && onCompose()}>
              <IconSparkle size={11} stroke="var(--paper)"/> Compose
            </button>
            <button style={plStyles.btn()} onClick={onClose}>Cancel</button>
          </div>
        </div>

        <div style={scStyles.right}>
          <div style={scStyles.rightHead}>
            <div>
              <div style={scStyles.topLabel}>Preview</div>
              <div style={{ fontFamily: 'var(--f-display)', fontSize: 22, marginTop: 4 }}>
                <span style={{ fontStyle: 'italic' }}>What you'll get.</span>
              </div>
            </div>
            <div style={{ fontFamily: 'var(--f-mono)', fontSize: 10.5, color: 'var(--ink-3)' }}>
              ~ 6 entities · 3 scripts · 12 tiles
            </div>
          </div>

          <div style={scStyles.preview}>
            <ComposePreview />
          </div>

          <div style={scStyles.ents}>
            {SC.entities.map((e, i) => (
              <div key={i} style={scStyles.entRow}>
                <span style={{
                  width: 6, height: 6,
                  background: e.kind === 'actor' ? 'var(--orange)' : e.kind === 'camera' ? 'var(--navy)' : e.kind === 'light' ? 'var(--moss)' : 'var(--ink-3)',
                  display: 'inline-block',
                }}/>
                <span style={{ color: 'var(--ink)' }}>{e.name}{e.note && <span style={{ color: 'var(--ink-3)', marginLeft: 6 }}>{e.note}</span>}</span>
                <span style={{ color: 'var(--ink-3)' }}>{e.components.join(' · ')}</span>
                <span style={{ color: 'var(--ink-4)', textAlign: 'right' }}>{e.kind}</span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

function ComposePreview() {
  // A small editorial scene sketch
  return (
    <svg viewBox="0 0 600 280" width="100%" height="100%" preserveAspectRatio="xMidYMid meet">
      <defs>
        <pattern id="cp-g" width="20" height="20" patternUnits="userSpaceOnUse">
          <path d="M 20 0 L 0 0 0 20" stroke="var(--ink)" strokeOpacity="0.06" strokeWidth="0.6" fill="none"/>
        </pattern>
      </defs>
      <rect width="600" height="280" fill="url(#cp-g)" />
      {/* trees line */}
      {[40, 100, 160, 220, 360, 420, 480, 540].map((x, i) => (
        <g key={i} transform={`translate(${x}, 70)`}>
          <path d="M0 60 L10 0 L20 60 Z" fill="none" stroke="var(--moss)" strokeWidth="1.4"/>
          <line x1="10" y1="60" x2="10" y2="72" stroke="var(--moss)" strokeWidth="1.4"/>
        </g>
      ))}
      {/* Player */}
      <g transform="translate(280, 165)">
        <rect x="-12" y="-12" width="24" height="24" fill="rgba(212,84,30,0.1)" stroke="var(--orange)" strokeWidth="1.4" strokeDasharray="3 2"/>
        <text x="-22" y="-18" fontFamily="var(--f-mono)" fontSize="10" fill="var(--orange)">Player</text>
        <path d="M-4 -4 L0 -8 L4 -4 L4 4 L-4 4 Z" fill="none" stroke="var(--orange)" strokeWidth="1.2"/>
      </g>
      {/* Wolf */}
      <g transform="translate(450, 200)">
        <rect x="-16" y="-10" width="32" height="20" fill="rgba(43,74,111,0.1)" stroke="var(--navy)" strokeWidth="1.4" strokeDasharray="3 2"/>
        <text x="-22" y="-14" fontFamily="var(--f-mono)" fontSize="10" fill="var(--navy)">Wolf</text>
        <path d="M-12 0 Q -8 -6 -4 0 Q 0 -6 4 0 Q 8 -6 12 0" fill="none" stroke="var(--navy)" strokeWidth="1.2"/>
      </g>
      {/* Mushrooms */}
      {[ [150, 200], [320, 215], [380, 175] ].map(([x, y], i) => (
        <g key={i} transform={`translate(${x}, ${y})`}>
          <rect x="-7" y="-7" width="14" height="14" fill="rgba(94,107,58,0.12)" stroke="var(--moss)" strokeWidth="1.2" strokeDasharray="2 2"/>
          {i === 0 && <text x="-12" y="-10" fontFamily="var(--f-mono)" fontSize="9" fill="var(--moss)">Mushroom ×3</text>}
          <circle cx="0" cy="-1" r="3" fill="none" stroke="var(--moss)" strokeWidth="1"/>
          <line x1="0" y1="2" x2="0" y2="5" stroke="var(--moss)" strokeWidth="1"/>
        </g>
      ))}
      {/* Camera frame */}
      <rect x="60" y="60" width="480" height="180" stroke="var(--navy)" strokeWidth="1" fill="none" strokeDasharray="2 3"/>
      <text x="64" y="56" fontFamily="var(--f-mono)" fontSize="9" fill="var(--navy)">Main Camera</text>
      {/* sun marker */}
      <g transform="translate(500, 40)">
        <circle r="5" fill="var(--orange)"/>
        <text x="10" y="3" fontFamily="var(--f-display)" fontStyle="italic" fontSize="13" fill="var(--orange)">dusk</text>
      </g>
    </svg>
  );
}

Object.assign(window, { ProposalsLane, SceneComposer });
