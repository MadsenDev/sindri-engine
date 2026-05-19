// v2 Editor — minimal topbar, no menu items in nav, AI status surfaced quietly.

const v3ed = {
  shell: { position: 'absolute', inset: 0, display: 'grid', gridTemplateRows: '56px 1fr 28px', background: V3.PAPER, color: V3.INK },
  // ── topbar
  topbar: {
    display: 'grid', gridTemplateColumns: '380px 1fr 300px',
    borderBottom: `1px solid ${V3.RULE2}`, background: V3.PAPER,
    alignItems: 'center'
  },
  topLeft: { display: 'flex', alignItems: 'center', padding: '0 18px', gap: 12, height: '100%', overflow: 'hidden', minWidth: 0 },
  topCenter: { display: 'flex', alignItems: 'center', padding: '0 18px', gap: 14, height: '100%' },
  topRight: { display: 'flex', alignItems: 'center', justifyContent: 'flex-end', padding: '0 22px', gap: 12, height: '100%' },

  breadcrumb: {
    fontFamily: V3.F_MONO, fontSize: 12, color: V3.INK3,
    display: 'flex', alignItems: 'baseline', gap: 6,
    cursor: 'pointer',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    minWidth: 0
  },
  bcCurrent: { color: V3.INK },

  cmdK: {
    display: 'flex', alignItems: 'center', gap: 10,
    height: 32, padding: '0 14px',
    background: V3.PAPER2,
    border: `1px solid ${V3.RULE}`,
    color: V3.INK3,
    fontSize: 13, fontFamily: V3.F_DISPLAY, width: '100%', maxWidth: 420, cursor: 'text'
  },
  cmdKLeft: { display: 'flex', alignItems: 'center', gap: 8, flex: 1 },
  cmdKKeys: { fontFamily: V3.F_MONO, fontSize: 11, color: V3.INK4, fontStyle: 'normal' },

  // run group — tight, no big borders
  run: { display: 'flex', alignItems: 'center', gap: 4 },
  runBtn: (variant) => ({
    width: 34, height: 28,
    display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
    cursor: 'pointer',
    background: variant === 'primary' ? V3.INK : 'transparent',
    color: variant === 'primary' ? V3.PAPER : V3.INK3,
    border: variant === 'primary' ? `1px solid ${V3.INK}` : `1px solid ${V3.RULE2}`
  }),

  // top-right
  aiStatus: {
    display: 'flex', alignItems: 'center', gap: 8,
    fontSize: 11.5, color: V3.INK3,
    fontFamily: V3.F_MONO,
    padding: '5px 10px',
    border: `1px solid ${V3.RULE}`,
    background: V3.PAPER2,
    maxWidth: 240,
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis'
  },
  aiStatusDot: (color) => ({ width: 6, height: 6, background: color }),
  aiStatusLabel: { color: V3.INK2, fontFamily: V3.F_SANS, fontSize: 12, marginRight: 4 },

  composeBtn: {
    display: 'inline-flex', alignItems: 'center', gap: 6,
    fontFamily: V3.F_DISPLAY, fontSize: 16, color: V3.ORANGE, cursor: 'pointer'
  },

  // ── body
  body: { display: 'grid', gridTemplateColumns: '260px 1fr 360px', minHeight: 0 },
  left: { borderRight: `1px solid ${V3.RULE}`, minHeight: 0, overflow: 'hidden', background: V3.PAPER },
  center: { display: 'grid', gridTemplateRows: '1fr 280px', minHeight: 0 },
  centerTop: { borderBottom: `1px solid ${V3.RULE}`, minHeight: 0, position: 'relative', overflow: 'hidden' },
  right: { borderLeft: `1px solid ${V3.RULE}`, minHeight: 0, overflow: 'hidden', background: V3.PAPER },

  // ── scenario picker (subtle pill bar, top-right of viewport)
  scenarioBar: {
    position: 'absolute', right: 14, top: 52,
    display: 'flex', gap: 0,
    background: V3.PAPER,
    border: `1px solid ${V3.RULE2}`,
    fontFamily: V3.F_SANS, fontSize: 11,
    zIndex: 30
  },
  scenarioBtn: (active) => ({
    padding: '5px 10px',
    background: active ? V3.INK : 'transparent',
    color: active ? V3.PAPER : V3.INK3,
    cursor: 'pointer',
    borderLeft: `1px solid ${V3.RULE2}`
  }),
  scenarioLbl: { padding: '5px 10px', color: V3.INK4, display: 'inline-flex', alignItems: 'center', gap: 4 },

  // ── status bar
  status: { display: 'flex', alignItems: 'center', borderTop: `1px solid ${V3.RULE}`, background: V3.PAPER2, padding: '0 18px', gap: 18, fontFamily: V3.F_MONO, fontSize: 11, color: V3.INK4 },
  statusDot: (c) => ({ width: 6, height: 6, background: c, display: 'inline-block', marginRight: 6 }),
  statusRight: { marginLeft: 'auto', display: 'flex', gap: 18 }
};

