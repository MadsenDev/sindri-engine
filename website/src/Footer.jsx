// Footer — wordmark, brief, link columns, status row.

function Footer() {
  return (
    <footer id="docs" className="footer section">
      <div className="wrap">
        <div className="brand-col">
          <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 14 }}>
            <ForgeMark size={26} />
            <Wordmark size={22} />
          </div>
          <p>
            A 2d game engine, editor workspace, and local ai-assisted tooling stack.
            Built in the open. Apache-2.0. No telemetry. No account.
          </p>
        </div>
        <div>
          <h5>engine</h5>
          <ul>
            <li><a href="#">overview</a></li>
            <li><a href="#">architecture</a></li>
            <li><a href="#">lua api</a></li>
            <li><a href="#">renderer</a></li>
            <li><a href="#">changelog</a></li>
          </ul>
        </div>
        <div>
          <h5>editor</h5>
          <ul>
            <li><a href="#">getting started</a></li>
            <li><a href="#">keymap · ⌘k</a></li>
            <li><a href="#">scene format</a></li>
            <li><a href="#">cli</a></li>
          </ul>
        </div>
        <div>
          <h5>ai peer</h5>
          <ul>
            <li><a href="#">proposal protocol · design</a></li>
            <li><a href="#">ollama setup</a></li>
            <li><a href="#">sindri-ai crate</a></li>
            <li><a href="https://github.com/MadsenDev/sindri-engine" target="_blank" rel="noreferrer">source · github</a></li>
          </ul>
        </div>
      </div>
      <div className="bottom">
        <span>© 2026 madsendev · mit or apache-2.0</span>
        <span>· sindri · main branch · active development</span>
        <span className="spacer" />
        <span className="ai-status"><span className="dot" />ai · ollama · local-first</span>
      </div>
    </footer>
  );
}

Object.assign(window, { Footer });
