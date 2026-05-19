// v2 SceneTree — simpler tree, quiet labels, no all-caps banners.

const v3tree = {
  root: { display: 'flex', flexDirection: 'column', height: '100%', fontFamily: V3.F_SANS, fontSize: 13, color: V3.INK },
  tabs: {
    display: 'flex', height: 42, padding: '0 16px',
    borderBottom: `1px solid ${V3.RULE}`,
    alignItems: 'flex-end', gap: 20,
  },
  tab: (active) => ({
    paddingBottom: 10, paddingTop: 10,
    fontSize: 12.5,
    color: active ? V3.INK : V3.INK3,
    fontWeight: active ? 500 : 400,
    borderBottom: active ? `2px solid ${V3.INK}` : '2px solid transparent',
    marginBottom: -1,
    cursor: 'pointer',
  }),
  sectionRow: {
    display: 'flex', alignItems: 'center', justifyContent: 'space-between',
    padding: '14px 16px 6px',
  },
  sectionLabel: { color: V3.INK3, fontSize: 12 },
  rows: { display: 'flex', flexDirection: 'column' },
  row: (selected) => ({
    display: 'flex', alignItems: 'center', gap: 8,
    padding: '6px 16px', cursor: 'pointer',
    background: selected ? '#1e2530' : 'transparent',
    borderLeft: selected ? `2px solid ${V3.INK}` : '2px solid transparent',
    color: V3.INK,
    fontSize: 13,
  }),
  ghostRow: {
    display: 'flex', alignItems: 'center', gap: 8,
    padding: '6px 16px',
    color: V3.ORANGE,
    fontSize: 13,
  },
  comp: {
    display: 'flex', alignItems: 'center', gap: 8,
    padding: '4px 16px 4px 36px',
    color: V3.INK3, fontSize: 12,
  },
  compProp: {
    display: 'flex', alignItems: 'center', gap: 8,
    padding: '4px 16px 4px 36px',
    color: V3.ORANGE, fontSize: 12,
  },
  meta: {
    marginLeft: 'auto', color: V3.INK4, fontFamily: V3.F_MONO, fontSize: 10.5,
  },
  caret: (open) => ({
    display: 'inline-flex',
    transform: open ? 'rotate(0)' : 'rotate(-90deg)',
    color: V3.INK4, transition: 'transform 80ms',
  }),
  dot: (c) => ({
    width: 7, height: 7, borderRadius: 0, flex: 'none',
    background: c === 'orange' ? V3.ORANGE : c === 'navy' ? V3.NAVY : c === 'stone' ? V3.INK4 : V3.MOSS,
  }),
};

