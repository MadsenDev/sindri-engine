// v2 Inspector — calm two-column field layout.
// Hero entity name in serif (only display-serif moment).
// AI suggestions tucked under fields as quiet links.

const v3ins = {
  root: { display: 'flex', flexDirection: 'column', height: '100%', fontFamily: V3.F_SANS, fontSize: 13, color: V3.INK, overflow: 'hidden' },
  empty: {
    padding: 28, color: V3.INK3,
    display: 'flex', flexDirection: 'column', gap: 14,
  },
  emptyHero: {
    fontFamily: V3.F_DISPLAY, fontSize: 26, lineHeight: 1.2, color: V3.INK2,
  },
  hdr: { padding: '22px 22px 18px', borderBottom: `1px solid ${V3.RULE}` },
  hdrTopline: { fontSize: 11.5, color: V3.INK3, display: 'flex', alignItems: 'center', gap: 8 },
  hdrName: {
    fontFamily: V3.F_DISPLAY, fontSize: 36, lineHeight: 1.05, marginTop: 4, color: V3.INK,
  },
  hdrMeta: { marginTop: 10, fontFamily: V3.F_MONO, fontSize: 11, color: V3.INK4 },
  scroll: { flex: 1, overflowY: 'auto' },

  section: { borderBottom: `1px solid ${V3.RULE}`, padding: '16px 22px' },
  sectionHead: {
    display: 'flex', alignItems: 'center', justifyContent: 'space-between',
    marginBottom: 10,
  },
  sectionTitle: {
    display: 'flex', alignItems: 'center', gap: 8,
    fontSize: 12.5, color: V3.INK, fontWeight: 500,
  },
  kv: {
    display: 'grid',
    gridTemplateColumns: '78px 1fr',
    rowGap: 8, columnGap: 12,
    fontSize: 12.5,
  },
  k: { color: V3.INK3 },
  v: { color: V3.INK, fontFamily: V3.F_MONO, fontSize: 12, fontVariantNumeric: 'tabular-nums' },

  // AI suggestions row — quiet links at bottom of inspector
  aiBlock: { padding: '16px 22px', borderBottom: `1px solid ${V3.RULE}` },
  aiLabel: { fontSize: 11.5, color: V3.ORANGE, display: 'inline-flex', alignItems: 'center', gap: 6 },
  aiList: { marginTop: 10, display: 'flex', flexDirection: 'column', gap: 4 },
  aiLink: {
    display: 'flex', alignItems: 'center', gap: 8,
    fontSize: 13, color: V3.INK2,
    padding: '6px 0',
    cursor: 'pointer',
    borderBottom: `1px dotted ${V3.PAPER3}`,
  },
  addComp: {
    margin: '8px 22px 14px',
    padding: '10px 12px',
    fontSize: 12.5, color: V3.INK3,
    border: `1px dashed ${V3.RULE2}`,
    cursor: 'pointer',
    display: 'flex', alignItems: 'center', gap: 8,
  },
};

