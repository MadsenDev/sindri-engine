// Inspector — contextual right panel.
// When entity selected: shows components in editorial style with inline values.
// Also surfaces AI-suggested actions for the selection.

const insStyles = {
  root: {
    display: 'flex',
    flexDirection: 'column',
    height: '100%',
    fontSize: 12.5,
    fontFamily: 'var(--f-sans)',
    overflow: 'hidden',
  },
  empty: {
    padding: 24,
    color: 'var(--ink-3)',
    fontStyle: 'italic',
    fontFamily: 'var(--f-display)',
    fontSize: 18,
    lineHeight: 1.35,
  },
  hdr: {
    padding: '18px 18px 14px',
    borderBottom: '1px solid var(--rule)',
  },
  hdrTopline: {
    display: 'flex', alignItems: 'center', gap: 10,
    fontSize: 10, letterSpacing: '0.16em', textTransform: 'uppercase',
    color: 'var(--ink-3)', fontWeight: 500,
  },
  hdrName: {
    fontFamily: 'var(--f-display)', fontSize: 34, lineHeight: 1.05,
    marginTop: 6, color: 'var(--ink)',
  },
  hdrMeta: {
    marginTop: 8,
    display: 'flex', gap: 14, alignItems: 'baseline',
    fontFamily: 'var(--f-mono)', fontSize: 11, color: 'var(--ink-3)',
  },
  scroll: { flex: 1, overflowY: 'auto', overflowX: 'hidden' },
  section: { borderBottom: '1px solid var(--paper-3)', padding: '14px 18px' },
  sectionHead: {
    display: 'flex', alignItems: 'center', justifyContent: 'space-between',
    marginBottom: 10,
  },
  sectionTitle: {
    display: 'flex', alignItems: 'center', gap: 8,
    fontSize: 10.5, letterSpacing: '0.16em', textTransform: 'uppercase',
    color: 'var(--ink)', fontWeight: 500,
  },
  sectionMenu: { color: 'var(--ink-3)', cursor: 'pointer' },
  kv: {
    display: 'grid',
    gridTemplateColumns: '78px 1fr',
    rowGap: 6, columnGap: 12,
    fontFamily: 'var(--f-mono)', fontSize: 11.5,
  },
  k: { color: 'var(--ink-3)' },
  v: { color: 'var(--ink)' },
  vec: { display: 'inline-flex', gap: 10 },
  vecPart: (axis) => ({
    display: 'inline-flex', alignItems: 'baseline', gap: 4,
  }),
  axisChip: {
    fontFamily: 'var(--f-sans)', fontSize: 9, letterSpacing: '0.1em',
    color: 'var(--ink-3)',
  },
  aiBlock: {
    padding: '14px 18px',
    background: 'var(--paper-2)',
    borderBottom: '1px solid var(--rule)',
  },
  aiHead: {
    display: 'flex', alignItems: 'baseline', gap: 6,
    fontFamily: 'var(--f-display)', fontStyle: 'italic',
    fontSize: 18, color: 'var(--ink)',
    flexWrap: 'wrap',
  },
  aiList: { display: 'flex', flexDirection: 'column', gap: 0, marginTop: 8 },
  aiAction: {
    display: 'flex', alignItems: 'center', gap: 10,
    padding: '8px 0',
    borderTop: '1px solid var(--paper-3)',
    fontSize: 12.5,
    color: 'var(--ink-2)',
    cursor: 'pointer',
  },
  aiActionLabel: { flex: 1 },
  addCompRow: {
    marginTop: 4,
    padding: '10px 18px',
    fontFamily: 'var(--f-display)', fontStyle: 'italic',
    fontSize: 14, color: 'var(--ink-3)',
    cursor: 'pointer',
    borderTop: '1px dashed var(--rule)',
    display: 'flex', alignItems: 'center', gap: 8,
  },
};

