Forge2D Editor Roadmap (Unity-Like)
===================================

Foundations
-----------
- [x] Serialize/restore entities + core components (Transform/Sprite/Physics/ScriptTag) in scene save/load.
- [x] Add component add/remove in Inspector (Transform/Sprite/Physics/ScriptTag).
- [x] Sync PhysicsBody changes to PhysicsWorld in edit mode (create/remove body/colliders).
- [x] Persist texture paths in Sprite inspector (pick asset, preview).

Viewport + Interaction
----------------------
- [x] Replace canvas renderer with engine/wgpu render pipeline (shared scene camera).
- [x] Selection by click/box select; show selection bounds.
- [x] Gizmo: move/rotate/scale handles w/ snapping (grid + angle).
- [x] Transform gizmo respects parent hierarchy transforms.
- [x] Camera controls: frame selected, reset, configurable grid size.

Scene System
------------
- [x] Scene dirty indicator + save prompt on close/open.
- [x] Multi-scene tabs (open several scenes).
- [x] Prefab-like reuse (save entity tree to asset, instantiate).

Assets + Project
----------------
- [x] Asset import pipeline (textures into project/assets/textures).
- [ ] Thumbnail previews in Project panel.
- [ ] Drag-drop assets into viewport to spawn sprites.

Play Mode
---------
- [x] Play/stop with full scene snapshot (components + physics + texture paths).
- [ ] Game view render (separate camera, resolution controls).
- [ ] Pause/step while in play mode.

Scripting Integration
---------------------
- [ ] Script component editor (script path + params).
- [ ] Hot-reload scripts in editor play mode.
- [ ] Console panel for script logs/errors.
