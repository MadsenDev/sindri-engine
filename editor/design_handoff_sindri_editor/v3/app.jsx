// v2 App root.
const { useReducer, useState, useEffect } = React;

const v3InitialState = {
  entities: window.SINDRI_DATA.INITIAL_SCENE.entities,
  selectedId: 'drone',
  openIds: new Set(['drone']),
  openFile: 'drone.lua',
  sceneTab: 'scene',
  viewTab: 'scene',
  proposalStatuses: {},
  hoverProposal: null,
};

function v3Reducer(s, a) {
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
    default: return s;
  }
}

function V2App() {
  const [state, dispatch] = useReducer(v3Reducer, v3InitialState);
  const [scenario, setScenario] = useState('rest');
  const [logo, setLogo] = useState('forge');

  useEffect(() => {
    const onLogo = (e) => setLogo(e.detail);
    window.addEventListener('sindri:logo', onLogo);
    return () => window.removeEventListener('sindri:logo', onLogo);
  }, []);

  // Compose modal: scenario 'composing' opens the modal in input state.
  // Modal's inner Compose button triggers the actual thinking → result transition.
  // No auto-transition on scenario change.

  // ⌘K and Escape
  useEffect(() => {
    const h = (e) => {
      if ((e.metaKey || e.ctrlKey) && (e.key === 'k' || e.key === 'K')) {
        e.preventDefault();
        setScenario('cmdk');
      }
      if (e.key === 'Escape') {
        if (['cmdk', 'composing', 'compose-result'].includes(scenario)) setScenario('rest');
      }
    };
    window.addEventListener('keydown', h);
    return () => window.removeEventListener('keydown', h);
  }, [scenario]);

  // Inner-modal Compose click: show wash briefly, then surface ghosts.
  const [composing, setComposing] = useState(false);
  const onCompose = () => {
    setComposing(true);
    setTimeout(() => {
      setComposing(false);
      setScenario('compose-result');
    }, 1600);
  };

  return (
    <div className="app-shell" style={{ position: 'relative' }}>
      <V2Editor scenario={scenario} setScenario={setScenario} state={state} dispatch={dispatch} logo={logo}/>
      <V2CommandBar open={scenario === 'cmdk'} onClose={() => setScenario('rest')}
        prompt="Make the Drone follow the Beacon and chime nearby"
        ctx={[
          { kind: 'scene', text: 'editor_scene' },
          { kind: 'entity', text: '@Drone' },
          { kind: 'entity', text: '@Beacon' },
        ]}/>
      <V2SceneComposer open={scenario === 'composing'} onClose={() => setScenario('rest')}
        onCompose={onCompose} isComposing={composing}/>
      <SindriTweaks/>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById('root')).render(<V2App/>);
