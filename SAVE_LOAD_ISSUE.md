# Save/Load Physics Issue Summary

## Problem Description

When saving a physics scene (S key) and loading it (L key), restored objects fall through the floor. However:
- **New objects spawned with mouse click work perfectly** (they collide with ground)
- **Objects collide with each other** (so colliders ARE working)
- **Objects do NOT collide with the ground** (ground collision fails for restored objects only)

This suggests:
- Ground collider is fine (new objects work)
- Restored object colliders are fine (they collide with each other)
- Something about the interaction between restored objects and the preserved ground is broken

## What We've Tried

1. **Entity ID remapping**: Created new World entities and remapped all EntityIds from saved scene
2. **Preserving ground entity**: Tried preserving ground during restore (didn't work)
3. **Recreating ground**: Delete and recreate ground before restore (didn't work)
4. **Order of operations**: Changed order to create bodies → add colliders → set velocities → wake up
5. **Upward offset**: Added 2.0 unit upward offset to prevent embedding
6. **Wake up bodies**: Explicitly wake up dynamic bodies after restoration
7. **Query pipeline update**: Update query pipeline after restore
8. **Physics step after load**: Run a small physics step (0.001s) after loading
9. **Verification**: Added checks to ensure colliders are attached (they are)

## Current Implementation

### Save Process:
1. Extract physics state from `PhysicsWorld`
2. Filter out ground and sensor entities
3. Save to JSON file

### Load Process:
1. Load scene from JSON
2. Delete all existing entities (including ground/sensor)
3. Recreate ground and sensor fresh
4. Remap all EntityIds (create new World entities)
5. Restore physics using `restore_scene_physics_preserve` with ground/sensor in preserve list
6. Run small physics step (0.001s)
7. Rebuild entity tracking list

## Key Observations

- Objects saved while falling still have velocities (though we reset to zero on load)
- Some objects "do circles" before landing mid-air (suggests physics is working but ground collision isn't)
- Two objects sometimes "seem to be on the same line mid-air" (they're colliding with each other)
- Debug output shows ground has 1 collider after restore
- Debug output shows restored entities have colliders

## Files to Include

1. `sindri/src/scene.rs` - Scene serialization/deserialization logic
2. `sindri/src/physics.rs` - Physics world implementation
3. `examples/physics_demo/src/main.rs` - Demo with save/load implementation
4. `sindri/src/world.rs` - World/EntityId system (for context)

## Questions to Investigate

1. Does Rapier's broad phase need to be explicitly rebuilt when bodies are added?
2. Is there a timing issue - do we need to run multiple physics steps?
3. Are preserved entities properly registered in the broad phase?
4. Is there a collision group/filter issue preventing restored objects from colliding with preserved ground?
5. Should we NOT preserve entities and instead recreate everything fresh?

## Rapier Version

Using `rapier2d = "0.14"`

