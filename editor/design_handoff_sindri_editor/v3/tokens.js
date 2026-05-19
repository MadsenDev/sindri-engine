// v3 tokens — dark, retro, pixel.
// Palette echoes the original Sindri screenshot: very dark navy-black,
// warm amber forge accent, cool cyan for entities, sage moss for "good" state.

const PAPER  = '#0d1117';   // editor background (very dark)
const PAPER2 = '#161b22';   // elevated panel, headers, modal scrim base
const PAPER3 = '#1e2530';   // hover, selection background
const INK    = '#e6e1d4';   // primary text — warm off-white
const INK2   = '#c4beae';
const INK3   = '#8a8580';
const INK4   = '#5a554e';
const RULE   = '#1f242c';   // soft default rule
const RULE2  = '#2a3038';   // major region rule
const ORANGE = '#f0c050';   // amber forge ember
const NAVY   = '#6dbcdb';   // cyan entity accent (renamed conceptually but kept var name)
const MOSS   = '#9bb070';   // sage

// Pixel-feel typography. Pixelify Sans is the primary face — modern pixel,
// readable at small sizes. JetBrains Mono kept for code (more legible than
// pixel monos at long lines).
const F_DISPLAY = '"Pixelify Sans", "VT323", monospace';
const F_SANS    = '"Pixelify Sans", "VT323", monospace';
const F_MONO    = '"JetBrains Mono", "VT323", monospace';

window.V3 = {
  PAPER, PAPER2, PAPER3, INK, INK2, INK3, INK4, RULE, RULE2, ORANGE, NAVY, MOSS,
  F_DISPLAY, F_SANS, F_MONO,
};
