// Features grid — 4 hard-cornered cards. AI card uses amber accents.

function Features() {
  return (
    <section id="features" className="section features">
      <span className="section-label">
        <span className="dot" />
        <span className="num">02</span> · features
      </span>

      <div className="wrap">
        <div className="section-heading">
          <div className="left">
            <span className="eyebrow muted"><span className="bar" />four moving parts</span>
            <h2>An engine, an editor, a peer.<br />Built so the seams show.</h2>
          </div>
          <div className="right">
            <p>
              Sindri keeps its layers honest. The runtime is fast Rust. Gameplay is plain Lua you can read.
              The editor is the workspace. The AI is a collaborator with full agency — and a paper trail.
            </p>
          </div>
        </div>

        <div className="feature-grid">
          <FeatureCard
            num="01"
            icon={<IconRust size={22} />}
            title="Engine"
            kind=""
            body="A 2D core in Rust. wgpu-backed renderer, Rapier2D physics, batched sprites + tilemaps + particles + lighting + text, audio via rodio, A* pathfinding, undo/redo commands."
            list={["wgpu 28 · winit 0.30", "rapier2d 0.14 physics", "ecs-style world + entityid", "scene serialization"]}
          />
          <FeatureCard
            num="02"
            icon={<IconGrid size={22} />}
            title="Editor"
            body="A Tauri 2 + React workspace that talks to a local Axum engine server on 127.0.0.1:7878. Scene hierarchy, component inspection, script editing, viewport preview, local AI chat."
            list={["tauri 2 + react", "local server · axum 0.7", "scene + component + script", "editor in active dev"]}
          />
          <FeatureCard
            num="03"
            icon={<IconSparkle size={22} />}
            title="AI peer"
            kind="ai"
            body="Local AI via Ollama. The crate sindri-ai defines action types for entity and component manipulation. No external API required. Parity-tracked against the engine so AI tooling doesn't drift from runtime."
            list={["crate · sindri-ai", "ollama · local-first", "entity + component actions", "parity-tracked"]}
          />
          <FeatureCard
            num="04"
            icon={<IconLua size={22} />}
            title="Scripting"
            body="Lua 5.4 via mlua. Entity-attached scripts with on_update / on_fixed_update lifecycles, deferred world mutation, collision and trigger callbacks, hot reload."
            list={["lua 5.4 · mlua 0.9", "on_update · on_fixed_update", "deferred command buffer", "hot reload"]}
          />
        </div>
      </div>
    </section>
  );
}

function FeatureCard({ num, icon, title, body, list, kind = "" }) {
  return (
    <div className={"feature-card " + kind}>
      <span className="num">{num}</span>
      <span className="icon">{icon}</span>
      <h3>
        {title}
        {kind === "ai" && <span className="ember-dot" />}
      </h3>
      <p>{body}</p>
      <ul>
        {list.map(l => <li key={l}>{l}</li>)}
      </ul>
    </div>
  );
}

Object.assign(window, { Features });
