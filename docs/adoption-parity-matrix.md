# Sindri Adoption / Parity Matrix

This matrix tracks whether Sindri features are usable across the full engine,
runtime, editor, serialization, and local AI tooling stack.

The goal is to prevent ecosystem drift: a feature should not look production-ready
just because the engine can execute it if the editor cannot inspect it, the scene
format cannot preserve it, or the AI can generate invalid partial state.

## Legend

| Mark | Meaning |
| ---- | ------- |
| Yes | Supported enough to use intentionally |
| Partial | Exists, but has known gaps or narrow support |
| No | Not supported yet |
| N/A | Not applicable |

## Capability Matrix

| Feature | Engine | Runtime | Editor | Scene Serialization | AI Read | AI Create | AI Modify | AI Debug | AI Explain | AI Scene-Aware | Status |
| ------- | ------ | ------- | ------ | ------------------- | ------- | --------- | --------- | -------- | ---------- | -------------- | ------ |
| Transform | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Stable |
| Sprite | Yes | Yes | Partial | Yes | Yes | Yes | Yes | Partial | Yes | Partial | Active |
| Collider | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Partial | Yes | Yes | Active |
| PhysicsBody | Yes | Yes | Yes | Partial | Yes | Yes | Yes | Partial | Yes | Partial | Active |
| Lua ScriptComponent | Yes | Yes | Partial | Partial | Yes | Yes | Yes | Partial | Yes | Partial | Active |
| Camera | Yes | Yes | Partial | Yes | Yes | Yes | Yes | Partial | Yes | Partial | Active |
| AudioSource | Yes | Yes | Partial | Yes | Yes | Yes | Yes | No | Yes | Partial | Active |
| Tilemap | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Partial | Yes | Yes | Active |
| Animation (AnimatedSprite) | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Partial | Yes | Yes | Active |
| Particles | Yes | Yes | No | No | Partial | No | No | No | Yes | No | Experimental |
| Lighting | Yes | Yes | No | No | Partial | No | No | No | Yes | No | Experimental |
| Pathfinding/Grid | Yes | Yes | No | No | Partial | No | No | No | Yes | No | Active |
| HUD/UI Primitives | Yes | Yes | No | No | Partial | No | No | No | Yes | No | Active |
| InputMap | Yes | Yes | No | No | Partial | No | No | Partial | Yes | No | Active |
| Built-in gameplay markers | Yes | Yes | No | No | Partial | No | No | No | Yes | No | Active |
| Component metadata registry | Partial | Partial | No | N/A | Partial | No | No | No | Yes | Partial | Prototype |
| Scene Graph / Hierarchy | Partial | Partial | Yes | Yes | Yes | Partial | Partial | Partial | Yes | Yes | Active |
| Undo/Redo Commands | Yes | Partial | Partial | N/A | Partial | No | No | No | Yes | Partial | Active |
| Runtime errors | Partial | Partial | Partial | N/A | Yes | No | No | Partial | Yes | Partial | Active |
| AI Commands | Partial | Partial | Partial | N/A | Yes | Partial | Partial | Partial | Partial | Partial | Prototype |
| Playback controls | Partial | Partial | Partial | N/A | Yes | No | Partial | Partial | Yes | Partial | Active |
| Native play preview | Yes | Partial | Partial | N/A | Partial | No | No | Partial | Yes | Partial | Active |
| Screenshot capture | Yes | Yes | Partial | N/A | Yes | No | No | Partial | Yes | Partial | Active |

## Evidence Notes

This matrix is grounded in the current repo state:

