// Script pane — bottom dock. Code editor with line numbers, syntax highlighting via spans,
// and AI "ghost" suggestions inline.

const spStyles = {
  root: {
    height: '100%',
    display: 'flex',
    flexDirection: 'column',
    background: 'var(--paper-2)',
    borderTop: '1px solid var(--rule)',
    overflow: 'hidden',
  },
  tabs: {
    display: 'flex',
    alignItems: 'center',
    height: 32,
    borderBottom: '1px solid var(--rule)',
    paddingRight: 12,
    background: 'var(--paper)',
  },
  tab: (active) => ({
    display: 'flex', alignItems: 'center', gap: 8,
    padding: '0 14px',
    height: '100%',
    fontFamily: 'var(--f-mono)',
    fontSize: 11,
    color: active ? 'var(--ink)' : 'var(--ink-3)',
    background: active ? 'var(--paper-2)' : 'transparent',
    borderRight: '1px solid var(--rule)',
    cursor: 'pointer',
    position: 'relative',
  }),
  tabClose: { color: 'var(--ink-3)', opacity: 0.6 },
  ai: { marginLeft: 'auto', display: 'flex', alignItems: 'center', gap: 10, fontFamily: 'var(--f-mono)', fontSize: 10, color: 'var(--ink-3)' },
  editor: {
    flex: 1,
    fontFamily: 'var(--f-mono)',
    fontSize: 12.5,
    lineHeight: 1.55,
    color: 'var(--ink)',
    overflow: 'auto',
    position: 'relative',
  },
  lineRow: { display: 'flex', alignItems: 'flex-start' },
  gutter: {
    fontFamily: 'var(--f-mono)',
    fontSize: 10.5,
    color: 'var(--ink-4)',
    textAlign: 'right',
    padding: '0 12px',
    minWidth: 44,
    userSelect: 'none',
    flex: 'none',
  },
  code: { whiteSpace: 'pre', flex: 1, paddingRight: 16 },
  ghost: { color: 'var(--orange)', fontStyle: 'italic', opacity: 0.8 },
  diffAdd: { background: 'rgba(94,107,58,0.08)', borderLeft: '2px solid var(--moss)' },
  diffDel: { background: 'rgba(212,84,30,0.06)', borderLeft: '2px solid var(--orange)' },
  proposalBar: {
    margin: '8px 14px 0 56px',
    border: '1px solid var(--orange)',
    background: 'rgba(212,84,30,0.04)',
    padding: '8px 12px',
    fontSize: 11,
    color: 'var(--ink-2)',
    display: 'flex',
    alignItems: 'center',
    gap: 12,
  },
  proposalTitle: {
    fontFamily: 'var(--f-display)',
    fontStyle: 'italic',
    fontSize: 14,
    color: 'var(--ink)',
  },
  actBtn: (primary) => ({
    fontFamily: 'var(--f-sans)',
    fontSize: 10,
    letterSpacing: '0.14em',
    textTransform: 'uppercase',
    fontWeight: 500,
    padding: '5px 10px',
    border: '1px solid ' + (primary ? 'var(--orange)' : 'var(--ink-3)'),
    background: primary ? 'var(--orange)' : 'transparent',
    color: primary ? 'var(--paper)' : 'var(--ink-2)',
    cursor: 'pointer',
  }),
  emptyScript: {
    display: 'flex', flexDirection: 'column', gap: 6,
    padding: 28,
    fontFamily: 'var(--f-display)',
    fontStyle: 'italic',
    fontSize: 16,
    color: 'var(--ink-3)',
  },
};

