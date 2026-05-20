// Hero — oversized wordmark + tagline + buttons on the left,
// a Cmd-K composer that "composes" on hover/in-view on the right.

const COMPOSER_PROMPT = "tighten the drone orbit and flag any collider mismatch.";

function Hero({ tweaks }) {
  const ref = React.useRef(null);
  const seen = useInView(ref, { threshold: 0.2 });
  const [hover, setHover] = React.useState(false);
  const active = seen || hover;

  // typed-in prompt
  const typed = useTyped(COMPOSER_PROMPT, { active, speed: 26, startDelay: 380 });
  const done = typed === COMPOSER_PROMPT;

  // After typing completes, advance through "composing" → suggestion-active states
  const [stage, setStage] = React.useState(0); // 0 typing, 1 composing, 2 suggestion landed
  React.useEffect(() => {
    if (!done) { setStage(0); return; }
    const t1 = setTimeout(() => setStage(1), 600);
    const t2 = setTimeout(() => setStage(2), 2400);
    return () => { clearTimeout(t1); clearTimeout(t2); };
  }, [done]);

  return (
    <section id="top" ref={ref} className="hero section"
      onMouseEnter={() => setHover(true)}>
      <span className="section-label">
        <span className="dot" />
        <span className="num">01</span> · landing
      </span>

      {/* faint hex pit background mark */}
      <ForgeBackground />

      <div className="wrap hero-grid">
        <div className="hero-left">
          <div className="hero-meta">
            <span className="pill"><span className="ember" />ai-as-peer · local-first · active dev</span>
            <span>·</span>
            <span>rust · lua · wgpu · rapier2d</span>
          </div>

          <h1 className="hero-title">
            <span className="stack">forge games</span>
            <span className="stack">with a peer,</span>
            <span className="stack">not a chatbot<span className="ember-i">.</span></span>
          </h1>

          <p className="hero-sub">
            Sindri is a 2d game engine and editor where a local <span className="amber-word">ai assistant</span>
            {" "}lives inside the workspace — proposing scenes, scripts, and components as reviewable diffs.
            You stay in control. Every change lands as a proposal.
          </p>

          <div className="hero-actions">
            <a className="btn ink" href="#install">
              <IconDownload size={14} />
              Build from source
              <span className="meta">cargo</span>
            </a>
            <a className="btn" href="#editor">
              <IconBook size={13} />
              Read the editor tour
            </a>
            <a className="btn ghost" href="https://github.com/MadsenDev/sindri-engine" target="_blank" rel="noreferrer">
              <IconGithub size={13} />
              View source
            </a>
          </div>

          <div className="hero-platforms">
            <span className="ok">●</span> macos · linux · windows · build with <code style={{ color: "var(--ink-3)" }}>cargo build --workspace</code> · mit or apache-2.0
          </div>
        </div>

        <div className="hero-right">
          <ComposerCard typed={typed} done={done} stage={stage} />
          <MiniViewport stage={stage} />
        </div>
      </div>
    </section>
  );
}

function ForgeBackground() {
  // Hex tessellation, very faint. Two layers — outer ring fades to nothing.
  const cells = [];
  const cols = 14, rows = 8;
  const r = 38; // hex radius (flat-top? we'll use pointy)
  const w = r * Math.sqrt(3);
  const h = r * 2;
  for (let y = 0; y < rows; y++) {
    for (let x = 0; x < cols; x++) {
      const cx = x * w + (y % 2 ? w / 2 : 0);
      const cy = y * h * 0.75;
      const dx = cx - (cols * w) / 2;
      const dy = cy - (rows * h * 0.75) / 2;
      const dist = Math.sqrt(dx*dx + dy*dy);
      const norm = Math.min(1, dist / 240);
      const op = (1 - norm) * 0.18;
      cells.push({ cx, cy, op, key: `${x}-${y}` });
    }
  }
  const hexPath = (cx, cy) => {
    const pts = [];
    for (let i = 0; i < 6; i++) {
      const a = (Math.PI / 3) * i - Math.PI / 2;
      pts.push(`${(cx + r * Math.cos(a)).toFixed(2)},${(cy + r * Math.sin(a)).toFixed(2)}`);
    }
    return pts.join(" ");
  };
  return (
    <svg className="forge-bg" viewBox="0 0 1200 600" preserveAspectRatio="xMidYMid slice">
      <g stroke="#e6e1d4" fill="none" strokeWidth="0.6">
        {cells.map(c => (
          <polygon key={c.key}
            points={hexPath(c.cx + 600 - (cols * w)/2, c.cy + 300 - (rows * h * 0.75)/2)}
            style={{ opacity: c.op }} />
        ))}
      </g>
      {/* one warm ember somewhere off-center */}
      <g transform="translate(820, 220)">
        <polygon points={hexPath(0, 0)} fill="#f0c050" opacity="0.06"/>
        <polygon points={hexPath(0, 0)} stroke="#f0c050" fill="none" strokeWidth="0.8" opacity="0.35"/>
      </g>
    </svg>
  );
}

