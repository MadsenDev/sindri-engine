# Handoff — Sindri Editor

## Overview

Sindri is a 2D game engine with a heavy AI focus. This handoff documents a redesign of the editor that treats the local AI assistant as a peer collaborator with full agency over the project (entities, components, scripts, assets) — not a chat sidebar. Every AI action lands as a reviewable proposal so the user stays in control.

The redesign keeps the existing terminal-leaning, dark + pixel aesthetic of the current Sindri build, sharpened up with a tighter color system and a consistent type stack.

---

## About the design files

The files in this bundle are **design references created in HTML/JSX** — high-fidelity prototypes showing intended look and behaviour. They are not production code to copy verbatim. The task is to **recreate these designs in the Sindri Editor's existing environment**, using its established UI patterns and any framework or rendering layer the engine editor is built in (Tauri/webview, native, Dear ImGui, custom — whichever you already have).

If the editor is already React/HTML-based, the JSX in `v3/*.jsx` is close to drop-in-able but should be refactored to match your existing conventions (component split, state management, theming).

The prototypes use placeholder data (a "patrol drone" scene) and an Ollama `qwen3:14b` status badge. Replace with real wiring.

---

## Fidelity

**High-fidelity.** Exact colors, typography, spacing, and interaction states are specified. Implement pixel-perfect against the spec below.

---

## Files in this bundle

```
Sindri Editor.html              ← main prototype, entry point
Sindri Features Brief.html      ← written feature summary with embedded screenshots
Sindri Logo Options.html        ← 3 logo concepts (Anvil / Forge / Rune) on a design canvas

v3/                             ← v3 (recommended) design — dark + pixel
  tokens.js                       palette + font tokens (V3 namespace)
  logo-v3.jsx                     uppercase pixel wordmark + lockup override
  scene-tree.jsx                  left pane: entity tree, files, history
  viewport.jsx                    center: scene render + grid
  inspector.jsx                   right pane: entity fields + AI suggestions
  script-pane.jsx                 bottom dock: script editor + diff
  command-bar.jsx                 ⌘K natural-language palette
  ai-flows.jsx                    proposal review lane + scene composer modal
  editor.jsx                      top-level editor layout
  app.jsx                         root + state + scenario routing

js/                             ← shared modules
  data.js                         placeholder scene + AI flow data
  icons.jsx                       hairline icon set (Transform, Sprite, Script, etc.)
  logo.jsx                        3 logo marks + wordmark
  tweaks.jsx                      Tweaks panel (theme/logo/density switching)

tweaks-panel.jsx                ← starter component used by tweaks.jsx
design-canvas.jsx               ← starter component used by Logo Options

assets/
  sindri-forge-light.svg          chosen logo, ink+amber (paper background)
  sindri-forge-dark.svg           chosen logo, cream+amber (dark background)

screens/                        ← reference screenshots used in the brief
  brief-rest.png
  brief-cmdk.png
  brief-compose.png
  brief-result.png
  brief-review.png
  brief-inline.png
```

To open the prototype locally: serve the folder with any static server (`python3 -m http.server`, `npx serve`, etc.) and load `Sindri Editor.html`. Opening directly from disk also works in most browsers.

---

## Brand mark

The chosen logo is **Forge** — two concentric hairline hexagons containing a solid amber hex ember. SVGs are in `assets/`. Geometry:

- Outer hex: stroke-width `1.6`, radius `13` (relative to 32×32 viewbox)
- Inner hex: stroke-width `1.2`, radius `7`
- Ember hex (filled): radius `2.8`
- All hexagons are point-up (first vertex at top).

Wordmark: "SINDRI" in **Pixelify Sans** (Google Fonts), weight 700, letter-spacing `0.14em`, uppercase. The "I" between "S" and "NDRI" is rendered in the amber accent color.

---

## Design tokens

### Color palette — dark theme (default)

