// Editor preview — scaled-down recreation of the editor shell.

function EditorPreview() {
  const [sel, setSel] = React.useState("2");
  const entities = [
    { id: "5",  name: "Main Camera", kind: "Camera", swatch: "cyan", indent: 0 },
    { id: "3",  name: "Ground",      kind: "Sprite · Collider", swatch: "stone", indent: 0 },
    { id: "2",  name: "Drone",       kind: "Sprite · Script · Collider", swatch: "orange", indent: 0 },
    { id: "1",  name: "Beacon",      kind: "Sprite · Script · Collider", swatch: "cyan", indent: 0 },
    { id: "11", name: "Player",      kind: "PhysicsBody · Script", swatch: "moss", indent: 0 },
    { id: "12", name: "Sprite",      kind: "Sprite · Collider", swatch: "cyan", indent: 0 },
    { id: "fix", name: "Collider size mismatch", kind: "suggest_fix · proposed", swatch: "ghost", ghost: true, indent: 1 },
  ];

  return (
    <section id="editor" className="editor-preview section">
      <span className="section-label">
        <span className="dot" />
        <span className="num">05</span> · editor · the workspace
      </span>

      <div className="wrap">
        <div className="section-heading">
          <div className="left">
            <span className="eyebrow muted"><span className="bar" />live editor shell · current direction</span>
            <h2>Scene · viewport · inspector.<br />Everything you need, nothing you don't.</h2>
          </div>
          <div className="right">
            <p>
              The actual Tauri editor is the visual baseline: compact hierarchy, large grid viewport, inspector,
              script editor, and status chrome in one fixed workspace. This preview mirrors that real workflow with
              repo-grounded scene data.
            </p>
          </div>
        </div>

        <div className="design-target-banner">
          <span className="dot-amber" />
          <span className="label">actual editor reference</span>
          <span>· based on the working <code>editor/</code> app and <code>examples/editor_scene</code>, with repo-grounded scene and script details.</span>
        </div>

        <div className="editor-shell">
          <EditorTopbar />
          <div className="editor-body">
            <div className="editor-pane">
              <div className="pane-head">
                scene
                <span className="spacer" />
                <span className="count">6</span>
                <IconPlus size={11} style={{ color: "var(--ink-4)" }}/>
              </div>
              <div className="scene-tree">
                {entities.map(e => (
                  <div key={e.id}
                    className={"scene-row" + (sel === e.id ? " sel" : "") + (e.ghost ? " ghost" : "")}
                    onClick={() => !e.ghost && setSel(e.id)}>
                    {e.indent > 0 && <span className="indent" />}
                    <span className={"swatch " + (e.swatch === "ghost" ? "" : e.swatch)}
                      style={e.swatch === "ghost" ? { background: "var(--amber)" } : null} />
                    <span className="name">{e.name}</span>
                    <span className="kind">{e.kind}</span>
                  </div>
                ))}
              </div>
              <div className="pane-head" style={{ borderTop: "1px solid var(--rule)", borderBottom: 0 }}>
                assets
                <span className="spacer" />
                <span className="count">4</span>
              </div>
              <div style={{ padding: "6px 12px 14px", fontFamily: "var(--font-mono)", fontSize: "var(--t-11)", color: "var(--ink-3)", letterSpacing: "var(--track-mid)", display: "flex", flexDirection: "column", gap: 3 }}>
                <span><IconFolder size={11}/> sprites/</span>
                <span style={{ marginLeft: 14 }}>generated:white</span>
                <span><IconFolder size={11}/> scripts/</span>
                <span style={{ marginLeft: 14 }}>drone.lua</span>
                <span style={{ marginLeft: 14 }}>player.lua</span>
                <span style={{ marginLeft: 14 }}>beacon.lua</span>
              </div>
            </div>

            <div className="editor-pane">
              <div className="pane-head" style={{ display: "flex", gap: 16 }}>
                <span style={{ color: "var(--ink)" }}>editor_scene.sindri</span>
                <span style={{ color: "var(--ink-4)" }}>drone.lua</span>
                <span style={{ color: "var(--ink-4)" }}>player.lua</span>
                <span className="spacer" />
                <span className="count">1280 × 720 · 60fps</span>
              </div>
              <div className="editor-vp" style={{ flex: 1 }}>
                <div className="grid-bg" />
                <div className="camera-frame" />
                {/* drone */}
                <div className="vp-entity" style={{ left: "62%", top: 70, width: 34, height: 34, background: "rgba(212,84,30,0.12)", border: "1px solid var(--orange)" }}>
                  <span className="lbl">drone</span>
                </div>
                {/* beacon */}
                <div className="vp-entity" style={{ left: "28%", top: 220, width: 14, height: 14, background: "var(--cyan-glow)", border: "1px solid var(--cyan)" }}>
                  <span className="lbl">beacon</span>
                </div>
                {/* ghost - proposed review card */}
                <div className="vp-entity" style={{
                  left: "60%", top: 60,
                  width: 100, height: 56,
                  border: "1.5px dashed var(--amber)",
                  background: "var(--amber-glow)",
                  animation: "sk-ghost-fade 1.8s ease-in-out infinite",
                }}>
                  <span className="lbl" style={{ color: "var(--amber)" }}>suggest_fix · collider 147×145</span>
                </div>
                {/* selection marker around drone */}
                <div style={{
                  position: "absolute", left: "calc(62% - 6px)", top: 64,
                  width: 46, height: 46,
                  border: "1px solid var(--ink)",
                  pointerEvents: "none",
                }}>
                  {/* corner ticks */}
                  {[[0,0],[1,0],[0,1],[1,1]].map(([x,y]) => (
                    <span key={`${x}${y}`} style={{
                      position: "absolute",
                      left: x ? "auto" : -3, right: x ? -3 : "auto",
                      top: y ? "auto" : -3, bottom: y ? -3 : "auto",
                      width: 5, height: 5, background: "var(--ink)",
                    }} />
                  ))}
                </div>
              </div>
            </div>

            <div className="editor-pane right">
              <div className="pane-head">
                inspector
                <span className="spacer" />
                <span className="count">Drone · #2</span>
              </div>
              <div className="inspector">
                <div className="insp-section">
                  <div className="insp-head">
                    <IconTransform size={11} /> transform
                  </div>
                  <div className="insp-row"><span className="k">x</span><span className="v">600.0</span></div>
                  <div className="insp-row"><span className="k">y</span><span className="v">300.0</span></div>
                  <div className="insp-row"><span className="k">rot</span><span className="v">1.57</span></div>
                  <div className="insp-row"><span className="k">scale</span><span className="v">4.059969 × 4.059969</span></div>
                </div>
                <div className="insp-section">
                  <div className="insp-head">
                    <IconSprite size={11} /> sprite
                  </div>
                  <div className="insp-row"><span className="k">texture</span><span className="v">generated:white</span></div>
                  <div className="insp-row"><span className="k">size</span><span className="v">36 × 36</span></div>
                  <div className="insp-row"><span className="k">color</span><span className="v">[0.34, 0.72, 1.0, 1.0]</span></div>
                </div>
                <div className="insp-section">
                  <div className="insp-head">
                    <IconScript size={11} /> script
                  </div>
                  <div className="insp-row"><span className="k">path</span><span className="v">scripts/drone.lua</span></div>
                </div>
                <div className="insp-section">
                  <div className="insp-head">
                    <IconTransform size={11} /> collider
                  </div>
                  <div className="insp-row"><span className="k">width</span><span className="v">147.0</span></div>
                  <div className="insp-row"><span className="k">height</span><span className="v">145.0</span></div>
                  <div className="insp-row"><span className="k">trigger</span><span className="v">false</span></div>
                </div>
                <div className="insp-section" style={{ background: "var(--amber-glow)", borderTop: "1px solid var(--amber-dim)" }}>
                  <div className="insp-head" style={{ color: "var(--amber)" }}>
                    <IconSparkSmall size={11} /> proposed
                    <span className="spacer" />
                    <span className="badge">+1</span>
                  </div>
                  <div className="insp-row amber"><span className="k">action</span><span className="v">suggest_fix</span></div>
                  <div className="insp-row amber"><span className="k">entity</span><span className="v">#2 · Drone</span></div>
                </div>
              </div>
            </div>
          </div>
          <EditorStatusBar />
        </div>
      </div>
    </section>
  );
}

