Forge2D Engine Improvements TODO
===============================

Core lifecycle
--------------
- [x] Add `Game::fixed_update(&mut self, ctx)` and run fixed-step loop inside `Engine::run`.
- [x] Clamp large `delta_time` to avoid spiral-of-death after hitches/debug pauses.
- [x] Move `EngineContext::begin_frame()` call to right before update (clear per-frame input at a clear boundary).
- [x] Document or enforce the update/draw contract (single "update then render" or explicitly split).

Window and renderer ownership
-----------------------------
- [x] Remove leaked `Window` (`Box::leak`) by owning the window inside `EngineContext`.
- [x] Refactor renderer to avoid long-lived `&Window` borrow (init-time borrow only).
- [x] Ensure resize and surface handling still work with new ownership.

Ergonomics and helpers
----------------------
- [x] Add `delta_seconds()` and `fixed_delta_seconds()` helpers.
- [x] Add `EngineContext::draw(|gfx| ...)` or similar command-style helpers to hide borrow choreography.
- [x] Expand convenience asset helpers (continue `load_*` pattern).

Input
-----
- [x] Clarify input frame boundary in docs and comments; ensure per-frame flags are reset in one place.

Rendering
---------
- [x] Consider auto-rasterizing text glyphs in `draw_text()` or provide a higher-level text helper.
- [x] Provide a canonical render path example (fixed update + interpolation + input usage).

State machine
-------------
- [x] Make transition timing explicit (begin-of-frame vs end-of-frame) and align docs with behavior.

Audio
-----
- [x] Add cached/preloaded `SoundHandle` to avoid reopening files for repeated SFX.

Assets
------
- [x] Consider typed asset IDs or virtual asset root for simpler paths and future hot-reload.

Examples/docs
-------------
- [x] Add a "golden" example using fixed-update + interpolation (`fixed_update_alpha()`).
- [x] Document draw side-effects as unsupported if update/draw remain split.

Scripting
---------
- [x] Create per-instance Lua environments and cache callbacks (avoid globals pollution).
- [x] Bind `params` into per-instance environments instead of globals.
- [x] Route physics events to the correct script instance callbacks.
- [x] Clean up Lua registry values on destroy/reload to avoid leaks.
