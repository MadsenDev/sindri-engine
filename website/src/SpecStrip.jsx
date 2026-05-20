// Spec strip — terminal-style "system info" between hero and features.

function SpecStrip() {
  const cells = [
    { k: "engine",     v: "rust 2021 · wgpu 28" },
    { k: "scripting",  v: "lua 5.4 · mlua · hot reload" },
    { k: "physics",    v: "rapier2d 0.14" },
    { k: "server",     v: "axum · 127.0.0.1:7878" },
    { k: "ai · editor", v: "ollama · tauri 2 + react" },
  ];
  return (
    <div className="spec-strip">
      <div className="wrap">
        {cells.map(c => (
          <div className="cell" key={c.k}>
            <span className="k">{c.k}</span>
            <span className="v">{c.v}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

Object.assign(window, { SpecStrip });