function EditorTopbar() {
  return (
    <div className="editor-topbar">
      <div className="brand">
        <ForgeMark size={18} />
        <Wordmark size={13} />
      </div>
      <div className="crumbs">
        examples <span className="sep">/</span> editor_scene <span className="sep">/</span> scenes/editor_scene.sindri
      </div>
      <div className="play">
        <button title="play"><IconPlay size={12}/></button>
        <button title="pause"><IconPause size={12}/></button>
        <button title="stop"><IconStop size={12}/></button>
      </div>
      <div className="cmdk">
        <IconSparkSmall size={12} className="spark" style={{ color: "var(--amber)" }} />
        <span style={{ color: "var(--ink-4)" }}>ask sindri to build something…</span>
        <span className="keys"><kbd>⌘</kbd><kbd>K</kbd></span>
      </div>
      <div className="status">
        <span><span className="ok">●</span> ready</span>
        <span><span className="ai">●</span> ollama · localhost:11434</span>
      </div>
    </div>
  );
}

function EditorStatusBar() {
  return (
    <div className="editor-status">
      <span><span className="dot moss"/>engine · running</span>
      <span>6 entities · 3 scripts · 0 errors</span>
      <span className="spacer" />
      <span><span className="dot amber" />ai · ollama · localhost:11434</span>
      <span>sindri · main</span>
    </div>
  );
}

Object.assign(window, { EditorPreview });