// Very simple Lua-ish highlighter
function highlight(line) {
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

const COLORS = {
  plain: 'var(--ink)',
  comment: 'var(--ink-3)',
  string: 'var(--moss)',
  number: 'var(--navy)',
  keyword: 'var(--orange)',
  type: 'var(--navy)',
};

function CodeLine({ tokens }) {
  return (
    <React.Fragment>
      {tokens.map((tk, i) => (
        <span key={i} style={{ color: COLORS[tk.c] || 'var(--ink)', fontStyle: tk.c === 'comment' ? 'italic' : 'normal' }}>{tk.t}</span>
      ))}
    </React.Fragment>
  );
}

function ScriptPane({ openFile, lines, ghostProposal, onAccept, onReject, inlineGhost }) {
  if (!openFile) {
    return (
      <div style={spStyles.root}>
        <div style={spStyles.tabs}>
          <div style={spStyles.ai}>
            <IconSparkSmall size={11} stroke="var(--orange)"/>
            <span>Open a script from the inspector — or ask the assistant to write one.</span>
          </div>
        </div>
        <div style={spStyles.emptyScript}>no script open.</div>
      </div>
    );
  }

  // Build display lines. If ghostProposal exists, we splice removed/added.
  let display = lines.map((t, i) => ({ kind: 'plain', n: i + 1, t }));

  if (ghostProposal) {
    // Find removed start line in the original
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
    <div style={spStyles.root}>
      <div style={spStyles.tabs}>
        <div style={spStyles.tab(true)}>
          <IconScript size={11} stroke="var(--ink-3)"/>
          <span>{openFile}</span>
          {ghostProposal && <span style={{ color: 'var(--orange)', marginLeft: 6 }}>● proposal</span>}
          <span style={spStyles.tabClose}>×</span>
        </div>
        <div style={spStyles.tab(false)}>
          <IconScript size={11} stroke="var(--ink-4)"/>
          <span>beacon.lua</span>
          <span style={spStyles.tabClose}>×</span>
        </div>
        <div style={spStyles.ai}>
          <span>lua · LÖVE 11.5</span>
          <span style={{ color: 'var(--ink-4)' }}>·</span>
          <span>UTF-8</span>
        </div>
      </div>

      {ghostProposal && (
        <div style={spStyles.proposalBar}>
          <IconSparkle size={13} stroke="var(--orange)"/>
          <div>
            <div style={spStyles.proposalTitle}>{ghostProposal.title}</div>
            <div style={{ marginTop: 2, fontFamily: 'var(--f-mono)', fontSize: 10.5, color: 'var(--ink-3)' }}>
              −{ghostProposal.removed.length} / +{ghostProposal.added.length} lines · from prompt: <span style={{ fontStyle: 'italic', fontFamily: 'var(--f-display)', fontSize: 12 }}>"{ghostProposal.prompt}"</span>
            </div>
          </div>
          <div style={{ marginLeft: 'auto', display: 'flex', gap: 8 }}>
            <button style={spStyles.actBtn(false)} onClick={onReject}>Reject</button>
            <button style={spStyles.actBtn(true)} onClick={onAccept}>Accept</button>
          </div>
        </div>
      )}

      <div style={spStyles.editor}>
        {display.map((row, i) => (
          <div key={i} style={{
            ...spStyles.lineRow,
            ...(row.kind === 'add' ? spStyles.diffAdd : {}),
            ...(row.kind === 'del' ? spStyles.diffDel : {}),
          }}>
            <div style={spStyles.gutter}>
              {row.kind === 'add' ? <span style={{ color: 'var(--moss)' }}>+</span>
                : row.kind === 'del' ? <span style={{ color: 'var(--orange)' }}>−</span>
                : row.n}
            </div>
            <div style={spStyles.code}>
              <CodeLine tokens={highlight(row.t)} />
            </div>
          </div>
        ))}

        {inlineGhost && (
          <React.Fragment>
            <div style={spStyles.lineRow}>
              <div style={spStyles.gutter}><span style={{ color: 'var(--orange)' }}>✦</span></div>
              <div style={{ ...spStyles.code, ...spStyles.ghost }}>
                {inlineGhost.map((l, i) => <div key={i}>{l}</div>)}
              </div>
            </div>
            <div style={{ ...spStyles.lineRow, padding: '6px 0' }}>
              <div style={spStyles.gutter}></div>
              <div style={{ fontFamily: 'var(--f-mono)', fontSize: 10, color: 'var(--ink-3)' }}>
                <KeyChip>tab</KeyChip> accept · <KeyChip>esc</KeyChip> dismiss · <KeyChip>⌘</KeyChip> <KeyChip>↵</KeyChip> apply &amp; explain
              </div>
            </div>
          </React.Fragment>
        )}
      </div>
    </div>
  );
}

Object.assign(window, { ScriptPane });
