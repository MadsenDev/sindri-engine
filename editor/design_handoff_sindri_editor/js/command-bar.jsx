// Command bar (Cmd+K) — the AI's primary input.
// Editorial palette: paper-on-paper, hairlines, serif headline.

const cbStyles = {
  scrim: {
    position: 'absolute', inset: 0,
    background: 'rgba(10, 10, 10, 0.35)',
    backdropFilter: 'blur(2px)',
    zIndex: 50,
    display: 'flex',
    alignItems: 'flex-start',
    justifyContent: 'center',
    paddingTop: 120,
  },
  panel: {
    width: 720,
    background: 'var(--paper)',
    border: '1px solid var(--ink)',
    color: 'var(--ink)',
    boxShadow: '8px 8px 0 var(--ink)',
  },
  head: {
    padding: '14px 18px 12px',
    borderBottom: '1px solid var(--rule)',
    display: 'flex',
    alignItems: 'center',
    gap: 12,
  },
  prompt: {
    fontFamily: 'var(--f-display)',
    fontSize: 26,
    lineHeight: 1.1,
    background: 'transparent',
    border: 'none',
    outline: 'none',
    flex: 1,
    color: 'var(--ink)',
  },
  caret: {
    display: 'inline-block',
    width: 2, height: 26,
    background: 'var(--orange)',
    animation: 'caret 1s steps(1) infinite',
  },
  ctxbar: {
    padding: '8px 18px',
    borderBottom: '1px solid var(--rule)',
    display: 'flex',
    gap: 8,
    alignItems: 'center',
    flexWrap: 'wrap',
  },
  ctxchip: (kind) => ({
    fontSize: 10.5,
    letterSpacing: '0.04em',
    fontFamily: 'var(--f-mono)',
    padding: '3px 8px',
    border: '1px solid ' + (kind === 'entity' ? 'var(--navy)' : 'var(--ink-3)'),
    color: kind === 'entity' ? 'var(--navy)' : 'var(--ink-3)',
    background: kind === 'entity' ? 'var(--navy-soft)' : 'transparent',
    display: 'inline-flex', gap: 6, alignItems: 'center',
  }),
  ctxlabel: {
    fontSize: 9.5, letterSpacing: '0.16em', textTransform: 'uppercase', color: 'var(--ink-3)',
    fontWeight: 500,
  },
  body: { padding: '4px 0 8px', maxHeight: 360, overflowY: 'auto' },
  group: { padding: '12px 18px 4px' },
  groupLabel: {
    fontSize: 9.5, letterSpacing: '0.16em', textTransform: 'uppercase',
    color: 'var(--ink-3)', fontWeight: 500, marginBottom: 4,
  },
  item: (active) => ({
    display: 'flex', alignItems: 'center', gap: 12,
    padding: '8px 18px',
    background: active ? 'var(--paper-3)' : 'transparent',
    borderLeft: active ? '2px solid var(--orange)' : '2px solid transparent',
    fontSize: 13,
    color: 'var(--ink)',
    cursor: 'pointer',
    whiteSpace: 'nowrap',
  }),
  itemGlyph: (kind) => ({
    width: 16, color: kind === '✦' ? 'var(--orange)' : 'var(--ink-3)',
    display: 'inline-flex', justifyContent: 'center',
    fontFamily: kind === '✦' ? 'var(--f-display)' : 'var(--f-mono)',
    fontSize: kind === '✦' ? 14 : 12,
  }),
  itemHint: {
    marginLeft: 'auto', fontFamily: 'var(--f-mono)', fontSize: 10.5,
    color: 'var(--ink-3)',
  },
  foot: {
    borderTop: '1px solid var(--rule)',
    padding: '10px 18px',
    display: 'flex', alignItems: 'center', justifyContent: 'space-between',
    fontFamily: 'var(--f-mono)', fontSize: 10.5, color: 'var(--ink-3)',
  },
  modelChip: {
    display: 'inline-flex', gap: 6, alignItems: 'center',
    fontFamily: 'var(--f-mono)', fontSize: 10.5, color: 'var(--ink)',
    border: '1px solid var(--rule)', padding: '3px 8px',
  },
  dot: (c) => ({ width: 6, height: 6, background: c, display: 'inline-block' }),
};

function CommandBar({ open, onClose, prompt = '', ctx = [] }) {
  if (!open) return null;
  const G = window.SINDRI_DATA.COMMAND_SUGGESTIONS;
  return (
    <div style={cbStyles.scrim} onClick={onClose}>
      <div style={cbStyles.panel} onClick={(e) => e.stopPropagation()}>
        <div style={cbStyles.head}>
          <IconSparkle size={20} stroke="var(--orange)"/>
          <div style={cbStyles.prompt}>
            {prompt ? prompt : <span style={{ color: 'var(--ink-3)', fontStyle: 'italic' }}>What should we build…</span>}
            <span style={cbStyles.caret}/>
          </div>
          <KeyChip>esc</KeyChip>
        </div>

        <div style={cbStyles.ctxbar}>
          <span style={cbStyles.ctxlabel}>Context</span>
          {ctx.map((c, i) => (
            <span key={i} style={cbStyles.ctxchip(c.kind)}>
              {c.kind === 'entity' && <span style={cbStyles.dot('var(--navy)')}/>}
              {c.kind === 'scene' && <IconGrid size={10}/>}
              {c.kind === 'file' && <IconScript size={10}/>}
              <span>{c.text}</span>
              <IconX size={9} stroke="currentColor"/>
            </span>
          ))}
          <span style={{ fontFamily: 'var(--f-mono)', fontSize: 10.5, color: 'var(--ink-4)' }}>+ type @ to add</span>
        </div>

        <div style={cbStyles.body}>
          {G.map((g, gi) => (
            <div key={gi} style={cbStyles.group}>
              <div style={cbStyles.groupLabel}>{g.group}</div>
              {g.items.map((it, ii) => (
                <div key={ii} style={cbStyles.item(gi === 0 && ii === 2)}>
                  <span style={cbStyles.itemGlyph(it.glyph)}>{it.glyph}</span>
                  <span style={{ flex: 1, overflow: 'hidden', textOverflow: 'ellipsis' }}>{it.label}</span>
                  {it.hint && <span style={cbStyles.itemHint}>{it.hint}</span>}
                </div>
              ))}
            </div>
          ))}
        </div>

        <div style={cbStyles.foot}>
          <div style={{ display: 'flex', gap: 12 }}>
            <span><KeyChip>↑</KeyChip><KeyChip>↓</KeyChip> navigate</span>
            <span><KeyChip>↵</KeyChip> apply</span>
            <span><KeyChip>⌘</KeyChip><KeyChip>↵</KeyChip> preview diff</span>
          </div>
          <div style={cbStyles.modelChip}>
            <span style={cbStyles.dot('var(--moss)')}/>
            <span>qwen3:14b · local</span>
          </div>
        </div>
      </div>
    </div>
  );
}

const CB_STYLE = document.createElement('style');
CB_STYLE.innerHTML = '@keyframes caret { 50% { opacity: 0; } }';
document.head.appendChild(CB_STYLE);

Object.assign(window, { CommandBar });
