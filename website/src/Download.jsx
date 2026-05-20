// Install / Build & run — uses the actual cargo commands from the repo README.

function Download() {
  return (
    <section id="install" className="download section">
      <span className="section-label">
        <span className="dot" />
        <span className="num">07</span> · get sindri · build &amp; run
      </span>

      <div className="wrap">
        <div className="left">
          <h2>
            Build<span className="ember">.</span><br />
            Locally.<br />
            <span style={{color: "var(--ink-3)"}}>cargo · mit or apache-2.0</span>
          </h2>
          <p>
            Sindri is source-only. Clone the repo, build the workspace with cargo, and run any of the curated
            examples. The editor is a separate Tauri 2 + React app that talks to a local engine server.
            AI features need a local Ollama install.
          </p>
          <div className="links">
            <a className="btn ink" href="https://github.com/MadsenDev/sindri-engine" target="_blank" rel="noreferrer">
              <IconGithub size={14}/>
              Clone on GitHub
            </a>
            <a className="btn" href="https://github.com/MadsenDev/sindri-engine#quick-start" target="_blank" rel="noreferrer">
              <IconBook size={13}/>
              Quick start
            </a>
          </div>
        </div>

        <div className="right">
          <div className="install-card">
            <div className="head">
              <span>quickstart</span>
              <span className="v">cargo · pnpm · ollama</span>
            </div>

            <div className="step">
              <div className="step-meta">
                <span className="n">01</span>
                <span className="t">clone</span>
                <span className="d">~10 s</span>
              </div>
              <pre className="shell"><span className="prompt">$</span> git clone https://github.com/MadsenDev/sindri-engine.git</pre>
            </div>

            <div className="step">
              <div className="step-meta">
                <span className="n">02</span>
                <span className="t">build the workspace</span>
                <span className="d">crates · sindri · sindri-server · sindri-ai</span>
              </div>
              <pre className="shell"><span className="prompt">$</span> cargo build --workspace</pre>
            </div>

            <div className="step">
              <div className="step-meta">
                <span className="n">03</span>
                <span className="t">run an example</span>
              <span className="d">choose one below</span>
              </div>
              <pre className="shell"><span className="prompt">$</span> cargo run -p hello_sindri</pre>
            </div>

            <div className="step">
              <div className="step-meta">
                <span className="n">04</span>
                <span className="t">editor &amp; server (optional)</span>
                <span className="d">tauri 2 + react · axum on 127.0.0.1:7878</span>
              </div>
              <pre className="shell">
                <span className="prompt">$</span> cargo run -p sindri-server{"\n"}
                <span className="prompt">$</span> cd editor &amp;&amp; pnpm install &amp;&amp; pnpm tauri dev
              </pre>
            </div>
          </div>

          <div className="examples-card">
            <div className="head">
              <span>curated examples</span>
              <span className="v">examples/</span>
            </div>
            <ExampleRow name="hello_sindri"       purpose="minimal onboarding" />
            <ExampleRow name="platformer"         purpose="rust-first gameplay + physics" />
            <ExampleRow name="scripted_asteroids" purpose="lua scripting + hot reload" />
            <ExampleRow name="editor_scene"       purpose="scene loading + editor workflow" />
            <ExampleRow name="rendering"          purpose="particles, lighting, camera, object counts" />
          </div>
        </div>
      </div>
    </section>
  );
}

function ExampleRow({ name, purpose }) {
  return (
    <div className="example-row">
      <span className="ex-name">{name}</span>
      <span className="ex-purpose">{purpose}</span>
      <span className="ex-cmd">cargo run -p {name}</span>
    </div>
  );
}

Object.assign(window, { Download });
