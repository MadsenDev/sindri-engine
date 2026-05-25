# Sindri Roadmap & Development Philosophy

## Current Status

Sindri has reached a point where it has **enough subsystems to build real games**, but the focus is shifting from "adding features" to "making existing features cohere."

## Development Philosophy

**Coherence over completeness.** The engine should make existing systems work well together rather than adding every possible feature.

**Reference game driven.** New features should be driven by building actual games, not by feature checklists.

## Known Gaps (Not Priorities)

These are **not yet implemented**. They may be added if a reference game forces them:

### Animation System
- ✅ Sprite sheet animation — `AnimatedSprite` component with named clips, frame timing, looping/one-shot, Lua `self:animated_sprite()` facet, full editor support (clip editor, spritesheet slicer)
- Tweening utilities — not built in; use delta time manually
- State-driven animation graphs — not yet
- Timing utilities beyond delta/fixed — not yet

**Status:** Core sprite animation is implemented and editor-ready. State machines and tweening are not built in.

### Advanced UI Framework
- Layout system (anchors, padding, constraints)
- Input routing (click-through, focus management)
- Basic widgets (button, slider, list, etc.)

**Status:** HUD primitives only. For complex UI, use `egui` or build custom.

### Debug Tooling
- In-engine debug drawing (colliders, contacts, AABB, nav grid)
- Gizmos and entity picking (select entity under mouse)
- Live console/log overlay
- Entity tree inspector
- Performance HUD toggle

**Status:** Not implemented. Use `println!`, external profilers, or build custom tools.

### Asset Pipeline
- Texture atlasing
- Asset compression
- Hot reload (shaders/textures/scenes)
- Asset packing for distribution

**Status:** Runtime loading only. Use external tools for preprocessing.

### Export/Packaging
- Platform-specific packaging
- Asset bundling
- Distribution tools

**Status:** Use `cargo` and platform-specific tools.

## Potential Next Steps (If Reference Game Demands)

These would be added **only if** building a reference game forces them:

### 1. Animation System
**If needed for:** Platformer with character animation, cutscenes, UI transitions

**Minimal viable version:**
- Sprite sheet animation (frame indices, timing)
- Simple tweening (lerp, ease functions)
- Animation state machine (idle, walk, jump, etc.)

### 2. Debug Tools
**If needed for:** Complex games with many entities, physics debugging, pathfinding visualization

**Minimal viable version:**
- Debug draw API (lines, shapes, text)
- Entity selection (mouse picking)
- Simple inspector (show entity components)

### 3. ECS Ergonomics
**If needed for:** Games with many entity types, complex systems, performance issues

**Options:**
- Improve query ergonomics (multi-component queries)
- Add system scheduling
- Or recommend integrating `hecs`/`bevy_ecs` for advanced needs

### 4. Scene Editor Workflow
**If needed for:** Level design, rapid iteration, non-programmer content creation

**Minimal viable version:**
- JSON scene editor (external tool or simple in-engine)
- Hot reload scenes
- Visual entity placement

## Reference Game Approach

The recommended development cycle:

1. **Pick a reference game type:**
   - Platformer (movement feel, slopes, coyote time, animation, camera)
   - Top-down tile game (pathfinding, interactions, UI, persistence)
   - Action game (combat, effects, state machines)
   - Puzzle game (grid logic, UI, state management)

2. **Build it using existing systems:**
   - Use Sindri's physics, rendering, pathfinding, etc.
   - Implement missing pieces (animation, UI) as needed
   - Keep it simple—don't add features "just in case"

3. **Identify pain points:**
   - What feels awkward?
   - What requires too much boilerplate?
   - What breaks the flow?

4. **Add only what's forced:**
   - If animation is painful, add minimal animation system
   - If debugging is painful, add minimal debug tools
   - If UI is painful, add minimal UI primitives

5. **Iterate:**
   - Build the same game twice
   - Second time should be faster
   - If not, the engine isn't winning

## Success Metrics

Sindri is "winning" if:

- ✅ You can build a complete game slice (not just a demo)
- ✅ Building the same game twice is faster the second time
- ✅ The engine doesn't get in your way
- ✅ Systems work well together (no awkward integration)
- ✅ Code feels clean, not like workarounds

## Anti-Goals

Things Sindri explicitly **won't** become:

- ❌ A "do everything" engine (like Unity/Godot)
- ❌ A framework that requires learning a custom language/scripting system
- ❌ A tool that requires complex build pipelines or external tools
- ❌ A system that hides complexity behind "magic" (you should understand what's happening)

## Current Focus

**Right now:** The engine has enough features. The focus is on:

1. **Documentation** - Make it clear what Sindri does and doesn't do
2. **Examples** - Build complete game slices, not just demos
3. **Coherence** - Make existing systems work better together
4. **Reference game** - Pick one and use it to drive decisions

**Not focusing on:** Adding new major subsystems unless a reference game forces them.

