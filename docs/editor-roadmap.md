# Forge2D Editor Functional Roadmap

## Goal

This document captures what it will take to turn the current Forge2D editor from a functional scene manipulation tool into a fully functional day-to-day editor for building games with the engine.

Assumption: "fully functional editor" means a credible authoring tool for scenes, assets, prefabs, and play/test iteration, not just a UI shell that can inspect and mutate a runtime `World`.

## Current State

The editor already has solid foundations:

- Tauri backend command surface for scene, entity, component, prefab, project, and play controls
- React-based docking UI with hierarchy, inspector, viewport, project, play, and console panels
- Scene save/load
- Prefab save/instantiate
- Metadata-driven inspector editing
- Viewport rendering and simple transform gizmos
- Basic play/pause/stop flow

Relevant code:

- [editor/src/main.rs](../editor/src/main.rs)
- [editor/src/app/App.tsx](../editor/src/app/App.tsx)
- [editor/src/components/Viewport.tsx](../editor/src/components/Viewport.tsx)
- [editor/src/components/Inspector.tsx](../editor/src/components/Inspector.tsx)

The main gap is not "missing everything." The main gap is that the editor is still closer to a scene inspector/manipulator than a full production authoring environment.

## Workstreams

## 1. Editor Document Model

The current backend holds a single live world. The UI exposes scene tabs, but those tabs are not true independent documents. Switching tabs loads another scene into the same backend state or creates a new empty one.

This is the first architectural decision to make.

Options:

- Simplest: explicitly make the editor single-document and remove the illusion of true multi-tab editing
- Medium: keep per-tab scene state and only materialize the active scene into the runtime world
- Strongest: introduce an editor document layer separate from the runtime `World`

Why this matters:

- dirty-state handling is currently global, not per document
- tab switching is not a real multi-document workflow
- undo/redo and selection state are tied to the active world rather than a durable document model

Recommendation:

- If speed matters most, make the editor honestly single-scene first
- If multi-tab editing is a product requirement, build a document layer before adding more editor features

## 2. Authoring UX

The hierarchy and viewport are usable, but still lightweight.

Current limitations:

- entities are identified by ID rather than meaningful names
- hierarchy lacks filtering, search, icons, visibility toggles, lock state, and richer multi-select workflows
- selection bounds in the viewport are inferred from transform scale rather than true sprite/collider bounds
- gizmos are basic and effectively single-entity oriented
- there is no local/world transform mode, pivot mode, or richer snapping controls
- viewport only renders sprites — the engine also has animated sprites, particle emitters, lights (`DirectionalLight`, `PointLight`), and a full HUD system (`HudLayer`, `HudPanel`, `HudText`, `HudSprite`) that are invisible in the editor

To feel fully functional, the editor needs:

- entity naming or editor-only labels/metadata
- stronger hierarchy interaction and filtering
- better viewport hit-testing
- real transform tools with more robust snapping and selection behavior
- visibility/focus/isolate workflows
- expanded viewport rendering to cover animated sprites, particles, lights, and HUD elements

Recommended scope:

- add entity names first
- improve hierarchy usability second
- then strengthen gizmo and selection accuracy
- extend viewport rendering incrementally as component types are properly exposed

## 3. Asset Pipeline

The current asset workflow is narrow:

- project file tree
- texture import via file copy
- drag/drop textures and prefabs into the scene

That is enough for demos, but not enough for a serious editor workflow.

Missing capabilities:

- rename/move/delete from inside the editor
- reference-safe asset moves
- asset metadata/indexing
- previews beyond textures
- script asset creation and validation
- support for more asset classes such as audio, fonts, and script templates
- tilemap editor (the engine supports `TilemapComponent` and tile rendering, but tiles cannot be painted or edited in the editor)
- animation editor (the engine has a full `Animation`/`AnimatedSprite` system, but no clip authoring or timeline exists)
- particle emitter editor (the engine has `ParticleSystem`/`ParticleEmitter`, but no property editing in the editor)
- light placement (the engine has `DirectionalLight` and `PointLight` but they cannot be placed or configured from the editor)
- input map editing (the engine has `InputMap`/`ActionId`/`AxisBinding`, but there is no editor UI to define action bindings)