function V2SceneTree({ entities, selectedId, onSelect, openIds, onToggle, ghostEntities = [], proposedComponents = {}, activeTab, onTabChange }) {
  return (
    <div style={v3tree.root}>
      <div style={v3tree.tabs}>
        <div style={v3tree.tab(activeTab === 'scene')} onClick={() => onTabChange('scene')}>Scene</div>
        <div style={v3tree.tab(activeTab === 'files')} onClick={() => onTabChange('files')}>Files</div>
        <div style={v3tree.tab(activeTab === 'history')} onClick={() => onTabChange('history')}>History</div>
      </div>

      {activeTab === 'scene' && (
        <div style={{ flex: 1, overflow: 'auto' }}>
          <div style={v3tree.sectionRow}>
            <span style={v3tree.sectionLabel}>editor_scene</span>
            <IconPlus size={12} stroke={V3.INK3}/>
          </div>
          <div style={v3tree.rows}>
            {entities.map(e => {
              const open = openIds.has(e.id);
              const sel = selectedId === e.id;
              const props = proposedComponents[e.id] || [];
              return (
                <React.Fragment key={e.id}>
                  <div style={v3tree.row(sel)} onClick={() => onSelect(e.id)}>
                    <span style={v3tree.caret(open)} onClick={(ev) => { ev.stopPropagation(); onToggle(e.id); }}>
                      <IconChevron size={10}/>
                    </span>
                    <span style={v3tree.dot(e.color)}/>
                    <span>{e.name}</span>
                  </div>
                  {open && e.components.map((c) => {
                    const Ico = COMPONENT_ICONS[c.type] || IconFile;
                    return (
                      <div key={c.type} style={v3tree.comp}>
                        <Ico size={11} stroke={V3.INK3}/>
                        <span>{c.type}</span>
                        {c.source && <span style={v3tree.meta}>{c.source}</span>}
                      </div>
                    );
                  })}
                  {open && props.map((c) => {
                    const Ico = COMPONENT_ICONS[c.type] || IconFile;
                    return (
                      <div key={'p' + c.type} style={v3tree.compProp}>
                        <Ico size={11} stroke={V3.ORANGE}/>
                        <span>{c.type}</span>
                        <span style={{ ...v3tree.meta, color: V3.ORANGE }}>proposed</span>
                      </div>
                    );
                  })}
                </React.Fragment>
              );
            })}

            {ghostEntities.length > 0 && (
              <div style={{ ...v3tree.sectionRow, marginTop: 12, color: V3.ORANGE }}>
                <span style={{ color: V3.ORANGE, fontSize: 12 }}>Proposed by AI</span>
                <span style={{ color: V3.ORANGE, fontSize: 11, fontFamily: V3.F_MONO }}>+{ghostEntities.length}</span>
              </div>
            )}
            {ghostEntities.map((e, i) => (
              <div key={'g' + i} style={v3tree.ghostRow}>
                <span style={{ width: 10 }}/>
                <span style={v3tree.dot('orange')}/>
                <span>{e.name}{e.note && <span style={{ color: V3.INK4, marginLeft: 6, fontSize: 11 }}>{e.note}</span>}</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {activeTab === 'files' && <V2Files/>}
      {activeTab === 'history' && <V2History/>}
    </div>
  );
}

function V2Files() {
  const tree = [
    { name: 'scenes/', children: ['editor_scene.sndr', 'level_01.sndr'] },
    { name: 'scripts/', children: ['drone.lua', 'beacon.lua', 'player.lua'] },
    { name: 'sprites/', children: ['beacon.png', 'drone.png', 'ground.png'] },
    { name: 'audio/', children: ['chime.ogg', 'theme.ogg'] },
    { name: 'sindri.toml' },
  ];
  return (
    <div style={{ padding: '14px 16px', fontSize: 12.5, color: V3.INK }}>
      {tree.map((f) => (
        <div key={f.name} style={{ marginBottom: 8 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            {f.children ? <IconChevron size={10} stroke={V3.INK4}/> : <span style={{ width: 10 }}/>}
            {f.children ? <IconFolder size={12} stroke={V3.INK3}/> : <IconFile size={12} stroke={V3.INK3}/>}
            <span>{f.name}</span>
          </div>
          {f.children && f.children.map((c) => (
            <div key={c} style={{ paddingLeft: 30, color: V3.INK3, fontSize: 12, marginTop: 3 }}>{c}</div>
          ))}
        </div>
      ))}
    </div>
  );
}

function V2History() {
  const A = window.SINDRI_DATA.ACTIVITY;
  return (
    <div style={{ padding: '8px 16px', fontSize: 12 }}>
      {A.map((a, i) => (
        <div key={i} style={{
          display: 'flex', alignItems: 'flex-start', gap: 10,
          padding: '8px 0', borderBottom: `1px solid ${V3.RULE}`,
        }}>
          <span style={{ fontFamily: V3.F_MONO, fontSize: 10.5, color: V3.INK4, minWidth: 36 }}>{a.t}</span>
          <span style={{
            width: 6, height: 6, marginTop: 5,
            background: a.actor === 'ai' ? V3.ORANGE : V3.INK4,
          }}/>
          <span style={{ color: V3.INK2, fontSize: 12, lineHeight: 1.5 }}>{a.action}</span>
        </div>
      ))}
    </div>
  );
}

Object.assign(window, { V2SceneTree });