function V2Editor({ scenario, setScenario, state, dispatch, logo }) {
  const selectedEntity = state.entities.find((e) => e.id === state.selectedId) || null;
  const isProposalMode = scenario === 'review' || scenario === 'inline';
  const ghostEntities = scenario === 'compose-result' ? [
  { name: 'Player', x: 480, y: 300, w: 32, h: 32, rot: 0 },
  { name: 'Wolf', x: 740, y: 240, w: 56, h: 28, rot: -4 }] :
  [];
  const proposedComponents = scenario === 'review' ? {
    drone: [{ type: 'Audio' }]
  } : {};

  // Quiet AI status text
  const aiStatus = (() => {
    if (scenario === 'composing') return { dot: V3.ORANGE, text: '"forest clearing" — composing', kind: 'busy' };
    if (scenario === 'compose-result') return { dot: V3.ORANGE, text: '6 entities staged', kind: 'review' };
    if (scenario === 'review' || scenario === 'inline') return { dot: V3.ORANGE, text: '3 changes pending', kind: 'review' };
    return { dot: V3.MOSS, text: 'ready', kind: 'idle' };
  })();

  return (
    <div style={v3ed.shell}>
      <V2Topbar logo={logo} onCmdK={() => setScenario('cmdk')} onCompose={() => setScenario('composing')} aiStatus={aiStatus} />

      <div style={v3ed.body}>
        <aside style={v3ed.left}>
          <V2SceneTree
            entities={state.entities}
            selectedId={state.selectedId}
            onSelect={(id) => dispatch({ type: 'select', id })}
            openIds={state.openIds}
            onToggle={(id) => dispatch({ type: 'toggle', id })}
            ghostEntities={ghostEntities}
            proposedComponents={proposedComponents}
            activeTab={state.sceneTab}
            onTabChange={(t) => dispatch({ type: 'sceneTab', t })} />
          
        </aside>

        <main style={v3ed.center}>
          <div style={v3ed.centerTop}>
            <V2Viewport
              entities={state.entities}
              selectedId={state.selectedId}
              onSelect={(id) => dispatch({ type: 'select', id })}
              ghostEntities={ghostEntities}
              viewTab={state.viewTab}
              onViewTabChange={(t) => dispatch({ type: 'viewTab', t })}
              hoverProposal={scenario === 'review' && state.hoverProposal === 'ch1'}
              isComposing={scenario === 'composing'} />
            
            {/* subtle scenario switcher */}
            <div style={v3ed.scenarioBar}>
              <span style={v3ed.scenarioLbl}><IconSparkSmall size={10} stroke={V3.ORANGE} />Demo</span>
              {[
              ['rest', 'Rest'],
              ['cmdk', '⌘K'],
              ['composing', 'Compose'],
              ['compose-result', 'Result'],
              ['review', 'Review'],
              ['inline', 'Inline']].
              map(([k, lbl]) =>
              <span key={k} style={v3ed.scenarioBtn(scenario === k)} onClick={() => setScenario(k)}>{lbl}</span>
              )}
            </div>
          </div>
          <V2ScriptPane
            openFile={state.openFile}
            lines={state.openFile === 'drone.lua' ? window.SINDRI_DATA.DRONE_LUA : window.SINDRI_DATA.BEACON_LUA}
            ghostProposal={scenario === 'review' || scenario === 'inline' ? window.SINDRI_DATA.PROPOSED_DIFF.changes[0] : null}
            inlineGhost={scenario === 'inline-ghost' ? [
            'function Drone:onHitBeacon()',
            '  audio.play("chime.ogg")',
            '  particles.burst(self.transform, "ember")',
            'end'] :
            null} />
          
        </main>

        <aside style={v3ed.right}>
          {isProposalMode ?
          <V2ProposalsLane
            diff={window.SINDRI_DATA.PROPOSED_DIFF}
            statuses={state.proposalStatuses}
            onSet={(id, v) => dispatch({ type: 'set-proposal', id, v })}
            onAcceptAll={() => dispatch({ type: 'accept-all' })}
            onRejectAll={() => dispatch({ type: 'reject-all' })}
            onHover={(id) => dispatch({ type: 'hover-proposal', id })} /> :


          <V2Inspector
            entity={selectedEntity}
            onOpenScript={(f) => dispatch({ type: 'open-file', f })}
            onTriggerAI={(k) => k === 'chase' && setScenario('review')} />

          }
        </aside>
      </div>

      <V2Statusbar aiStatus={aiStatus} />
    </div>);

}

