// Main editor layout — top bar, panels, status bar.
// Layout is reimagined: hairline grid, no shadows, no rounded corners.

const edStyles = {
  shell: {
    position: 'absolute', inset: 0,
    display: 'grid',
    gridTemplateRows: '52px 1fr 26px',
    background: 'var(--paper)',
    color: 'var(--ink)',
  },
  topbar: {
    display: 'grid',
    gridTemplateColumns: '260px 1fr 320px',
    borderBottom: '1px solid var(--rule)',
    background: 'var(--paper)',
    alignItems: 'center',
  },
  topLeft: {
    display: 'flex', alignItems: 'center', gap: 16,
    padding: '0 18px', height: '100%',
    borderRight: '1px solid var(--rule)',
  },
  topCenter: {
    display: 'flex', alignItems: 'center',
    padding: '0 18px', height: '100%',
    gap: 16,
  },
  topRight: {
    display: 'flex', alignItems: 'center', justifyContent: 'flex-end',
    padding: '0 18px', height: '100%', gap: 12,
    borderLeft: '1px solid var(--rule)',
  },
  menuItem: {
    fontFamily: 'var(--f-sans)',
    fontSize: 12, color: 'var(--ink-2)',
    padding: '4px 0',
    cursor: 'pointer',
  },
  menus: { display: 'flex', gap: 16 },
  breadcrumb: {
    display: 'flex', alignItems: 'baseline', gap: 8,
    fontFamily: 'var(--f-mono)',
    fontSize: 11,
    color: 'var(--ink-3)',
  },
  bcSep: { color: 'var(--ink-4)' },
  bcCurrent: { color: 'var(--ink)', fontFamily: 'var(--f-display)', fontStyle: 'italic', fontSize: 16 },
  toolGroup: {
    display: 'flex', alignItems: 'center', gap: 4,
    padding: '0 8px',
    height: 26,
    borderLeft: '1px solid var(--rule)',
    color: 'var(--ink-2)',
  },
  toolBtn: (active) => ({
    width: 26, height: 26,
    display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
    cursor: 'pointer',
    color: active ? 'var(--orange)' : 'var(--ink-2)',
    background: active ? 'rgba(212,84,30,0.08)' : 'transparent',
  }),
  cmdK: {
    display: 'flex', alignItems: 'center', gap: 10,
    height: 28,
    padding: '0 12px',
    border: '1px solid var(--rule)',
    background: 'var(--paper-2)',
    color: 'var(--ink-3)',
    fontFamily: 'var(--f-display)',
    fontStyle: 'italic',
    fontSize: 14,
    width: '100%',
    maxWidth: 340,
    cursor: 'text',
  },
  runBlock: {
    display: 'flex', alignItems: 'center', gap: 0,
    border: '1px solid var(--ink)',
    height: 28,
  },
  runBtn: (active, primary) => ({
    width: 32, height: 26,
    display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
    cursor: 'pointer',
    background: primary ? 'var(--ink)' : 'transparent',
    color: primary ? 'var(--paper)' : active ? 'var(--ink)' : 'var(--ink-3)',
    borderRight: '1px solid var(--ink)',
  }),
  modelChip: {
    display: 'inline-flex', alignItems: 'center', gap: 6,
    fontFamily: 'var(--f-mono)', fontSize: 10.5,
    border: '1px solid var(--rule)',
    padding: '4px 10px',
    color: 'var(--ink-2)',
    background: 'var(--paper-2)',
  },

  // body
  body: {
    display: 'grid',
    gridTemplateColumns: '260px 1fr 360px',
    minHeight: 0,
  },
  left: { borderRight: '1px solid var(--rule)', minHeight: 0, overflow: 'hidden', background: 'var(--paper)' },
  center: { display: 'grid', gridTemplateRows: '1fr 270px', minHeight: 0 },
  centerTop: { borderBottom: '1px solid var(--rule)', minHeight: 0, position: 'relative', overflow: 'hidden' },
  centerBot: { minHeight: 0 },
  right: { borderLeft: '1px solid var(--rule)', minHeight: 0, overflow: 'hidden', background: 'var(--paper)' },

  // status bar
  status: {
    display: 'flex', alignItems: 'center',
    borderTop: '1px solid var(--rule)',
    background: 'var(--paper-2)',
    padding: '0 14px',
    gap: 18,
    fontFamily: 'var(--f-mono)',
    fontSize: 10.5,
    color: 'var(--ink-3)',
  },
  statusRight: { marginLeft: 'auto', display: 'flex', gap: 18 },
  statusDot: (c) => ({ width: 6, height: 6, background: c, display: 'inline-block', marginRight: 6 }),
  scenarioBar: {
    position: 'absolute',
    right: 16, top: 48,
    display: 'flex', gap: 0,
    border: '1px solid var(--rule)',
    background: 'var(--paper)',
    fontFamily: 'var(--f-sans)',
    fontSize: 9.5, letterSpacing: '0.12em', textTransform: 'uppercase',
    zIndex: 40,
  },
  scenarioBtn: (active) => ({
    padding: '5px 10px',
    background: active ? 'var(--ink)' : 'transparent',
    color: active ? 'var(--paper)' : 'var(--ink-3)',
    cursor: 'pointer',
    fontWeight: 500,
    borderLeft: '1px solid var(--rule)',
  }),
  scenarioLabel: {
    padding: '5px 10px',
    color: 'var(--ink-4)',
    fontWeight: 500,
    display: 'inline-flex', alignItems: 'center', gap: 4,
  },
  activityStrip: {
    position: 'absolute',
    left: 16, bottom: 12,
    height: 22,
    display: 'flex', alignItems: 'center',
    background: 'rgba(246,244,239,0.92)',
    border: '1px solid var(--rule)',
    fontFamily: 'var(--f-mono)', fontSize: 10,
    color: 'var(--ink-3)',
    padding: '0 10px',
    gap: 10,
    overflow: 'hidden',
    pointerEvents: 'none',
    maxWidth: 'calc(100% - 220px)',
  },
};