- Engine coverage comes from crate exports and implementations in `crates/sindri/src`, including built-in components, renderer systems, physics, scripting, scene APIs, command history, grids/pathfinding, HUD, audio, and camera systems.
- Editor/server scene JSON currently supports `Transform`, `Sprite`, `AnimatedSprite`, `Tilemap`, `PhysicsBody`, `Collider`, `Script`, `Camera`, and `AudioSource` through `sindri::component::Component`.
- Editor hierarchy can add/remove all nine scene JSON components, and the server patch route can update all of them. The inspector exposes editable fields for each of them, including a tile painter modal for Tilemap.
- The viewport understands `Transform`, `Sprite`, and `Collider` for preview/selection. It does not visualize engine tilemaps, particles, lights, animation, audio, pathfinding, HUD, or gameplay marker components.
- The Scene view now supports basic direct manipulation for `Transform` through Move, Scale, and Rotate tools, including simple gizmo hints, Shift snapping, and transform undo/redo. It does not yet have full axis-handle hit testing, multi-select, or prefab-style editing.
- Scene editing has an explicit save command and dirty indicator in the editor shell, alongside server autosave while playback is stopped.
- Play mode currently uses a separate native `wgpu` preview window owned by `sindri-server`. The editor Game tab is a status surface, not an embedded render surface.
- Playback has explicit play/pause/stop control. Stop restores the edit-scene snapshot captured when playback starts, and autosave is suppressed while playback is not stopped.
- Screenshot capture remains supported for AI/diagnostics, but it is now on-demand rather than a continuously polled editor viewport transport.
- The server Lua runtime exposes the editor-scene subset used by live preview scripts, including direct `self.x`/`self.y` fields plus `vec2`, `self:transform()`, `self:sprite()`, and `self:camera()` helpers. This is still narrower than the core engine `ScriptRuntime`.
- Runtime and Lua errors are now buffered server-side and exposed to the editor and AI, but they are still plain text logs rather than structured diagnostics with entity/script/source locations.
- `register_builtin_metadata()` currently registers only `Transform`, so runtime reflection is not yet the source of truth for the editor's full component set.
- Scene serialization is split: `crates/sindri/src/scene.rs` serializes the editor-facing JSON component enum, while `scene_physics.rs` captures physics snapshots and generic component payload helpers. Engine-only typed components such as `TilemapComponent`, `AnimatedSprite`, particle systems, lights, and gameplay markers are not represented in the editor scene enum today.
- AI actions in the editor/Tauri layer can create/delete/rename entities, edit transforms, write/attach scripts, add/remove scene JSON components, and patch all seven scene JSON component variants. The standalone `sindri-ai` crate has a narrower action enum than the editor prompt/action executor.
- AI script-edit requests now receive the current open Lua tab when script context is enabled, and `#scripts/foo.lua` mentions auto-load that referenced script into the request context for the current prompt.

## AI Capability Definitions

| Capability | Meaning |
| ---------- | ------- |
| AI Read | The AI can inspect the feature from scene/script/error/viewport context. |
| AI Create | The AI can create valid feature state through supported actions. |
| AI Modify | The AI can safely edit existing feature state without corrupting unrelated data. |
| AI Debug | The AI can reason about common failures using available context and propose grounded fixes. |
| AI Explain | The AI has enough prompt/docs context to teach or document the feature accurately. |
| AI Scene-Aware | The AI understands how the feature relates to entities, scripts, scene files, and editor state. |

## Current Ownership

| Area | Primary Owner | Notes |
| ---- | ------------- | ----- |
| Core components | Engine | Defines runtime behavior and component semantics. |
| Scene/component JSON | Runtime + Server | Defines what the editor and AI can exchange safely. |
| Inspector and hierarchy | Editor | Determines whether users can see and edit feature state. |
| Script authoring | Editor + AI | Monaco/editor UX plus AI script generation and attachment. |
| Script execution and hot reload | Runtime | `ScriptRuntime` owns lifecycle, facets, and reload behavior. |
| AI action schema | AI + Editor + Server | Must stay constrained to actions the editor/server can actually apply. |
| Debug/explain flows | AI + Editor | Requires structured errors, scene context, and feature-aware prompts. |

## Immediate Adoption Rules

- New engine features should add a row here before being treated as public/editor-ready.
- AI prompts should not claim write support for a feature unless `AI Create` or `AI Modify` is at least `Partial`.
- Editor affordances should not imply runtime support unless the server can serialize/apply the same data.
- Scene serialization gaps must be visible here before examples or AI flows depend on them.
- A feature is not `Stable` unless engine, runtime, editor, serialization, and AI read/write behavior are coherent.

## Machine-Readable Direction

The Markdown table is the human-maintained source of truth for now. The intended
next step is a checked-in machine-readable mirror that AI code can query before
generating actions:

```json
{
  "feature": "ParticleEmitter",
  "engine": true,
  "runtime": true,
  "editor": false,
  "scene_serialization": false,
  "ai": {
    "read": "partial",
    "create": false,
    "modify": false,
    "debug": false,
    "explain": true,
    "scene_aware": false
  },
  "owner": "Engine",
  "status": "experimental"
}
```

Once that exists, AI tooling should check the matrix before creating or modifying
features. If a capability is not supported, the AI should explain the gap and
offer a supported alternative instead of generating invalid partial state.
