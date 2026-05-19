// Scene tree — pure type list. Ghost rows for AI-proposed entities.

const treeStyles = {
  root: {
    display: 'flex',
    flexDirection: 'column',
    height: '100%',
    fontSize: 12.5,
    color: 'var(--ink)',
    fontFamily: 'var(--f-sans)',
  },
  tabs: {
    display: 'flex',
    alignItems: 'flex-end',
    height: 36,
    borderBottom: '1px solid var(--rule)',
    paddingLeft: 14,
    gap: 18,
  },
  tab: (active) => ({
    fontSize: 10.5,
    letterSpacing: '0.16em',
    textTransform: 'uppercase',
    paddingBottom: 8,
    color: active ? 'var(--ink)' : 'var(--ink-3)',
    cursor: 'pointer',
    borderBottom: active ? '2px solid var(--ink)' : '2px solid transparent',
    marginBottom: -1,
    fontWeight: 500,
  }),
  sectionHeader: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    padding: '14px 14px 8px',
  },
  sectionLabel: {
    fontSize: 10,
    letterSpacing: '0.16em',
    textTransform: 'uppercase',
    color: 'var(--ink-3)',
    fontWeight: 500,
  },
  add: {
    cursor: 'pointer',
    color: 'var(--ink-3)',
    display: 'flex',
    alignItems: 'center',
  },
  rows: { display: 'flex', flexDirection: 'column' },
  row: (selected, ghost) => ({
    display: 'flex',
    alignItems: 'center',
    gap: 8,
    padding: '5px 14px',
    cursor: 'pointer',
    background: selected ? 'var(--paper-3)' : 'transparent',
    borderLeft: selected ? '2px solid var(--ink)' : '2px solid transparent',
    color: ghost ? 'var(--ink-3)' : 'var(--ink)',
    fontStyle: ghost ? 'italic' : 'normal',
    fontFamily: ghost ? 'var(--f-display)' : 'var(--f-sans)',
    fontSize: ghost ? 14 : 12.5,
    position: 'relative',
  }),
  childRow: (ghost) => ({
    display: 'flex',
    alignItems: 'center',
    gap: 8,
    padding: '3px 14px 3px 38px',
    color: ghost ? 'var(--orange)' : 'var(--ink-3)',
    fontSize: 11.5,
    fontFamily: 'var(--f-sans)',
    fontStyle: ghost ? 'italic' : 'normal',
  }),
  caret: (open) => ({
    display: 'inline-flex',
    transform: open ? 'rotate(0)' : 'rotate(-90deg)',
    color: 'var(--ink-3)',
    transition: 'transform 80ms',
  }),
  dot: (color) => ({
    width: 6, height: 6, borderRadius: 0,
    background:
      color === 'orange' ? 'var(--orange)' :
      color === 'navy'   ? 'var(--navy)'   :
      color === 'stone'  ? 'var(--ink-3)'  :
      'var(--moss)',
    flex: 'none',
  }),
  ghostBadge: {
    marginLeft: 'auto',
    fontFamily: 'var(--f-sans)',
    fontSize: 9.5,
    letterSpacing: '0.16em',
    textTransform: 'uppercase',
    color: 'var(--orange)',
    fontStyle: 'normal',
  },
  componentRow: (selected) => ({
    display: 'flex',
    alignItems: 'center',
    gap: 8,
    padding: '3px 14px 3px 38px',
    color: 'var(--ink-2)',
    fontSize: 11.5,
    cursor: 'pointer',
    background: selected ? 'var(--paper-3)' : 'transparent',
  }),
  componentMeta: {
    marginLeft: 'auto',
    color: 'var(--ink-3)',
    fontFamily: 'var(--f-mono)',
    fontSize: 10.5,
  },
};

