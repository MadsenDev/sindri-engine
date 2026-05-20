// App root — composes everything, applies tweaks via CSS custom properties.

function App() {
  const [t, setTweak] = useTweaks(TWEAK_DEFAULTS);

  // Apply tweak-derived styles on documentElement
  React.useEffect(() => {
    const root = document.documentElement;
    // Embers — fade amber-glow channels
    const e = t.embers / 100;
    root.style.setProperty("--amber-glow",   `rgba(240, 192, 80, ${0.12 * e})`);
    root.style.setProperty("--amber-glow-2", `rgba(240, 192, 80, ${0.22 * e})`);

    // Density — scales section padding
    root.dataset.density = t.density;

    // Hex background toggle
    root.dataset.hex = t.showHex ? "on" : "off";
  }, [t.embers, t.density, t.showHex]);

  return (
    <div className="site" data-density={t.density}>
      <Nav />
      <Hero tweaks={t} />
      {t.showSpec && <SpecStrip />}
      <Features />
      {t.showPhilosophy && <Philosophy />}
      {t.showAIFlow && <AIFlow autoLoop={t.autoLoop} />}
      {t.showEditor && <EditorPreview />}
      {t.showCode && <CodeSample />}
      <Download />
      <Footer />
      <SindriTweaks t={t} setTweak={setTweak} />
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root")).render(<App />);
