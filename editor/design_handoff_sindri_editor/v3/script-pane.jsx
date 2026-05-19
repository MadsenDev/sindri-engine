// v2 Script pane — clean tabs, calm diff blocks, no banner.

const v3sp = {
  root: { height: '100%', display: 'flex', flexDirection: 'column', background: V3.PAPER, borderTop: `1px solid ${V3.RULE}`, overflow: 'hidden' },
  tabs: { display: 'flex', alignItems: 'center', height: 36, borderBottom: `1px solid ${V3.RULE}`, paddingRight: 14, background: V3.PAPER2 },
  tab: (active) => ({
    display: 'flex', alignItems: 'center', gap: 8,
    padding: '0 16px', height: '100%',
    fontFamily: V3.F_MONO, fontSize: 11.5,
    color: active ? V3.INK : V3.INK3,
    background: active ? V3.PAPER : 'transparent',
    cursor: 'pointer',
    borderRight: `1px solid ${V3.RULE}`,
  }),
  meta: { marginLeft: 'auto', fontFamily: V3.F_MONO, fontSize: 11, color: V3.INK4, display: 'flex', gap: 12 },
  editor: { flex: 1, fontFamily: V3.F_MONO, fontSize: 12.5, lineHeight: 1.6, overflow: 'auto', padding: '8px 0' },
  row: { display: 'flex' },
  gutter: { fontFamily: V3.F_MONO, fontSize: 10.5, color: V3.INK4, textAlign: 'right', padding: '0 12px', minWidth: 48, userSelect: 'none', flex: 'none' },
  code: { whiteSpace: 'pre', flex: 1, paddingRight: 16 },
  diffAdd: { background: 'rgba(155,176,112,0.10)' },
  diffDel: { background: 'rgba(240,192,80,0.10)' },
  ghost: { color: V3.ORANGE, opacity: 0.85 },
  empty: { padding: 32, color: V3.INK3, fontFamily: V3.F_DISPLAY, fontSize: 18 },
};

const V3_COLORS = {
  plain: V3.INK, comment: V3.INK4, string: V3.MOSS,
  number: V3.NAVY, keyword: V3.ORANGE, type: V3.NAVY,
};

function v3hl(line) {
  const tokens = [];
  const keywords = ['function', 'local', 'end', 'return', 'if', 'then', 'else', 'elseif', 'do', 'while', 'for', 'not', 'and', 'or', 'true', 'false', 'nil'];
  const re = /(--[^\n]*|"[^"]*"|\b\d+(\.\d+)?\b|\b\w+\b|[^\s\w])/g;
  let m, last = 0;
  while ((m = re.exec(line))) {
    if (m.index > last) tokens.push({ t: line.slice(last, m.index), c: 'plain' });
    const w = m[0];
    if (w.startsWith('--')) tokens.push({ t: w, c: 'comment' });
    else if (w.startsWith('"')) tokens.push({ t: w, c: 'string' });
    else if (/^\d/.test(w)) tokens.push({ t: w, c: 'number' });
    else if (keywords.includes(w)) tokens.push({ t: w, c: 'keyword' });
    else if (/^[A-Z]/.test(w)) tokens.push({ t: w, c: 'type' });
    else tokens.push({ t: w, c: 'plain' });
    last = m.index + w.length;
  }
  if (last < line.length) tokens.push({ t: line.slice(last), c: 'plain' });
  return tokens;
}

function V2CodeLine({ tokens }) {
  return (
    <React.Fragment>
      {tokens.map((tk, i) => (
        <span key={i} style={{ color: V3_COLORS[tk.c] || V3.INK, fontStyle: tk.c === 'comment' ? 'normal' : 'normal' }}>{tk.t}</span>
      ))}
    </React.Fragment>
  );
}

function V2ScriptPane({ openFile, lines, ghostProposal, inlineGhost }) {
  if (!openFile) {
    return (
      <div style={v3sp.root}>
        <div style={v3sp.tabs}>
          <div style={v3sp.meta}>
            <span><IconSparkSmall size={10} stroke={V3.ORANGE}/> open a script, or ask the assistant to write one</span>
          </div>
        </div>
        <div style={v3sp.empty}>no script open.</div>
      </div>
    );
  }

  let display = lines.map((t, i) => ({ kind: 'plain', n: i + 1, t }));
  if (ghostProposal) {
    const removedStart = ghostProposal.removed[0]?.n;
    if (removedStart) {
      const before = lines.slice(0, removedStart - 1).map((t, i) => ({ kind: 'plain', n: i + 1, t }));
      const removed = ghostProposal.removed.map(r => ({ kind: 'del', n: r.n, t: r.t }));
      const added = ghostProposal.added.map(a => ({ kind: 'add', n: a.n, t: a.t }));
      const afterStart = removedStart - 1 + ghostProposal.removed.length;
      const after = lines.slice(afterStart).map((t, i) => ({ kind: 'plain', n: afterStart + 1 + i, t }));
      display = [...before, ...removed, ...added, ...after];
    }
  }

  return (
    <div style={v3sp.root}>
      <div style={v3sp.tabs}>
        <div style={v3sp.tab(true)}>
          <IconScript size={11} stroke={V3.INK3}/>
          <span>{openFile}</span>
          {ghostProposal && <span style={{ color: V3.ORANGE, marginLeft: 4 }}>●</span>}
        </div>
        <div style={v3sp.tab(false)}>
          <IconScript size={11} stroke={V3.INK4}/>
          <span>beacon.lua</span>
        </div>
        <div style={v3sp.meta}>
          <span>lua · LÖVE 11.5</span>
        </div>
      </div>

      <div style={v3sp.editor}>
        {display.map((row, i) => (
          <div key={i} style={{ ...v3sp.row, ...(row.kind === 'add' ? v3sp.diffAdd : row.kind === 'del' ? v3sp.diffDel : {}) }}>
            <div style={v3sp.gutter}>
              {row.kind === 'add' ? <span style={{ color: V3.MOSS }}>+</span>
                : row.kind === 'del' ? <span style={{ color: V3.ORANGE }}>−</span>
                : row.n}
            </div>
            <div style={v3sp.code}><V2CodeLine tokens={v3hl(row.t)}/></div>
          </div>
        ))}

        {inlineGhost && (
          <React.Fragment>
            <div style={v3sp.row}>
              <div style={v3sp.gutter}><span style={{ color: V3.ORANGE }}>✦</span></div>
              <div style={{ ...v3sp.code, ...v3sp.ghost }}>
                {inlineGhost.map((l, i) => <div key={i}>{l}</div>)}
              </div>
            </div>
            <div style={{ ...v3sp.row, padding: '6px 0' }}>
              <div style={v3sp.gutter}/>
              <div style={{ fontFamily: V3.F_MONO, fontSize: 10.5, color: V3.INK4 }}>
                <V2Key>tab</V2Key> accept · <V2Key>esc</V2Key> dismiss
              </div>
            </div>
          </React.Fragment>
        )}
      </div>
    </div>
  );
}

Object.assign(window, { V2ScriptPane });
