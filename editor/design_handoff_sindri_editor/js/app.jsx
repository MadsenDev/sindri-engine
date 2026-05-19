// App root — owns scene state and scenario routing.

const { useReducer, useState, useEffect } = React;

const initialState = {
  entities: window.SINDRI_DATA.INITIAL_SCENE.entities,
  selectedId: 'drone',
  openIds: new Set(['drone']),
  openFile: 'drone.lua',
  sceneTab: 'scene',
  viewTab: 'scene',
  proposalStatuses: {},
  hoverProposal: null,
};

function reducer(s, a) {
  switch (a.type) {
    case 'select':       return { ...s, selectedId: a.id };
    case 'toggle': {
      const n = new Set(s.openIds);
      n.has(a.id) ? n.delete(a.id) : n.add(a.id);
      return { ...s, openIds: n };
    }
    case 'open-file':    return { ...s, openFile: a.f };
    case 'sceneTab':     return { ...s, sceneTab: a.t };
    case 'viewTab':      return { ...s, viewTab: a.t };
    case 'set-proposal': return { ...s, proposalStatuses: { ...s.proposalStatuses, [a.id]: a.v } };
    case 'accept-all': {
      const next = {};
      window.SINDRI_DATA.PROPOSED_DIFF.changes.forEach((c) => next[c.id] = 'accept');
      return { ...s, proposalStatuses: next };
    }
    case 'reject-all': {
      const next = {};
      window.SINDRI_DATA.PROPOSED_DIFF.changes.forEach((c) => next[c.id] = 'reject');
      return { ...s, proposalStatuses: next };
    }
    case 'hover-proposal': return { ...s, hoverProposal: a.id };
    case 'open-cmdk':    return { ...s, cmdkOpen: true };
    case 'close-cmdk':   return { ...s, cmdkOpen: false };
    case 'open-compose': return { ...s, composeOpen: true };
    case 'close-compose': return { ...s, composeOpen: false };
    default: return s;
  }
}

function App() {
  const [state, dispatch] = useReducer(reducer, initialState);
  const [scenario, setScenario] = useState('rest');
  const [logo, setLogo] = useState('anvil');
  const [aiPosition, setAiPosition] = useState('right');
  const [showActivity, setShowActivity] = useState(true);

  useEffect(() => {
    const fns = {
      'sindri:logo': (e) => setLogo(e.detail),
      'sindri:aipos': (e) => setAiPosition(e.detail),
      'sindri:activity': (e) => setShowActivity(e.detail),
    };
    Object.entries(fns).forEach(([k, fn]) => window.addEventListener(k, fn));
    return () => Object.entries(fns).forEach(([k, fn]) => window.removeEventListener(k, fn));
  }, []);

  // Scenario side-effects: certain scenarios pre-set state for nice demos
  useEffect(() => {
    if (scenario === 'cmdk') dispatch({ type: 'open-cmdk' });
    else dispatch({ type: 'close-cmdk' });

    if (scenario === 'composing' || scenario === 'compose-result') dispatch({ type: 'open-compose' });
    if (scenario === 'composing') {
      // close after compose-result transition
    }
    if (scenario === 'rest' || scenario === 'review' || scenario === 'inline') dispatch({ type: 'close-compose' });
  }, [scenario]);

  // Auto-progress: composing → compose-result after 1.8s
  useEffect(() => {
    if (scenario !== 'composing') return;
    const t = setTimeout(() => setScenario('compose-result'), 1800);
    return () => clearTimeout(t);
  }, [scenario]);

  // Listen for global ⌘K
  useEffect(() => {
    const handler = (e) => {
      if ((e.metaKey || e.ctrlKey) && (e.key === 'k' || e.key === 'K')) {
        e.preventDefault();
        setScenario('cmdk');
      }
      if (e.key === 'Escape') {
        if (scenario === 'cmdk' || scenario === 'composing' || scenario === 'compose-result') setScenario('rest');
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [scenario]);

  return (
    <div className={'app-shell'} style={{ position: 'relative' }}>
      <Editor
        scenario={scenario}
        setScenario={setScenario}
        state={state}
        dispatch={dispatch}
        logo={logo}
        aiPosition={aiPosition}
        showActivity={showActivity}
      />

      <CommandBar
        open={scenario === 'cmdk'}
        onClose={() => setScenario('rest')}
        prompt="Make the Drone follow the Beacon and chime nearby"
        ctx={[
          { kind: 'scene', text: 'editor_scene' },
          { kind: 'entity', text: '@Drone' },
          { kind: 'entity', text: '@Beacon' },
        ]}
      />

      <SceneComposer
        open={scenario === 'composing'}
        onClose={() => setScenario('rest')}
        onCompose={() => setScenario('composing')}
      />

      <SindriTweaks />
    </div>
  );
}

ReactDOM.createRoot(document.getElementById('root')).render(<App />);