Recommendation:

- start by adding internal asset operations and path/reference safety
- then add broader asset type support and previews
- treat tilemap, animation, particle, and light editing as distinct workstreams once the core asset pipeline is stable

## 4. Prefab System

Current prefabs are serialized snapshots that can be saved and instantiated. That is useful, but it is not a full prefab system.

Missing prefab features:

- persistent prefab linkage on instances
- instance override tracking
- apply/revert workflow
- nested prefab rules
- prefab instance indicators in hierarchy and inspector

This is a moderate systems project rather than a simple UI task.

Recommendation:

- keep current snapshot prefabs as MVP
- add prefab linkage and overrides once core document/play-mode workflows are stable

## 5. Play Mode and Runtime Integration

This is the largest product gap.

Current play mode:

- snapshots the scene
- steps physics
- copies body transforms back into entity transforms
- restores the snapshot on stop

That is closer to a physics preview than true game execution.

The editor should eventually run actual game/runtime behavior, including:

- runtime bootstrap from the active scene
- script lifecycle and update/fixed-update integration
- engine-level camera resolution
- runtime logging and errors forwarded into the editor console
- deterministic start/stop/reset behavior
- optional hot-reload for scripts where feasible
- physics debug visualization (collision wireframes, contact points, gravity vectors) during play mode
- input forwarding so keyboard/mouse events reach running game code during play

Without this, the editor cannot serve as a reliable play/test loop.

Recommendation:

- prioritize real runtime integration before investing heavily in secondary UI polish

## 6. Reliability and Workflow Polish

This determines whether the editor is trusted.

Important missing areas:

- autosave
- crash recovery
- per-document dirty tracking
- better surfaced validation and errors
- clipboard copy/paste
- subtree duplication
- scene/prefab reference integrity checks (broken texture/script path detection)
- stronger diagnostics in the console
- broader automated test coverage around serialization and editor commands
- consider migrating undo/redo from snapshot-based to command-based (the engine already defines a `CommandHistory` trait with typed commands like `CreateEntity`, `DeleteEntity`, `SetTransform`, `AddComponent`, etc.; this would reduce memory overhead and enable more granular history)

Recommendation:

- treat this as a continuous workstream, not a final cleanup phase

## Suggested Priority Order

If the goal is a usable internal editor quickly, work in this order:

1. Decide single-document vs real multi-document editing
2. Upgrade play mode to run actual runtime/script behavior
3. Add entity names and strengthen hierarchy/selection UX
4. Improve viewport hit-testing and transform tooling
5. Expand the asset pipeline
6. Add prefab linkage and overrides
7. Add autosave, recovery, validation, and better diagnostics

## MVP vs Post-MVP

### MVP

An editor that is meaningfully usable every day for internal scene authoring should include:

- stable single-document or true document-backed scene editing
- reliable scene save/load
- robust undo/redo
- entity naming
- hierarchy search/filter
- trustworthy viewport selection and transform tools
- texture/script/basic asset workflows
- real play mode with script/runtime integration
- clear runtime and editor errors

### Post-MVP

These features make the editor substantially stronger, but should come after the core loop is reliable:

- prefab linkage and overrides
- nested prefabs
- broader asset import pipeline
- advanced scene tabs/document workflows
- autosave and crash recovery
- richer diagnostics and debugging tools
- editor settings and personalization

## Rough Effort

For one strong engineer focused on the editor:

- usable daily for internal scene authoring: about 4 to 8 weeks
- credible public-facing editor with runtime/prefab/asset workflows: about 3 to 5 months
- polished Unity-lite experience: materially longer than that

These estimates assume the existing editor foundation remains intact and that the scope stays focused.

## Practical Recommendation

If the near-term goal is momentum, do not try to solve everything at once.

Take one of these paths:

- Product-honest path: make it a strong single-scene editor first, then add deeper systems
- Platform path: build the document/runtime split first, then layer features on top

If forced to choose only one high-leverage investment next, choose play mode/runtime integration. Without that, the editor remains useful for layout, but not for real game iteration.