| Token       | Hex        | Use                                          |
|-------------|------------|----------------------------------------------|
| `paper`     | `#0d1117`  | App background                               |
| `paper-2`   | `#161b22`  | Elevated panels, modal scrim base, chips     |
| `paper-3`   | `#1e2530`  | Hover / selection background                 |
| `ink`       | `#e6e1d4`  | Primary text — warm off-white                |
| `ink-2`     | `#c4beae`  | Secondary text                               |
| `ink-3`     | `#8a8580`  | Tertiary text / muted labels                 |
| `ink-4`     | `#5a554e`  | Very muted text, gutter line numbers         |
| `rule`      | `#1f242c`  | Default hairline rule                        |
| `rule-2`    | `#2a3038`  | Major region rule                            |
| `amber`     | `#f0c050`  | AI accent (one color, used exclusively for AI) |
| `cyan`      | `#6dbcdb`  | Entity / camera accent                       |
| `moss`      | `#9bb070`  | "Good" state (engine ready, accepted diff)   |

**Single accent rule:** amber is reserved for AI-related affordances. Don't use it for non-AI buttons.

### Color palette — light theme (optional, available via Tweaks)

If you want to ship a light variant, see `Sindri Editor v2.html` (preserved in the project but not in this handoff) for the corresponding editorial palette. The dark theme is the default and recommended one — it matches the existing Sindri aesthetic.

### Typography

| Token        | Family                                              | Use                                                   |
|--------------|-----------------------------------------------------|-------------------------------------------------------|
| `f-display`  | `"Pixelify Sans", "VT323", monospace`               | Hero text, modal titles, empty-state messages         |
| `f-sans`     | `"Pixelify Sans", "VT323", monospace`               | All UI text, labels, buttons                          |
| `f-mono`     | `"JetBrains Mono", "VT323", monospace`              | Code editor, numeric values, file paths, model IDs    |

Load via Google Fonts:
```html
<link href="https://fonts.googleapis.com/css2?family=Pixelify+Sans:wght@400;500;600;700&family=VT323&family=JetBrains+Mono:wght@400;500;600&display=swap" rel="stylesheet">
```

**Scale:** 10.5 / 11 / 11.5 / 12 / 12.5 / 13 / 14 / 16 / 18 / 22 / 26 / 36 px. No italic — pixel fonts don't italicize.

**Letter-spacing:** `0.14em` for the SINDRI wordmark and ALL-CAPS section labels. Otherwise default.

### Spacing scale

`4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 28, 32` (px). Standard panel padding is 16–22px.

### Borders & rules

Single-pixel borders only. No box-shadow on chrome. The only shadow allowed is on the Cmd-K modal scrim (a faint 24px blur drop) and the scene composer modal.

### Radii

**Zero.** Every rectangle is hard-cornered. No `border-radius` anywhere. This is intentional — it matches the existing terminal-leaning Sindri aesthetic.

---

## Layout

### Frame

Designed for **1480 × 920** at native scale. The host page applies a uniform transform scale to fit the viewport, but the editor itself is laid out at native pixels. Real implementation should use the natural window size.

### Top-level grid (vertical)

```
┌───────────────────────────────────────────────────────┐
│  Topbar              56px                              │
├───────────────────────────────────────────────────────┤
│                                                        │
│  Body                1fr                              │
│  ┌──────────┬──────────────────────────┬───────────┐  │
│  │ Scene    │  Viewport     1fr        │ Inspector │  │
│  │ tree     │  ────────────────         │ /         │  │
│  │ 260px    │  Script pane  280px      │ Proposals │  │
│  │          │                           │ 360px     │  │
│  └──────────┴──────────────────────────┴───────────┘  │
├───────────────────────────────────────────────────────┤
│  Status bar          28px                              │
└───────────────────────────────────────────────────────┘
```

### Topbar (56px)

Three columns, gridded `380px 1fr 300px`.