function SceneTree({ entities, selectedId, onSelect, openIds, onToggle, ghostEntities = [], proposedComponents = {}, activeTab = 'scene', onTabChange }) {
  return (
    <div style={treeStyles.root}>
      <div style={treeStyles.tabs}>
        <div style={treeStyles.tab(activeTab === 'scene')} onClick={() => onTabChange && onTabChange('scene')}>Scene</div>
        <div style={treeStyles.tab(activeTab === 'files')} onClick={() => onTabChange && onTabChange('files')}>Files</div>
        <div style={treeStyles.tab(activeTab === 'assets')} onClick={() => onTabChange && onTabChange('assets')}>Assets</div>
        <div style={treeStyles.tab(activeTab === 'history')} onClick={() => onTabChange && onTabChange('history')}>History</div>
      </div>

      {activeTab === 'scene' && (
        <React.Fragment>
          <div style={treeStyles.sectionHeader}>
            <span style={treeStyles.sectionLabel}>Editor scene</span>
            <span style={treeStyles.add} title="New entity"><IconPlus size={12}/></span>
          </div>
          <div style={treeStyles.rows}>
            {entities.map((e) => {
              const open = openIds.has(e.id);
              const sel = selectedId === e.id;
              const propComps = proposedComponents[e.id] || [];
              return (
                <React.Fragment key={e.id}>
                  <div style={treeStyles.row(sel, false)} onClick={() => onSelect(e.id)}>
                    <span style={treeStyles.caret(open)} onClick={(ev) => { ev.stopPropagation(); onToggle(e.id); }}>
                      <IconChevron size={10}/>
                    </span>
                    <span style={treeStyles.dot(e.color)}></span>
                    <span>{e.name}</span>
                  </div>
                  {open && e.components.map((c) => {
                    const C = COMPONENT_ICONS[c.type] || IconFile;
                    return (
                      <div key={c.type} style={treeStyles.componentRow(false)}>
                        <C size={11}/>
                        <span>{c.type}</span>
                        {c.source && <span style={treeStyles.componentMeta}>{c.source}</span>}
                      </div>
                    );
                  })}
                  {open && propComps.map((c) => {
                    const C = COMPONENT_ICONS[c.type] || IconFile;
                    return (
                      <div key={'prop-' + c.type} style={{ ...treeStyles.componentRow(false), color: 'var(--orange)', fontStyle: 'italic' }}>
                        <C size={11}/>
                        <span>{c.type}</span>
                        <span style={treeStyles.componentMeta}>{c.note || 'proposed'}</span>
                      </div>
                    );
                  })}
                </React.Fragment>
              );
            })}

            {ghostEntities.length > 0 && (
              <div style={{ ...treeStyles.sectionHeader, marginTop: 12 }}>
                <span style={{ ...treeStyles.sectionLabel, color: 'var(--orange)' }}>Proposed by AI</span>
                <span style={{ ...treeStyles.add, color: 'var(--orange)', fontSize: 10, letterSpacing: '0.14em', textTransform: 'uppercase', fontFamily: 'var(--f-sans)' }}>{ghostEntities.length} new</span>
              </div>
            )}
            {ghostEntities.map((e, i) => (
              <div key={'ghost-' + i} style={treeStyles.row(false, true)}>
                <span style={treeStyles.caret(false)}><IconChevron size={10}/></span>
                <span style={treeStyles.dot('orange')}></span>
                <span>{e.name}{e.note ? <span style={{ color: 'var(--ink-3)', marginLeft: 6, fontSize: 11, fontFamily: 'var(--f-sans)', fontStyle: 'normal' }}>{e.note}</span> : null}</span>
              </div>
            ))}
          </div>
        </React.Fragment>
      )}

      {activeTab === 'files' && <FilesPane />}
      {activeTab === 'assets' && <AssetsPane />}
      {activeTab === 'history' && <HistoryPane />}
    </div>
  );
}

function FilesPane() {
  const tree = [
    { name: 'scenes/', children: ['editor_scene.sndr', 'level_01.sndr'] },
    { name: 'scripts/', children: ['drone.lua', 'beacon.lua', 'player.lua'] },
    { name: 'sprites/', children: ['beacon.png', 'drone.png', 'ground.png'] },
    { name: 'audio/', children: ['chime.ogg', 'theme.ogg'] },
    { name: 'sindri.toml' },
  ];
  return (
    <div style={{ padding: '12px 14px', fontSize: 12.5 }}>
      {tree.map((f) => (
        <div key={f.name} style={{ marginBottom: 6 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, color: 'var(--ink)' }}>
            {f.children ? <IconChevron size={10}/> : <span style={{ width: 10 }}/>}
            {f.children ? <IconFolder size={12}/> : <IconFile size={12}/>}
            <span>{f.name}</span>
          </div>
          {f.children && f.children.map((c) => (
            <div key={c} style={{ paddingLeft: 30, color: 'var(--ink-3)', fontSize: 11.5, marginTop: 2 }}>{c}</div>
          ))}
        </div>
      ))}
    </div>
  );
}

function AssetsPane() {
  return <div style={{ padding: 14, fontSize: 12, color: 'var(--ink-3)', fontStyle: 'italic', fontFamily: 'var(--f-display)' }}>Asset library — sprites, sounds, prefabs.</div>;
}

function HistoryPane() {
  const A = window.SINDRI_DATA.ACTIVITY;
  return (
    <div style={{ padding: '10px 14px', fontSize: 11.5 }}>
      {A.map((a, i) => (
        <div key={i} style={{ display: 'flex', alignItems: 'flex-start', gap: 8, padding: '6px 0', borderBottom: '1px solid var(--paper-3)' }}>
          <span style={{ fontFamily: 'var(--f-mono)', fontSize: 10, color: 'var(--ink-3)', minWidth: 32 }}>{a.t}</span>
          <span style={{
            fontSize: 9.5, letterSpacing: '0.14em', textTransform: 'uppercase',
            color: a.actor === 'ai' ? 'var(--orange)' : 'var(--ink-3)',
            fontWeight: 500, minWidth: 22,
          }}>{a.actor}</span>
          <span style={{
            color: 'var(--ink-2)',
            fontStyle: a.actor === 'ai' ? 'italic' : 'normal',
            fontFamily: a.actor === 'ai' ? 'var(--f-display)' : 'var(--f-sans)',
            fontSize: a.actor === 'ai' ? 13 : 11.5,
            lineHeight: 1.4,
          }}>{a.action}</span>
        </div>
      ))}
    </div>
  );
}

Object.assign(window, { SceneTree });
