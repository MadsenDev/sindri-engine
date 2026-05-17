use std::collections::VecDeque;

use anyhow::Result;

use crate::engine::EngineContext;

/// Trait for types that can manage state transitions.
/// This allows states to transition without direct access to StateMachine.
pub trait StateMachineLike {
    /// Push a new state onto the stack.
    fn push(&mut self, state: Box<dyn State>);

    /// Pop the current top state.
    fn pop(&mut self);

    /// Replace the current top state.
    fn replace(&mut self, state: Box<dyn State>);
}

/// A game state that can be managed by a StateMachine.
pub trait State {
    /// Called when this state is entered (pushed onto the stack).
    fn on_enter(&mut self, _ctx: &mut EngineContext) -> Result<()> {
        Ok(())
    }

    /// Called when this state is exited (popped from the stack).
    fn on_exit(&mut self, _ctx: &mut EngineContext) -> Result<()> {
        Ok(())
    }

    /// Update this state. Called every frame.
    /// The state machine is provided so states can transition to other states.
    fn update(
        &mut self,
        ctx: &mut EngineContext,
        state_machine: &mut dyn StateMachineLike,
    ) -> Result<()>;

    /// Draw this state. Called every frame after update.
    /// The frame is already begun by StateMachine, so states should only draw to it.
    fn draw(
        &mut self,
        renderer: &mut crate::render::Renderer,
        frame: &mut crate::render::Frame,
    ) -> Result<()>;
}

/// Internal helper to allow states to queue transitions without borrow conflicts.
struct StateTransitionHelper<'a> {
    pending_push: &'a mut Option<Box<dyn State>>,
    pending_pop: &'a mut bool,
    pending_replace: &'a mut Option<Box<dyn State>>,
}

impl<'a> StateMachineLike for StateTransitionHelper<'a> {
    fn push(&mut self, state: Box<dyn State>) {
        *self.pending_push = Some(state);
    }

    fn pop(&mut self) {
        *self.pending_pop = true;
    }

    fn replace(&mut self, state: Box<dyn State>) {
        *self.pending_replace = Some(state);
    }
}

/// Manages a stack of game states.
///
/// States are drawn from bottom to top (oldest to newest).
/// Only the top state receives update calls.
/// Pending transitions are applied at the start of the next update.
///
/// # Example
///
/// ```rust,no_run
/// use sindri::{StateMachine, State, StateMachineLike, EngineContext, Renderer, Frame, KeyCode};
/// use anyhow::Result;
///
/// struct MenuState;
///
/// impl State for MenuState {
///     fn update(&mut self, ctx: &mut EngineContext, sm: &mut dyn StateMachineLike) -> Result<()> {
///         if ctx.input().is_key_pressed(KeyCode::Enter) {
///             sm.push(Box::new(GameplayState::new()));
///         }
///         Ok(())
///     }
///     
///     fn draw(&mut self, renderer: &mut Renderer, frame: &mut Frame) -> Result<()> {
///         renderer.clear(frame, [0.1, 0.1, 0.2, 1.0])?;
///         // Draw menu
///         Ok(())
///     }
/// }
///
/// struct GameplayState;
/// impl GameplayState {
///     fn new() -> Self { Self }
/// }
/// impl State for GameplayState {
///     fn update(&mut self, _ctx: &mut EngineContext, _sm: &mut dyn StateMachineLike) -> Result<()> {
///         Ok(())
///     }
///     fn draw(&mut self, _renderer: &mut Renderer, _frame: &mut Frame) -> Result<()> {
///         Ok(())
///     }
/// }
/// ```
pub struct StateMachine {
    states: VecDeque<Box<dyn State>>,
    pending_push: Option<Box<dyn State>>,
    pending_pop: bool,
    pending_replace: Option<Box<dyn State>>,
}

impl StateMachine {
    /// Create a new empty state machine.
    pub fn new() -> Self {
        Self {
            states: VecDeque::new(),
            pending_push: None,
            pending_pop: false,
            pending_replace: None,
        }
    }

    /// Create a state machine with an initial state.
    pub fn with_initial_state(initial: Box<dyn State>) -> Self {
        let mut sm = Self::new();
        // Note: on_enter will be called in init() when the engine starts
        sm.states.push_back(initial);
        sm
    }

    /// Push a new state onto the stack.
    /// The current top state will be paused (no more updates), but will still be drawn.
    /// The new state will be entered and will receive updates.
    ///
    /// # Note
    /// State transitions are applied at the start of the next update.
    pub fn push(&mut self, state: Box<dyn State>) {
        self.pending_push = Some(state);
    }

    /// Pop the current top state.
    /// The previous state (if any) will resume receiving updates.
    ///
    /// # Note
    /// State transitions are applied at the start of the next update.
    pub fn pop(&mut self) {
        self.pending_pop = true;
    }

    /// Replace the current top state with a new state.
    /// Equivalent to `pop()` followed by `push()`.
    ///
    /// # Note
    /// State transitions are applied at the start of the next update.
    pub fn replace(&mut self, state: Box<dyn State>) {
        self.pending_replace = Some(state);
    }

    /// Check if the state machine is empty.
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    /// Get the number of states in the stack.
    pub fn len(&self) -> usize {
        self.states.len()
    }

    /// Apply pending state transitions.
    /// Called automatically by the engine, but can be called manually if needed.
    pub fn apply_transitions(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Handle replace first (it's a pop + push)
        if let Some(mut new_state) = self.pending_replace.take() {
            if let Some(mut old_state) = self.states.pop_back() {
                old_state.on_exit(ctx)?;
            }
            new_state.on_enter(ctx)?;
            self.states.push_back(new_state);
            return Ok(());
        }

        // Handle pop
        if self.pending_pop {
            self.pending_pop = false;
            if let Some(mut state) = self.states.pop_back() {
                state.on_exit(ctx)?;
            }
        }

        // Handle push
        if let Some(mut new_state) = self.pending_push.take() {
            new_state.on_enter(ctx)?;
            self.states.push_back(new_state);
        }

        Ok(())
    }

    /// Update the top state (if any).
    /// This method handles the borrow checker issues internally.
    pub fn update_top(&mut self, ctx: &mut EngineContext) -> Result<()> {
        if let Some(state) = self.states.back_mut() {
            // Create a helper that can queue transitions
            let mut helper = StateTransitionHelper {
                pending_push: &mut self.pending_push,
                pending_pop: &mut self.pending_pop,
                pending_replace: &mut self.pending_replace,
            };
            state.update(ctx, &mut helper)?;
        }
        Ok(())
    }

    /// Get all states (for drawing, from bottom to top).
    /// Draws states in order (oldest to newest).
    /// The frame should already be begun by the caller.
    pub fn draw_all(
        &mut self,
        renderer: &mut crate::render::Renderer,
        frame: &mut crate::render::Frame,
    ) -> Result<()> {
        for state in self.states.iter_mut() {
            state.draw(renderer, frame)?;
        }
        Ok(())
    }

    /// Get all states (immutable, for inspection).
    pub fn states(&self) -> &VecDeque<Box<dyn State>> {
        &self.states
    }

    /// Call on_enter for the top state (used for initial state initialization).
    pub fn init_top_state(&mut self, ctx: &mut EngineContext) -> Result<()> {
        if let Some(state) = self.states.back_mut() {
            state.on_enter(ctx)?;
        }
        Ok(())
    }
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}