- **Left (380px):** Logo lockup (Forge mark + SINDRI wordmark, 22px) · vertical hairline · breadcrumb (`scenes / editor_scene`, mono 12px). Breadcrumb truncates with ellipsis if needed.
- **Center (flex):** Cmd-K input (paper-2 bg, 32px tall, max-width 420px, italic placeholder "Ask Sindri to build something…", `⌘K` keycap on the right) + Run controls (Play primary, Pause, Stop — each 34×28px, hairline borders).
- **Right (300px):** "Compose" link (amber, Pixelify Sans 16px, with sparkle icon) + AI status pill (mono 11.5px, `paper-2` bg, dot + text + `qwen3:14b` model, max-width 240px, truncates).

### Scene tree pane (260px, left)

- **Tabs** (42px tall): Scene / Files / History. Active tab gets a 2px ink underline.
- **Section header**: muted label "editor_scene" + "+" affordance.
- **Entity row**: caret, 7×7px color dot (orange / navy / moss / stone), entity name. Selected row gets `paper-3` background + 2px ink left border.
- **Component child row**: indented 36px, hairline icon, type name, optional mono source path on the right.
- **Ghost entity**: amber color, italic feel via display fallback. Section header "Proposed by AI" above with amber `+N` count.

### Viewport (center top)

- **Tabs** (42px): Scene / Game / Split. Right side shows mono `1280 × 720 · 0.47×`.
- **Stage**: light grid pattern via SVG (two layers: 40px minor at 0.04 ink opacity, 200px major at 0.08).
- **Camera frame**: cyan 1px border, faint cyan fill `rgba(109,188,219,0.06)`, "Main Camera" label top-left in cyan sans.
- **Entity boxes**: 1px solid border in entity color, faint same-color fill (`rgba(...,0.08)`). Selected: 2px ink border. Labels in 11px sans above the box.
- **Ghost entities**: 1.5px dashed amber border, amber fill `rgba(240,192,80,0.10)`.
- **Bottom-right meta**: mono `x 524  y 312` in ink-4.

### Script pane (center bottom, 280px tall)

