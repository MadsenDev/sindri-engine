// Sindri scene + AI flow data. Globals.

const INITIAL_SCENE = {
  name: "editor_scene",
  path: "scenes/editor_scene.sndr",
  entities: [
    {
      id: "beacon",
      name: "Beacon",
      kind: "actor",
      color: "orange",
      components: [
        { type: "Transform", pos: [-32, 0], rot: 12, scale: 1 },
        { type: "Sprite", source: "sprites/beacon.png", tint: "#d4541e" },
        { type: "Script", source: "beacon.lua", lines: 28 },
      ],
      pos: { x: 540, y: 260, w: 72, h: 72, rot: 15 },
    },
    {
      id: "drone",
      name: "Drone",
      kind: "actor",
      color: "navy",
      components: [
        { type: "Transform", pos: [40, 4], rot: -8, scale: 1.1 },
        { type: "Sprite", source: "sprites/drone.png", tint: "#2b4a6f" },
        { type: "Script", source: "drone.lua", lines: 42 },
      ],
      pos: { x: 660, y: 235, w: 92, h: 92, rot: -8 },
    },
    {
      id: "ground",
      name: "Ground",
      kind: "static",
      color: "stone",
      components: [
        { type: "Transform", pos: [0, -90], rot: 0, scale: 1 },
        { type: "Sprite", source: "sprites/ground.png", tint: "#a8a298" },
      ],
      pos: { x: 380, y: 370, w: 520, h: 18, rot: 0 },
    },
    {
      id: "camera",
      name: "Main Camera",
      kind: "camera",
      color: "navy",
      components: [
        { type: "Transform", pos: [0, 0], rot: 0, scale: 1 },
        { type: "Camera", size: [1280, 720], active: true },
      ],
      pos: null,
    },
  ],
};

// Drone.lua content used for the script pane
const DRONE_LUA = [
  '-- drone.lua',
  '-- A simple patrol drone.',
  '',
  'local Drone = {}',
  '',
  'function Drone:init()',
  '  self.speed = 60',
  '  self.dir = 1',
  '  self.range = 120',
  '  self.origin = self.transform.x',
  'end',
  '',
  'function Drone:update(dt)',
  '  local t = self.transform',
  '  t.x = t.x + self.speed * self.dir * dt',
  '',
  '  if math.abs(t.x - self.origin) > self.range then',
  '    self.dir = -self.dir',
  '  end',
  'end',
  '',
  'return Drone',
];

const BEACON_LUA = [
  '-- beacon.lua',
  '-- Pulses light, attracts drones.',
  '',
  'local Beacon = {}',
  '',
  'function Beacon:init()',
  '  self.pulse = 0',
  '  self.radius = 80',
  'end',
  '',
  'function Beacon:update(dt)',
  '  self.pulse = (self.pulse + dt) % 1.0',
  '  self.sprite.alpha = 0.6 + 0.4 * math.sin(self.pulse * 6.28)',
  'end',
  '',
  'return Beacon',
];

// AI proposed diff for "make drone follow beacon"
const PROPOSED_DIFF = {
  prompt: "Make the drone follow the beacon and play a chime when it gets close.",
  summary: "3 changes across 2 entities — adds a new component, modifies a script, creates an audio asset.",
  changes: [
    {
      id: "ch1",
      kind: "modify-script",
      entity: "Drone",
      file: "drone.lua",
      title: "Rewrite update() to chase target",
      removed: [
        { n: 14, t: '  t.x = t.x + self.speed * self.dir * dt' },
        { n: 15, t: '' },
        { n: 16, t: '  if math.abs(t.x - self.origin) > self.range then' },
        { n: 17, t: '    self.dir = -self.dir' },
        { n: 18, t: '  end' },
      ],
      added: [
        { n: 14, t: '  local target = scene.find("@Beacon").transform' },
        { n: 15, t: '  local dx, dy = target.x - t.x, target.y - t.y' },
        { n: 16, t: '  local d = math.sqrt(dx*dx + dy*dy)' },
        { n: 17, t: '  if d > 0 then' },
        { n: 18, t: '    t.x = t.x + (dx/d) * self.speed * dt' },
        { n: 19, t: '    t.y = t.y + (dy/d) * self.speed * dt' },
        { n: 20, t: '  end' },
        { n: 21, t: '  if d < 32 and not self.chimed then' },
        { n: 22, t: '    audio.play("chime.ogg")' },
        { n: 23, t: '    self.chimed = true' },
        { n: 24, t: '  end' },
      ],
    },
    {
      id: "ch2",
      kind: "add-component",
      entity: "Drone",
      title: "Attach Audio component",
      details: [
        { k: "type", v: "Audio" },
        { k: "clip", v: "chime.ogg" },
        { k: "volume", v: "0.7" },
        { k: "spatial", v: "true" },
      ],
    },
    {
      id: "ch3",
      kind: "add-asset",
      entity: "assets/audio",
      title: "Generate audio asset · chime.ogg",
      details: [
        { k: "kind", v: "synth, bell" },
        { k: "length", v: "0.8s" },
        { k: "key", v: "C5" },
        { k: "source", v: "generated · placeholder" },
      ],
    },
  ],
};