function Inspector({ entity, onOpenScript, onTriggerAI }) {
  if (!entity) {
    return (
      <div style={insStyles.root}>
        <div style={insStyles.empty}>
          Nothing selected.
          <div style={{ fontStyle: 'normal', fontFamily: 'var(--f-sans)', fontSize: 11, color: 'var(--ink-3)', marginTop: 14, letterSpacing: '0.16em', textTransform: 'uppercase', fontWeight: 500 }}>
            Pick an entity in the scene
          </div>
          <div style={{ marginTop: 18, fontFamily: 'var(--f-sans)', fontStyle: 'normal', fontSize: 12, color: 'var(--ink-3)', lineHeight: 1.5 }}>
            Or press <KeyChip>⌘</KeyChip> <KeyChip>K</KeyChip> to ask the assistant.
          </div>
        </div>
      </div>
    );
  }

  return (
    <div style={insStyles.root}>
      <div style={insStyles.hdr}>
        <div style={insStyles.hdrTopline}>
          <span style={{
            width: 8, height: 8,
            background: entity.color === 'orange' ? 'var(--orange)' :
              entity.color === 'navy' ? 'var(--navy)' : 'var(--ink-3)',
          }}/>
          <span>{entity.kind}</span>
        </div>
        <div style={insStyles.hdrName}>{entity.name}</div>
        <div style={insStyles.hdrMeta}>
          <span>id · {entity.id}</span>
          <span>·</span>
          <span>{entity.components.length} components</span>
        </div>
      </div>

      {/* AI suggestions, top-of-inspector */}
      <div style={insStyles.aiBlock}>
        <div style={insStyles.aiHead}>
          <IconSparkle size={14} stroke="var(--orange)" />
          <span style={{ whiteSpace: 'nowrap' }}>Ask about <span style={{ color: 'var(--orange)' }}>{entity.name}</span></span>
        </div>
        <div style={insStyles.aiList}>
          {entity.id === 'drone' && (
            <React.Fragment>
              <div style={insStyles.aiAction} onClick={() => onTriggerAI && onTriggerAI('chase')}>
                <IconSparkSmall size={11} stroke="var(--orange)"/>
                <span style={insStyles.aiActionLabel}>Make it chase the Beacon</span>
                <IconChevronR size={11} stroke="var(--ink-3)"/>
              </div>
              <div style={insStyles.aiAction}>
                <IconSparkSmall size={11} stroke="var(--orange)"/>
                <span style={insStyles.aiActionLabel}>Refactor as a state machine</span>
                <IconChevronR size={11} stroke="var(--ink-3)"/>
              </div>
            </React.Fragment>
          )}
          {entity.id !== 'drone' && (
            <div style={insStyles.aiAction}>
              <IconSparkSmall size={11} stroke="var(--orange)"/>
              <span style={insStyles.aiActionLabel}>Explain what this entity does</span>
              <IconChevronR size={11} stroke="var(--ink-3)"/>
            </div>
          )}
          <div style={insStyles.aiAction}>
            <IconSparkSmall size={11} stroke="var(--orange)"/>
            <span style={insStyles.aiActionLabel}>Generate sprite variations</span>
            <IconChevronR size={11} stroke="var(--ink-3)"/>
          </div>
        </div>
      </div>

      <div style={insStyles.scroll}>
        {entity.components.map((c, i) => <ComponentBlock key={i} c={c} entityId={entity.id} onOpenScript={onOpenScript}/>)}
        <div style={insStyles.addCompRow}>
          <IconPlus size={11} stroke="var(--ink-3)"/>
          <span>Add component…</span>
        </div>
      </div>
    </div>
  );
}

function ComponentBlock({ c, entityId, onOpenScript }) {
  const Icon = COMPONENT_ICONS[c.type] || IconFile;
  return (
    <div style={insStyles.section}>
      <div style={insStyles.sectionHead}>
        <div style={insStyles.sectionTitle}>
          <Icon size={12} stroke="var(--ink)"/>
          <span>{c.type}</span>
        </div>
        <span style={insStyles.sectionMenu}>···</span>
      </div>
      {c.type === 'Transform' && (
        <div style={insStyles.kv}>
          <span style={insStyles.k}>position</span>
          <span style={insStyles.v}>
            <Vec parts={[['x', c.pos[0]], ['y', c.pos[1]]]}/>
          </span>
          <span style={insStyles.k}>rotation</span>
          <span style={insStyles.v}>{c.rot}°</span>
          <span style={insStyles.k}>scale</span>
          <span style={insStyles.v}>{c.scale.toFixed(2)}×</span>
        </div>
      )}
      {c.type === 'Sprite' && (
        <div style={insStyles.kv}>
          <span style={insStyles.k}>source</span>
          <span style={insStyles.v}>{c.source}</span>
          <span style={insStyles.k}>tint</span>
          <span style={insStyles.v}>
            <span style={{ display: 'inline-flex', gap: 8, alignItems: 'center' }}>
              <span style={{ width: 10, height: 10, background: c.tint, border: '1px solid var(--ink)' }}/>
              <span>{c.tint}</span>
            </span>
          </span>
        </div>
      )}
      {c.type === 'Script' && (
        <div onClick={() => onOpenScript && onOpenScript(c.source)} style={{ cursor: 'pointer' }}>
          <div style={insStyles.kv}>
            <span style={insStyles.k}>source</span>
            <span style={{ ...insStyles.v, color: 'var(--navy)', textDecoration: 'underline', textUnderlineOffset: 2 }}>{c.source}</span>
            <span style={insStyles.k}>lines</span>
            <span style={insStyles.v}>{c.lines}</span>
          </div>
        </div>
      )}
      {c.type === 'Camera' && (
        <div style={insStyles.kv}>
          <span style={insStyles.k}>size</span>
          <span style={insStyles.v}><Vec parts={[['w', c.size[0]], ['h', c.size[1]]]}/></span>
          <span style={insStyles.k}>active</span>
          <span style={{ ...insStyles.v, color: c.active ? 'var(--orange)' : 'var(--ink-3)' }}>{c.active ? 'true' : 'false'}</span>
        </div>
      )}
    </div>
  );
}

function Vec({ parts }) {
  return (
    <span style={{ display: 'inline-flex', gap: 14 }}>
      {parts.map(([axis, val]) => (
        <span key={axis} style={{ display: 'inline-flex', gap: 4, alignItems: 'baseline' }}>
          <span style={{ fontSize: 9, color: 'var(--ink-3)', letterSpacing: '0.1em' }}>{axis}</span>
          <span>{typeof val === 'number' ? val.toFixed(0) : val}</span>
        </span>
      ))}
    </span>
  );
}

function KeyChip({ children }) {
  return (
    <span style={{
      display: 'inline-block',
      padding: '1px 6px',
      border: '1px solid var(--ink-3)',
      fontFamily: 'var(--f-mono)',
      fontSize: 10,
      lineHeight: 1.3,
      color: 'var(--ink-2)',
      margin: '0 1px',
    }}>{children}</span>
  );
}

Object.assign(window, { Inspector, KeyChip });
