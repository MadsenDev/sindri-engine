// Shared utilities.

// useInView — observe an element and return whether it's in view (sticky once seen)
function useInView(ref, { threshold = 0.3, once = true } = {}) {
  const [inView, setInView] = React.useState(false);
  React.useEffect(() => {
    if (!ref.current) return;
    const obs = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setInView(true);
          if (once) obs.disconnect();
        } else if (!once) {
          setInView(false);
        }
      },
      { threshold }
    );
    obs.observe(ref.current);
    return () => obs.disconnect();
  }, []);
  return inView;
}

// useTyped — types out a string char-by-char while `active` is true.
function useTyped(target, { active, speed = 28, startDelay = 200 } = {}) {
  const [text, setText] = React.useState("");
  React.useEffect(() => {
    if (!active) { setText(""); return; }
    let i = 0;
    let id;
    const start = setTimeout(() => {
      const tick = () => {
        if (i <= target.length) {
          setText(target.slice(0, i));
          i += 1;
          id = setTimeout(tick, speed);
        }
      };
      tick();
    }, startDelay);
    return () => { clearTimeout(start); clearTimeout(id); };
  }, [active, target]);
  return text;
}

// SindriWordmark — small reusable
function Wordmark({ size = 18 }) {
  return (
    <span className="wordmark" style={{ fontSize: size }}>
      S<span className="i">I</span>NDRI
    </span>
  );
}

// ForgeMark — inline svg of the ember hex
function ForgeMark({ size = 22 }) {
  return (
    <svg width={size} height={size} viewBox="0 0 32 32" fill="none" style={{ display: "block" }}>
      <polygon points="16,2 28,9 28,23 16,30 4,23 4,9" stroke="#e6e1d4" strokeWidth="1.2" fill="none"/>
      <polygon points="16,7 24,11.5 24,20.5 16,25 8,20.5 8,11.5" stroke="#e6e1d4" strokeWidth="1" fill="none"/>
      <polygon points="16,11 20.5,13.5 20.5,18.5 16,21 11.5,18.5 11.5,13.5" fill="#f0c050"/>
    </svg>
  );
}

Object.assign(window, { useInView, useTyped, Wordmark, ForgeMark });