function V2Inspector({ entity, onOpenScript, onTriggerAI }) {
  if (!entity) {
    return (
      <div style={v3ins.root}>
        <div style={v3ins.empty}>
          <div style={v3ins.emptyHero}>Nothing selected.</div>
          <div style={{ fontSize: 12.5, color: V3.INK3, lineHeight: 1.5 }}>
            Pick an entity in the scene to inspect it — or press <V2Key>⌘</V2Key><V2Key>K</V2Key> to ask the assistant.
          </div>
        </div>
      </div>
    );
  }
  return (
    <div style={v3ins.root}>
      <div style={v3ins.hdr}>
        <div style={v3ins.hdrTopline}>
          <span style={{
            width: 8, height: 8,
            background: entity.color === 'orange' ? V3.ORANGE : entity.color === 'navy' ? V3.NAVY : V3.INK3,
          }}/>
          <span>{entity.kind}</span>
        </div>
        <div style={v3ins.hdrName}>{entity.name}</div>
        <div style={v3ins.hdrMeta}>{entity.id} · {entity.components.length} components</div>
      </div>

      <div style={v3ins.scroll}>
        {entity.components.map((c, i) => <V2CompBlock key={i} c={c} onOpenScript={onOpenScript}/>)}
        <div style={v3ins.addComp}>
          <IconPlus size={11} stroke={V3.INK3}/>
          <span style={{ whiteSpace: 'nowrap' }}>Add component</span>
        </div>

        <div style={v3ins.aiBlock}>
          <span style={v3ins.aiLabel}>
            <IconSparkSmall size={11} stroke={V3.ORANGE}/>
            <span style={{ whiteSpace: 'nowrap' }}>Ask about <span style={{ color: V3.INK }}>{entity.name}</span></span>
          </span>
          <div style={v3ins.aiList}>
            {entity.id === 'drone' ? (
              <React.Fragment>
                <div style={v3ins.aiLink} onClick={() => onTriggerAI && onTriggerAI('chase')}>
                  <span style={{ flex: 1 }}>Make it chase the Beacon</span>
                  <IconChevronR size={10} stroke={V3.INK4}/>
                </div>
                <div style={v3ins.aiLink}>
                  <span style={{ flex: 1 }}>Refactor as a state machine</span>
                  <IconChevronR size={10} stroke={V3.INK4}/>
                </div>
              </React.Fragment>
            ) : (
              <div style={v3ins.aiLink}>
                <span style={{ flex: 1 }}>Explain what this entity does</span>
                <IconChevronR size={10} stroke={V3.INK4}/>
              </div>
            )}
            <div style={{ ...v3ins.aiLink, borderBottom: 'none' }}>
              <span style={{ flex: 1 }}>Generate sprite variations</span>
              <IconChevronR size={10} stroke={V3.INK4}/>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

function V2CompBlock({ c, onOpenScript }) {
  const Ico = COMPONENT_ICONS[c.type] || IconFile;
  return (
    <div style={v3ins.section}>
      <div style={v3ins.sectionHead}>
        <div style={v3ins.sectionTitle}>
          <Ico size={12} stroke={V3.INK}/>
          <span>{c.type}</span>
        </div>
        <span style={{ color: V3.INK4, cursor: 'pointer' }}>···</span>
      </div>
      {c.type === 'Transform' && (
        <div style={v3ins.kv}>
          <span style={v3ins.k}>position</span><span style={v3ins.v}>{c.pos[0]}, {c.pos[1]}</span>
          <span style={v3ins.k}>rotation</span><span style={v3ins.v}>{c.rot}°</span>
          <span style={v3ins.k}>scale</span><span style={v3ins.v}>{c.scale.toFixed(2)}</span>
        </div>
      )}
      {c.type === 'Sprite' && (
        <div style={v3ins.kv}>
          <span style={v3ins.k}>source</span><span style={v3ins.v}>{c.source}</span>
          <span style={v3ins.k}>tint</span>
          <span style={{ ...v3ins.v, display: 'inline-flex', gap: 8, alignItems: 'center' }}>
            <span style={{ width: 12, height: 12, background: c.tint, border: `1px solid ${V3.RULE2}` }}/>
            <span>{c.tint}</span>
          </span>
        </div>
      )}
      {c.type === 'Script' && (
        <div onClick={() => onOpenScript && onOpenScript(c.source)} style={{ cursor: 'pointer' }}>
          <div style={v3ins.kv}>
            <span style={v3ins.k}>source</span>
            <span style={{ ...v3ins.v, color: V3.NAVY, textDecoration: 'underline', textUnderlineOffset: 2 }}>{c.source}</span>
            <span style={v3ins.k}>lines</span><span style={v3ins.v}>{c.lines}</span>
          </div>
        </div>
      )}
      {c.type === 'Camera' && (
        <div style={v3ins.kv}>
          <span style={v3ins.k}>size</span><span style={v3ins.v}>{c.size[0]} × {c.size[1]}</span>
          <span style={v3ins.k}>active</span><span style={{ ...v3ins.v, color: c.active ? V3.ORANGE : V3.INK4 }}>{c.active ? 'true' : 'false'}</span>
        </div>
      )}
    </div>
  );
}

function V2Key({ children }) {
  return (
    <span style={{
      display: 'inline-block', padding: '1px 6px',
      border: `1px solid ${V3.RULE2}`,
      fontFamily: V3.F_MONO, fontSize: 10.5,
      color: V3.INK3, margin: '0 1px',
    }}>{children}</span>
  );
}

Object.assign(window, { V2Inspector, V2Key });
