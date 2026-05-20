// Philosophy block — the three-line manifesto from the design system, in display type.

function Philosophy() {
  return (
    <section id="philosophy" className="philosophy section">
      <span className="section-label">
        <span className="dot" />
        <span className="num">03</span> · philosophy
      </span>

      <div className="wrap">
        <div className="left">
          <span className="eyebrow"><span className="bar" />on layering</span>
        </div>
        <div className="right">
          <div className="quote">
            <span className="line"><span className="dim">rust owns</span> the engine.</span>
            <span className="line"><span className="dim">lua owns</span> gameplay behavior.</span>
            <span className="line">the editor and <span className="amber">ai</span> understand both.</span>
          </div>
          <p className="footnote">
            We don't believe in single-language stacks for games. Rust is the right answer for the parts you only write
            once and lean on forever. Lua is the right answer for the parts you rewrite every afternoon.
            The <span className="amber">ai</span> tooling is designed to read both — your scripts, your scenes,
            your engine config — and is being built so every change lands as a proposal you sign off on,
            not an edit that just happens. The engine is functional today; the editor and ai surfaces are in active development.
          </p>
        </div>
      </div>
    </section>
  );
}

Object.assign(window, { Philosophy });