- **Tabs** (36px): one per open file, each with a small Script icon, filename, optional amber `●` if there's an unaccepted proposal on it. Active tab has paper background.
- **Editor**: JetBrains Mono 12.5px, line-height 1.6. Gutter is 48px wide, right-aligned line numbers in ink-4.
- **Diff lines**: added → moss tint background + green `+` in gutter. Removed → amber tint background + amber `−` in gutter.
- **Inline ghost suggestion** (Cursor-style): amber italic-feel text (Pixelify doesn't have true italic; use color + opacity 0.85), prefixed by `✦` in the gutter. A small hint row below: `Tab accept · Esc dismiss`.

### Inspector pane (360px, right) — entity selected

- **Header** (22px padding): topline `■ <kind>` (color dot + entity kind), entity name in 36px Pixelify Sans, meta row in mono 11px: `<id> · <N> components`.
- **Component blocks**: each in its own section with hairline divider. Header is icon + type name + `···` menu. Body is a two-column grid: 78px label column (ink-3), value column (mono 12px).
- **Add component**: dashed-border row at bottom with `+ Add component`.
- **AI suggestions block**: at bottom. Amber sparkle + "Ask about <Entity>" label, then 2–3 vertical link rows separated by dotted dividers. Each row has a chevron-right at the end.

### Inspector pane — review mode (when AI has proposals)

Replaces the inspector entirely with a Proposals Lane:
- **Header**: amber "✦ AI proposal · awaiting review" label, the quoted prompt in Pixelify Sans 22px, summary in 12px ink-3. Below: Accept all (primary), Reject all, Follow-up. Then a mono count line: `0 accepted · 0 rejected · 3 pending`.
- **Change blocks** (one per proposed change): kind badge (`script` / `component` / `asset`, color-coded), title in 14px ink, meta in mono 11px. Then the diff or key/value details. Per-change action row: `accept` / `reject` / `refine…`.

### Status bar (28px)

`paper-2` background, hairline top border. Mono 11px ink-4. Left to right: `● engine ready` (moss dot) · `● ollama · localhost:11434` (moss) · `<ai-status-text>` · spacer · `4 entities` · `0 errors` · `sindri v0.1.4`.

---

## Scenarios / states

The prototype demonstrates six discrete scenarios, picked via a small "Demo →" switcher in the top-right of the viewport (this picker is for demo only; remove in production):

1. **Rest** — default state, entity selected, inspector shows component fields.
2. **⌘K** — command palette open with sample prompt + context chips.
3. **Compose** — scene composer modal open in input state, awaiting the user's prompt.
4. **Composing** — sub-state of Compose: animated dots + wash over the preview while the model drafts.
5. **Result** — modal closed, ghost entities (Player, Wolf) visible in scene tree under "Proposed by AI" and in viewport as dashed amber boxes.
6. **Review** — proposals lane open with 3 staged changes (script diff, component, asset).
7. **Inline** — same as Review plus the script editor shows the diff inline with banner-less accept/reject lines.

---

## Features (functional spec)

See `Sindri Features Brief.html` for the narrative version with screenshots. Bullet form:

1. **⌘K command palette** — primary input, accepts natural language. Suggestions grouped by `AI · scene`, `AI · selection`, `Editor`. Context chips show what's attached; `@` to add more. Footer keymap: `↑↓ navigate · ↵ apply · ⌘↵ preview diff`.
2. **Scene composer** — full-screen modal. Prompt on the left, live preview SVG on the right showing drafted entities with bounding boxes. Below the preview, a row-by-row entity table (name, components, kind). Compose button triggers the thinking state, then resolves to staged ghosts.
3. **Proposal review lane** — replaces inspector when there are staged changes. Per-change accept/reject/refine. Accept all / Reject all at the top. Follow-up button for multi-turn.
4. **Ghost entities** in scene tree and viewport — visually distinct (amber, dashed), behave like real entities for selection/inspection but don't exist on disk until accepted.
5. **Ghost components** — proposed components attach to existing entities as orange rows in the tree, marked "proposed".
6. **Inline diff** in script editor — added/removed lines color-coded with gutter symbols; user can accept/reject from the inspector or in-line.
7. **Inline ghost suggestion** — Cursor-style ghost text autocomplete; `Tab` accepts.
8. **Activity history** — reversible log of every action with attribution (you / ai). AI rows tagged amber.
9. **Asset generation** — generated assets show in the project as `generated · placeholder`.
10. **Quiet AI status** — single pill in topbar with dot + text. No banner alerts.

### Stretch ideas (not prototyped, documented in the brief)

- Run-loop probes ("why isn't the drone moving?" with live state)
- Multi-turn refine on a single proposal
- Voice-while-playtesting
- Variation generator (forked scenes)
- Explain mode (AI leaves inline comments)

---

## Interactions & behavior

### Cmd-K
- Global keybind. `⌘K` (or `Ctrl+K` on Windows/Linux) opens from anywhere.
- Opens with current scene + selected entity attached as context chips.
- `Esc` closes. Click scrim closes. `↵` submits. `⌘↵` shows the diff before applying.
- Selected suggestion highlighted with `paper-3` background + 2px amber left border.

### Scene composer
- Opens via topbar "Compose" link or via the demo picker.
- Click outside or "Cancel" → closes.
- Inner "Compose" button → enters thinking state for ~1.6s (animated dots on the button, wash over the preview) → closes modal, ghosts appear in editor.
- `Esc` closes from any state.

### Proposal review
- Each change row has hover highlight in the viewport (e.g. a connection arrow drawn from Drone → Beacon for the chase script).
- Per-change accept/reject is toggleable (clicking accepted again un-accepts).
- "Accept all" / "Reject all" applies to all changes at once.

### Script editor
- Tabs are clickable to switch files. Active tab gets full opacity background.
- Click a script entry in the inspector to open it in the script pane.
- Inline ghost: `Tab` accepts, `Esc` dismisses, `⌘↵` accepts + explains.

### Tweaks panel
- An in-design control panel for switching theme (light/dark), logo (anvil/forge/rune), density, accent color. Production should not ship this — it's a prototype convenience. The starter component handles it.

---

## State management

For a real implementation, the editor needs persistent state for:

- `scene`: list of entities, each with `id`, `name`, `kind`, `color`, `components[]`, `pos`
- `selectedId`: currently selected entity id (or null)
- `openIds`: which scene-tree entities are expanded
- `openFile`: currently open script in the script pane
- `sceneTab`: which left-pane tab is active (`scene` / `files` / `history`)
- `viewTab`: which center-top tab is active (`scene` / `game` / `split`)
- `proposalStatuses`: per-change `accept | reject | null`
- `hoverProposal`: which proposal is hovered (for viewport overlay)
- `aiStatus`: `idle | busy | review` + a text string for the topbar pill
- `cmdkOpen`, `composeOpen`, `composing` (transient modal states)
- `activity[]`: history of actions with `{ time, actor, action, kind }`

The prototype uses a `useReducer` in `v3/app.jsx` — see there for the full action set.

---

## Wiring to the model (Ollama)

The prototype shows `qwen3:14b · local` as the active model. Implementation:

- Stream completions from the local Ollama server (default `localhost:11434`).
- The model should produce a structured diff object describing proposed changes (entities to add/remove/modify, scripts to write, components to attach, assets to generate).
- The UI then renders those as proposals — nothing is applied until the user accepts.
- For inline ghost suggestions in the script editor, use a smaller / faster model for low-latency completion.

### Suggested proposal schema

```ts
type Proposal = {
  prompt: string;          // the user's original prompt
  summary: string;         // one-line natural-language summary
  changes: Change[];
};

type Change =
  | { id: string; kind: 'modify-script'; entity: string; file: string; title: string;
      removed: { n: number; t: string }[]; added: { n: number; t: string }[] }
  | { id: string; kind: 'add-component'; entity: string; title: string;
      details: { k: string; v: string }[] }
  | { id: string; kind: 'add-entity'; title: string;
      entity: { name: string; kind: string; components: string[] }; }
  | { id: string; kind: 'remove-entity'; entity: string; title: string }
  | { id: string; kind: 'add-asset'; entity: string; title: string;
      details: { k: string; v: string }[] };
```

This is the shape the prototype uses (`js/data.js · PROPOSED_DIFF`).

---

## Assets

- **Logo SVGs**: `assets/sindri-forge-light.svg`, `assets/sindri-forge-dark.svg`. Built for crisp rendering at any size; 16px favicon all the way up to 240px hero. Use the dark variant on the dark editor background.
- **Icons**: hairline glyph set in `js/icons.jsx` — Transform, Sprite, Script, Camera, Audio, RigidBody, Collider, Light, Tilemap, plus UI icons (Play, Pause, Stop, Sparkle, Chevron, Plus, X, Check, Search, etc.). All built on a 16×16 viewbox with stroke-only paths.
- **Fonts**: Google Fonts — Pixelify Sans, VT323 (fallback), JetBrains Mono. See typography section.

No bitmap art is needed.

---

## Final notes for the developer

- **Don't ship the Tweaks panel.** It's a prototype affordance.
- **Don't ship the demo scenario picker** in the top-right of the viewport.
- **Don't ship the placeholder scene data** (`js/data.js`). It's there to make the prototype render something.
- **Do match the type, color, and spacing exactly.** This is high-fidelity.
- **Do treat amber as AI-only.** Don't use it for non-AI buttons or accents.
- **Do keep all corners hard.** No `border-radius`.