function ComposerCard({ typed, done, stage }) {
  const showCaret = !done || stage === 0;
  return (
    <div className="composer-card">
      <div className="head">
        <IconSparkle size={12} className="ember" /> <span className="ember">ai composer</span>
        <span className="spacer" />
        <span className="keys"><kbd>⌘</kbd><kbd>K</kbd></span>
      </div>
      <div className="composer-input">
        <span className="spark"><IconSparkSmall size={14} /></span>
        <div className="body">
          {typed}
          {showCaret && <span className="caret" />}
          {done && stage >= 1 && !typed.endsWith(".") && null}
          {done && stage === 0 && null}
        </div>
      </div>

      <div className="composer-suggestions">
        {stage < 1 && (
          <div className="row" style={{ color: "var(--ink-4)" }}>
            <span className="left"><IconChevronR size={10} /></span>
            sindri will propose changes you can review row-by-row.
          </div>
        )}
        {stage >= 1 && (
          <React.Fragment>
            <div className={"row" + (stage === 1 ? " composing" : " active")}>
              <span className="left"><IconSparkSmall size={12} /></span>
              update the orbit radius in drone.lua
              <span className="kind">{stage === 1 ? "composing…" : "write_script · scripts/drone.lua"}</span>
            </div>
            <div className={"row" + (stage >= 2 ? " active" : "")}>
              <span className="left"><IconSparkSmall size={12} /></span>
              set Drone scale back to 1.0 × 1.0
              <span className="kind">{stage >= 2 ? "edit_transform · entity #2" : "queued"}</span>
            </div>
            <div className="row">
              <span className="left"><IconSparkSmall size={12} /></span>
              note collider 147 × 145 vs sprite 36 × 36
              <span className="kind">suggest_fix · entity #2</span>
            </div>
          </React.Fragment>
        )}
      </div>

      <div className="composer-foot">
        <span>local · ollama · 127.0.0.1:11434</span>
        <span className="spacer" />
        {stage >= 1 ? (
          <span className="ai-tag"><span className="ember-dot" />composing 3 changes</span>
        ) : (
          <span className="ai-tag"><span className="ember-dot" />ready</span>
        )}
      </div>
    </div>
  );
}

function MiniViewport({ stage }) {
  return (
    <div className="mini-vp">
      <div className="grid-bg" />
      <div className="vp-meta"><span className="ok">●</span> editor_scene · 4 entities</div>
      {/* beacon */}
      <div className="entity" style={{ left: 30, top: 110, width: 14, height: 14, background: "rgba(109,188,219,0.12)" }}>
        <span className="label" style={{ left: 0, bottom: -16 }}>beacon</span>
      </div>
      {/* drone */}
      <div className="entity" style={{ left: 260, top: 60, background: "rgba(212,84,30,0.12)", borderColor: "var(--orange)" }}>
        <span className="label" style={{ left: 0, bottom: -16 }}>drone</span>
      </div>
      {/* ghost trail (proposed follow path) */}
      {stage >= 1 && (
        <React.Fragment>
          <div className="entity ghost" style={{ left: 200, top: 80, width: 22, height: 22 }} />
          <div className="entity ghost" style={{ left: 140, top: 100, width: 18, height: 18, animationDelay: "0.4s" }} />
          <div className="entity ghost" style={{ left: 90, top: 110, width: 14, height: 14, animationDelay: "0.8s" }} />
          <span className="label amber" style={{ left: 90, top: 130 }}>+ tighter orbit · proposed</span>
        </React.Fragment>
      )}
    </div>
  );
}

Object.assign(window, { Hero });
