// Sindri-themed tweak controls wrapped around the starter tweaks panel.

const TWEAK_DEFAULTS = /*EDITMODE-BEGIN*/{
  "embers": 100,
  "density": "standard",
  "showSpec": true,
  "showPhilosophy": true,
  "showAIFlow": true,
  "showEditor": true,
  "showCode": true,
  "autoLoop": true,
  "showHex": true
}/*EDITMODE-END*/;

function SindriTweaks({ t, setTweak }) {
  return (
    <TweaksPanel title="Tweaks · sindri.site">
      <TweakSection title="Visuals">
        <TweakSlider
          label="ember intensity"
          value={t.embers} min={0} max={100} step={5}
          onChange={(v) => setTweak("embers", v)}
          format={(v) => `${v}%`}
        />
        <TweakRadio
          label="density"
          value={t.density}
          options={[{ value: "compact", label: "compact" }, { value: "standard", label: "standard" }, { value: "cozy", label: "cozy" }]}
          onChange={(v) => setTweak("density", v)}
        />
        <TweakToggle
          label="hex pit background"
          value={t.showHex}
          onChange={(v) => setTweak("showHex", v)}
        />
        <TweakToggle
          label="ai flow auto-loop"
          value={t.autoLoop}
          onChange={(v) => setTweak("autoLoop", v)}
        />
      </TweakSection>

      <TweakSection title="Sections">
        <TweakToggle label="spec strip"     value={t.showSpec}       onChange={(v) => setTweak("showSpec", v)} />
        <TweakToggle label="philosophy"     value={t.showPhilosophy} onChange={(v) => setTweak("showPhilosophy", v)} />
        <TweakToggle label="ai proposal flow" value={t.showAIFlow}   onChange={(v) => setTweak("showAIFlow", v)} />
        <TweakToggle label="editor preview" value={t.showEditor}     onChange={(v) => setTweak("showEditor", v)} />
        <TweakToggle label="code sample"    value={t.showCode}       onChange={(v) => setTweak("showCode", v)} />
      </TweakSection>
    </TweaksPanel>
  );
}

Object.assign(window, { SindriTweaks, TWEAK_DEFAULTS });