function V2Topbar({ logo, onCmdK, onCompose, aiStatus }) {
  return (
    <header style={v3ed.topbar}>
      <div style={v3ed.topLeft}>
        <LogoLockup which={logo} size={22} />
        <div style={{ width: 1, height: 20, background: V3.RULE2, flex: 'none' }} />
        <div style={v3ed.breadcrumb} title="scenes / editor_scene">
          <span style={{ flex: 'none' }}>scenes</span>
          <span style={{ color: V3.INK4, margin: '0 4px', flex: 'none' }}>/</span>
          <span style={{
            color: V3.INK,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            minWidth: 0,
            flex: '1 1 auto',
          }}>editor_scene</span>
        </div>
      </div>
      <div style={v3ed.topCenter}>
        <div style={v3ed.cmdK} onClick={onCmdK}>
          <div style={v3ed.cmdKLeft}>
            <IconSparkle size={14} stroke={V3.ORANGE} />
            <span>Ask Sindri to build something…</span>
          </div>
          <span style={v3ed.cmdKKeys}><V2Key>⌘</V2Key><V2Key>K</V2Key></span>
        </div>

        <div style={v3ed.run}>
          <span style={v3ed.runBtn('primary')} title="Play"><IconPlay size={12} stroke={V3.PAPER} /></span>
          <span style={v3ed.runBtn()} title="Pause"><IconPause size={12} /></span>
          <span style={v3ed.runBtn()} title="Stop"><IconStop size={11} /></span>
        </div>
      </div>
      <div style={v3ed.topRight}>
        <span style={v3ed.composeBtn} onClick={onCompose}>
          <IconSparkle size={13} stroke={V3.ORANGE} /> Compose
        </span>
        <div style={v3ed.aiStatus}>
          <span style={v3ed.aiStatusDot(aiStatus.dot)} />
          <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{aiStatus.text}</span>
          <span style={{ color: V3.INK4 }}>·</span>
          <span style={{ flex: 'none' }}>qwen3:14b</span>
        </div>
      </div>
    </header>);

}

function V2Statusbar({ aiStatus }) {
  return (
    <footer style={v3ed.status}>
      <span><span style={v3ed.statusDot(V3.MOSS)} />engine ready</span>
      <span><span style={v3ed.statusDot(V3.MOSS)} />ollama · localhost:11434</span>
      <span style={{ color: V3.INK4 }}>·</span>
      <span>{aiStatus.text}</span>
      <div style={v3ed.statusRight}>
        <span>4 entities</span>
        <span>0 errors</span>
        <span>sindri v0.1.4</span>
      </div>
    </footer>);

}

Object.assign(window, { V2Editor });