// Top navigation — modeled after the editor topbar (sticky, hairline, sparse).

function Nav() {
  return (
    <nav className="nav">
      <div className="nav-inner">
        <a className="nav-brand" href="#top">
          <ForgeMark size={22} />
          <Wordmark size={16} />
        </a>
        <div className="nav-links">
          <a href="#features">features</a>
          <a href="#philosophy">philosophy</a>
          <a href="#ai">ai flow</a>
          <a href="#editor">editor</a>
          <a href="#code">scripting</a>
          <a href="#docs">docs</a>
        </div>
        <div className="nav-spacer" />
        <div className="nav-meta">
          <span><span className="status-dot" />github · main · active dev</span>
        </div>
        <a className="nav-cta" href="#install">
          <IconDownload size={13} />
          Get Sindri
          <span className="meta">⌘D</span>
        </a>
      </div>
    </nav>
  );
}

Object.assign(window, { Nav });