function Editor({ scenario, setScenario, state, dispatch, logo, aiPosition, showActivity }) {
  const selectedEntity = state.entities.find(e => e.id === state.selectedId) || null;
  const isProposalMode = scenario === 'review' || scenario === 'inline';
  const ghostEntities = scenario === 'compose-result' ? [
    { name: 'Player', x: 480, y: 360, w: 32, h: 32, rot: 0 },
    { name: 'Wolf', x: 740, y: 300, w: 56, h: 28, rot: -4, note: '(wolf_ai.lua)' },
  ] : [];
  const proposedComponents = scenario === 'review' ? {
    drone: [ { type: 'Audio', note: 'proposed' } ],
  } : {};

  return (
    <div style={edStyles.shell}>
      <Topbar
        logo={logo}
        onCmdK={() => dispatch({ type: 'open-cmdk' })}
        onCompose={() => dispatch({ type: 'open-compose' })}
      />

      <div style={edStyles.body}>
        <aside style={edStyles.left}>
          <SceneTree
            entities={state.entities}
            selectedId={state.selectedId}
            onSelect={(id) => dispatch({ type: 'select', id })}
            openIds={state.openIds}
            onToggle={(id) => dispatch({ type: 'toggle', id })}
            ghostEntities={ghostEntities}
            proposedComponents={proposedComponents}
            activeTab={state.sceneTab}
            onTabChange={(t) => dispatch({ type: 'sceneTab', t })}
          />
        </aside>

        <main style={edStyles.center}>
          <div style={edStyles.centerTop}>
            <Viewport
              entities={state.entities}
              selectedId={state.selectedId}
              onSelect={(id) => dispatch({ type: 'select', id })}
              ghostEntities={ghostEntities}
              viewTab={state.viewTab}
              onViewTabChange={(t) => dispatch({ type: 'viewTab', t })}
              hoverProposal={scenario === 'review' && state.hoverProposal === 'ch1'}
              isComposing={scenario === 'composing'}
            />
            {/* scenario bar */}
            <div style={edStyles.scenarioBar}>
              <span style={edStyles.scenarioLabel}><IconSparkSmall size={10} stroke="var(--orange)"/>Demo</span>
              {[
                ['rest', 'Rest'],
                ['cmdk', '⌘K'],
                ['composing', 'Compose'],
                ['compose-result', 'Result'],
                ['review', 'Review'],
                ['inline', 'Inline'],
              ].map(([k, lbl]) => (
                <span key={k} style={edStyles.scenarioBtn(scenario === k)} onClick={() => setScenario(k)}>{lbl}</span>
              ))}
            </div>

            {showActivity && (
              <div style={edStyles.activityStrip}>
                <IconSparkSmall size={10} stroke="var(--orange)"/>
                <span style={{ color: 'var(--orange)', letterSpacing: '0.14em', textTransform: 'uppercase', fontSize: 9, fontWeight: 500 }}>AI</span>
                <span style={{ fontFamily: 'var(--f-display)', fontStyle: 'italic', fontSize: 11.5, color: 'var(--ink)' }}>
                  {scenario === 'review' || scenario === 'inline'
                    ? '"make the drone follow the beacon" — 3 changes pending'
                    : scenario === 'composing'
                    ? '"forest clearing at dusk" — composing…'
                    : scenario === 'compose-result'
                    ? 'composed forest scene — 6 entities staged'
                    : 'idle — ⌘K to ask'}
                </span>
              </div>
            )}
          </div>
          <div style={edStyles.centerBot}>
            <ScriptPane
              openFile={state.openFile}
              lines={state.openFile === 'drone.lua' ? window.SINDRI_DATA.DRONE_LUA : window.SINDRI_DATA.BEACON_LUA}
              ghostProposal={scenario === 'review' || scenario === 'inline' ? window.SINDRI_DATA.PROPOSED_DIFF.changes[0] : null}
              onAccept={() => setScenario('rest')}
              onReject={() => setScenario('rest')}
              inlineGhost={scenario === 'inline-ghost' ? [
                'function Drone:onHitBeacon()',
                '  audio.play("chime.ogg")',
                '  particles.burst(self.transform, "ember")',
                'end',
              ] : null}
            />
          </div>
        </main>

        <aside style={edStyles.right}>
          {isProposalMode ? (
            <ProposalsLane
              diff={window.SINDRI_DATA.PROPOSED_DIFF}
              statuses={state.proposalStatuses}
              onSet={(id, v) => dispatch({ type: 'set-proposal', id, v })}
              onAcceptAll={() => dispatch({ type: 'accept-all' })}
              onRejectAll={() => dispatch({ type: 'reject-all' })}
              onHover={(id) => dispatch({ type: 'hover-proposal', id })}
            />
          ) : (
            <Inspector
              entity={selectedEntity}
              onOpenScript={(f) => dispatch({ type: 'open-file', f })}
              onTriggerAI={(k) => k === 'chase' && setScenario('review')}
            />
          )}
        </aside>
      </div>

      <Statusbar selectedId={state.selectedId} />
    </div>
  );
}

