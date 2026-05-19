// Tweaks UI — uses the starter <TweaksPanel>.

const TWEAK_DEFAULTS = /*EDITMODE-BEGIN*/{
  "theme": "light",
  "density": "comfortable",
  "logo": "forge",
  "accent": "#d4541e",
  "showActivity": true,
  "aiPosition": "right"
}/*EDITMODE-END*/;

function SindriTweaks() {
  const [t, setTweak] = useTweaks(TWEAK_DEFAULTS);

  // Apply theme as class on app shell
  React.useEffect(() => {
    const root = document.querySelector('.app-shell');
    if (!root) return;
    root.classList.toggle('theme-dark', t.theme === 'dark');
    root.classList.remove('density-compact', 'density-spacious');
    if (t.density === 'compact') root.classList.add('density-compact');
    if (t.density === 'spacious') root.classList.add('density-spacious');
    root.style.setProperty('--orange', t.accent);
  }, [t.theme, t.density, t.accent]);

  // Broadcast logo choice via custom event
  React.useEffect(() => {
    window.dispatchEvent(new CustomEvent('sindri:logo', { detail: t.logo }));
    window.dispatchEvent(new CustomEvent('sindri:aipos', { detail: t.aiPosition }));
    window.dispatchEvent(new CustomEvent('sindri:activity', { detail: t.showActivity }));
  }, [t.logo, t.aiPosition, t.showActivity]);

  return (
    <TweaksPanel title="Tweaks">
      <TweakSection label="Theme">
        <TweakRadio
          label="Mode"
          value={t.theme}
          options={[
            { value: 'light', label: 'Paper' },
            { value: 'dark', label: 'Ink' },
          ]}
          onChange={(v) => setTweak('theme', v)}
        />
        <TweakColor
          label="Accent"
          value={t.accent}
          options={['#d4541e', '#2b4a6f', '#5e6b3a', '#1a1a1a']}
          onChange={(v) => setTweak('accent', v)}
        />
        <TweakRadio
          label="Density"
          value={t.density}
          options={[
            { value: 'compact', label: 'Compact' },
            { value: 'comfortable', label: 'Comfy' },
            { value: 'spacious', label: 'Loose' },
          ]}
          onChange={(v) => setTweak('density', v)}
        />
      </TweakSection>

      <TweakSection label="Logo">
        <TweakRadio
          label="Mark"
          value={t.logo}
          options={[
            { value: 'anvil', label: 'Anvil' },
            { value: 'forge', label: 'Forge' },
            { value: 'rune', label: 'Rune' },
          ]}
          onChange={(v) => setTweak('logo', v)}
        />
      </TweakSection>

      <TweakSection label="Layout">
        <TweakRadio
          label="AI panel"
          value={t.aiPosition}
          options={[
            { value: 'right', label: 'Right' },
            { value: 'left', label: 'Left' },
          ]}
          onChange={(v) => setTweak('aiPosition', v)}
        />
        <TweakToggle
          label="Show activity strip"
          value={t.showActivity}
          onChange={(v) => setTweak('showActivity', v)}
        />
      </TweakSection>
    </TweaksPanel>
  );
}

Object.assign(window, { SindriTweaks });
