# State Management

Forge2D provides a state management system for organizing your game into distinct states (menu, gameplay, pause, etc.).

## Overview

The state system uses a **stack-based** approach:
- States are pushed onto a stack
- Only the **top state** receives `update()` calls
- All states in the stack receive `draw()` calls (from bottom to top)
- This allows pause overlays, menus over gameplay, etc.

## The State Trait

Implement the `State` trait for each game state:

```rust
use forge2d::{State, EngineContext, StateMachineLike};
use anyhow::Result;

struct MenuState;

impl State for MenuState {
    fn on_enter(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Called when this state is pushed onto the stack
        println!("Entered menu");
        Ok(())
    }

    fn on_exit(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Called when this state is popped from the stack
        println!("Exited menu");
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext, sm: &mut dyn StateMachineLike) -> Result<()> {
        // Called every frame (only for the top state)
        if ctx.input().is_key_pressed(KeyCode::Enter) {
            sm.push(Box::new(GameplayState::new()));
        }
        Ok(())
    }

    fn draw(&mut self, renderer: &mut Renderer, frame: &mut Frame) -> Result<()> {
        // Called every frame (for all states in the stack)
        // The frame is already begun by StateMachine, so just draw to it
        renderer.clear(frame, [0.1, 0.1, 0.2, 1.0])?;
        Ok(())
    }
}
```

## StateMachine

The `StateMachine` manages the state stack:

```rust
use forge2d::{StateMachine, State};

// Create with an initial state
let mut state_machine = StateMachine::with_initial_state(Box::new(MenuState::new()));

// Or create empty and push later
let mut state_machine = StateMachine::new();
state_machine.push(Box::new(MenuState::new()));
```

### State Transitions

States can transition during `update()`:

```rust
impl State for MenuState {
    fn update(&mut self, ctx: &mut EngineContext, sm: &mut dyn StateMachineLike) -> Result<()> {
        if ctx.input().is_key_pressed(KeyCode::Enter) {
            // Push a new state (menu stays in stack, but paused)
            sm.push(Box::new(GameplayState::new()));
        }
        Ok(())
    }
}

impl State for GameplayState {
    fn update(&mut self, ctx: &mut EngineContext, sm: &mut dyn StateMachineLike) -> Result<()> {
        if ctx.input().is_key_pressed(KeyCode::KeyP) {
            // Push pause overlay
            sm.push(Box::new(PauseState));
        }
        
        if ctx.input().is_key_pressed(KeyCode::Escape) {
            // Pop this state (return to previous state)
            sm.pop();
        }
        Ok(())
    }
}

impl State for PauseState {
    fn update(&mut self, ctx: &mut EngineContext, sm: &mut dyn StateMachineLike) -> Result<()> {
        if ctx.input().is_key_pressed(KeyCode::KeyP) {
            // Pop pause (return to gameplay)
            sm.pop();
        }
        
        if ctx.input().is_key_pressed(KeyCode::Escape) {
            // Pop pause and gameplay (return to menu)
            sm.pop(); // Pop pause
            sm.pop(); // Pop gameplay
        }
        Ok(())
    }
}
```

### Transition Methods

- **`push(state)`** - Push a new state onto the stack. Current top state is paused but still drawn.
- **`pop()`** - Pop the current top state. Previous state resumes updates.
- **`replace(state)`** - Replace the current top state (equivalent to `pop()` + `push()`).

**Important:** Transitions are **deferred** until after the current update/draw cycle. This prevents issues with borrowing and ensures clean state transitions.

## Using StateMachine with Engine

`StateMachine` implements `Game`, so you can use it directly:

```rust
use forge2d::{Engine, StateMachine, State};

fn main() -> Result<()> {
    let state_machine = StateMachine::with_initial_state(Box::new(MenuState::new()));
    
    Engine::new()
        .with_title("My Game")
        .with_size(1280, 720)
        .run(state_machine)
}
```

## State Stack Behavior

### Updates

Only the **top state** receives `update()` calls:

```
Stack: [Menu, Gameplay, Pause]
       └─ Only Pause receives update()
```

### Drawing

All states receive `draw()` calls, from **bottom to top**:

```
Stack: [Menu, Gameplay, Pause]
       └─ Draw order: Menu → Gameplay → Pause
```

This allows:
- **Pause overlays** - Pause state draws on top of gameplay
- **Menu backgrounds** - Menu draws behind gameplay
- **Layered UI** - Multiple UI states can be visible

## Common Patterns

### Menu → Gameplay

```rust
impl State for MenuState {
    fn update(&mut self, ctx: &mut EngineContext, sm: &mut dyn StateMachineLike) -> Result<()> {
        if ctx.input().is_key_pressed(KeyCode::Enter) {
            sm.replace(Box::new(GameplayState::new())); // Replace menu with gameplay
        }
        Ok(())
    }
}
```

### Gameplay → Pause

```rust
impl State for GameplayState {
    fn update(&mut self, ctx: &mut EngineContext, sm: &mut dyn StateMachineLike) -> Result<()> {
        if ctx.input().is_key_pressed(KeyCode::KeyP) {
            sm.push(Box::new(PauseState)); // Push pause on top
        }
        Ok(())
    }
}

impl State for PauseState {
    fn draw(&mut self, renderer: &mut Renderer, frame: &mut Frame) -> Result<()> {
        // Draw semi-transparent overlay
        // Note: clear() with alpha doesn't blend properly - in a real game,
        // you'd draw a semi-transparent sprite overlay instead
        renderer.clear(frame, [0.0, 0.0, 0.0, 0.5])?; // Dark overlay
        // Draw "PAUSED" text
        Ok(())
    }
}
```

### Returning to Previous State

```rust
impl State for PauseState {
    fn update(&mut self, ctx: &mut EngineContext, sm: &mut dyn StateMachineLike) -> Result<()> {
        if ctx.input().is_key_pressed(KeyCode::KeyP) {
            sm.pop(); // Return to gameplay
        }
        Ok(())
    }
}
```

### Game Over Screen

```rust
impl State for GameplayState {
    fn update(&mut self, ctx: &mut EngineContext, sm: &mut dyn StateMachineLike) -> Result<()> {
        if self.player_health <= 0 {
            sm.replace(Box::new(GameOverState::new(self.score))); // Replace with game over
        }
        Ok(())
    }
}
```

## State Lifecycle

1. **State created** - `Box::new(MyState::new())`
2. **Pushed** - `sm.push(state)`
3. **`on_enter()` called** - State is entered
4. **`update()` called** - Every frame (only for top state)
5. **`draw()` called** - Every frame (for all states)
6. **Popped** - `sm.pop()`
7. **`on_exit()` called** - State is exited

## Best Practices

1. **Use `on_enter()` for initialization** - Load assets, reset state, etc.
2. **Use `on_exit()` for cleanup** - Save data, free resources, etc.
3. **Use `replace()` for menu transitions** - Don't keep menu in stack
4. **Use `push()` for overlays** - Pause, dialogs, etc.
5. **Handle empty stack** - Check `sm.is_empty()` or ensure at least one state

## Example

See `examples/state_demo/` for a complete example demonstrating:
- Menu state
- Gameplay state
- Pause state
- State transitions

Run it with:

```bash
cargo run -p state_demo
```