// "Compose a scene" sample output
const COMPOSED_SCENE = {
  prompt: "A small forest clearing at dusk. The player can collect three glowing mushrooms. A wolf patrols the tree line.",
  entities: [
    { name: "Player",        kind: "actor",  components: ["Transform", "Sprite", "Script · player.lua", "RigidBody"] },
    { name: "Wolf",          kind: "actor",  components: ["Transform", "Sprite", "Script · wolf_ai.lua", "Collider"] },
    { name: "Mushroom",      kind: "actor",  components: ["Transform", "Sprite", "Script · pickup.lua"], note: "× 3" },
    { name: "Trees",         kind: "static", components: ["Transform", "Tilemap"], note: "12 tiles" },
    { name: "AmbientLight",  kind: "light",  components: ["Transform", "Light"], note: "dusk warm" },
    { name: "Main Camera",   kind: "camera", components: ["Transform", "Camera"] },
  ],
};

// Activity log
const ACTIVITY = [
  { t: "14:02", actor: "you",   action: "moved Drone to (660, 295)", kind: "edit" },
  { t: "14:03", actor: "you",   action: "edited drone.lua · added range field", kind: "edit" },
  { t: "13:58", actor: "ai",    action: "rewrote update() in drone.lua to chase target", kind: "ai" },
  { t: "13:58", actor: "ai",    action: "attached Audio component to Drone", kind: "ai" },
  { t: "13:58", actor: "ai",    action: "generated chime.ogg (0.8s, bell, C5)", kind: "ai" },
  { t: "13:42", actor: "you",   action: "created Beacon", kind: "edit" },
  { t: "13:41", actor: "ai",    action: "scaffolded scene from prompt · 'patrol drone demo'", kind: "ai" },
  { t: "13:40", actor: "you",   action: "new scene · editor_scene", kind: "edit" },
];

// Command palette suggestions
const COMMAND_SUGGESTIONS = [
  { group: "AI · scene",
    items: [
      { glyph: "✦", label: "Compose a scene from a description", hint: "⌘ ⏎" },
      { glyph: "✦", label: "Add a player that can move with arrow keys" },
      { glyph: "✦", label: "Make the Drone follow the Beacon and chime nearby" },
      { glyph: "✦", label: "Generate 3 background variants of this scene" },
    ],
  },
  { group: "AI · selection",
    items: [
      { glyph: "✦", label: "Explain what drone.lua does" },
      { glyph: "✦", label: "Refactor drone.lua to use a state machine" },
      { glyph: "✦", label: "Write a unit test for Drone:update" },
    ],
  },
  { group: "Editor",
    items: [
      { glyph: "→", label: "New entity", hint: "⌘ N" },
      { glyph: "→", label: "New script", hint: "⌘ ⇧ N" },
      { glyph: "→", label: "Open file…", hint: "⌘ P" },
      { glyph: "→", label: "Run · Play", hint: "⌘ R" },
    ],
  },
];

window.SINDRI_DATA = {
  INITIAL_SCENE, DRONE_LUA, BEACON_LUA, PROPOSED_DIFF,
  COMPOSED_SCENE, ACTIVITY, COMMAND_SUGGESTIONS,
};