function Topbar({ logo, onCmdK, onCompose }) {
  return (
    <header style={edStyles.topbar}>
      <div style={edStyles.topLeft}>
        <LogoLockup which={logo} size={24} />
      </div>
      <div style={edStyles.topCenter}>
        <div style={edStyles.menus}>
          {['File', 'Edit', 'Scene', 'View', 'Build'].map((m) => (
            <span key={m} style={edStyles.menuItem}>{m}</span>
          ))}
        </div>
        <div style={{ ...edStyles.toolGroup, marginLeft: 8 }}>
          <span style={edStyles.toolBtn(true)}  title="Move"><IconMove size={14}/></span>
          <span style={edStyles.toolBtn()}      title="Rotate"><IconRotate size={14}/></span>
          <span style={edStyles.toolBtn()}      title="Scale"><IconScale size={14}/></span>
        </div>
        <div style={edStyles.toolGroup}>
          <span style={edStyles.toolBtn()} title="Undo"><IconUndo size={14}/></span>
          <span style={edStyles.toolBtn()} title="Redo"><IconRedo size={14}/></span>
        </div>
        <div style={{ ...edStyles.toolGroup, marginRight: 'auto' }}>
          <span style={{ ...edStyles.toolBtn(), color: 'var(--ink-2)', padding: '0 8px', width: 'auto', fontSize: 11 }}>Save</span>
        </div>

        <div style={edStyles.cmdK} onClick={onCmdK}>
          <IconSparkle size={14} stroke="var(--orange)"/>
          <span style={{ flex: 1 }}>Ask Sindri to build something…</span>
          <span style={{ fontFamily: 'var(--f-mono)', fontSize: 10, color: 'var(--ink-3)', fontStyle: 'normal' }}>
            <KeyChip>⌘</KeyChip><KeyChip>K</KeyChip>
          </span>
        </div>

        <div style={edStyles.runBlock}>
          <span style={edStyles.runBtn(true, true)}><IconPlay size={12} stroke="var(--paper)"/></span>
          <span style={edStyles.runBtn()}><IconPause size={12} stroke="var(--ink-3)"/></span>
          <span style={{ ...edStyles.runBtn(), borderRight: 'none' }}><IconStop size={12} stroke="var(--ink-3)"/></span>
        </div>
      </div>
      <div style={edStyles.topRight}>
        <span onClick={onCompose} style={{
          fontFamily: 'var(--f-display)', fontStyle: 'italic', fontSize: 14,
          color: 'var(--orange)', cursor: 'pointer',
          display: 'inline-flex', alignItems: 'center', gap: 6,
        }}>
          <IconSparkle size={13} stroke="var(--orange)"/> Compose…
        </span>
        <div style={edStyles.modelChip}>
          <span style={{ width: 6, height: 6, background: 'var(--moss)', display: 'inline-block' }}/>
          <span>qwen3:14b</span>
        </div>
      </div>
    </header>
  );
}

function Statusbar({ selectedId }) {
  return (
    <footer style={edStyles.status}>
      <span><span style={edStyles.statusDot('var(--moss)')}/>engine ready</span>
      <span><span style={edStyles.statusDot('var(--moss)')}/>ollama</span>
      <span><span style={edStyles.statusDot('var(--moss)')}/>localhost:11434</span>
      <span style={{ color: 'var(--ink-4)' }}>·</span>
      <span>scenes/editor_scene.sndr</span>
      <span style={{ color: 'var(--ink-4)' }}>·</span>
      <span>{selectedId ? `selected · ${selectedId}` : 'no selection'}</span>
      <div style={edStyles.statusRight}>
        <span>4 entities</span>
        <span>0 errors</span>
        <span>sindri <span style={{ color: 'var(--ink-4)' }}>v0.1.4</span></span>
      </div>
    </footer>
  );
}

Object.assign(window, { Editor, Topbar, Statusbar });
