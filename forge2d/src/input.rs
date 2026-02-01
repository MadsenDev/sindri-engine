use std::collections::{HashMap, HashSet};

use winit::{
    event::{ElementState, KeyEvent, MouseButton},
    keyboard::{KeyCode, PhysicalKey},
};

/// Tracks keyboard and mouse state across frames.
pub struct InputState {
    keys_down: HashSet<KeyCode>,
    keys_pressed: HashSet<KeyCode>,
    keys_released: HashSet<KeyCode>,
    pending_keys_pressed: HashSet<KeyCode>,
    pending_keys_released: HashSet<KeyCode>,

    mouse_x: f32,
    mouse_y: f32,
    mouse_down: [bool; 8],
    mouse_pressed: [bool; 8],
    mouse_released: [bool; 8],
    pending_mouse_pressed: [bool; 8],
    pending_mouse_released: [bool; 8],
}

impl InputState {
    pub fn new() -> Self {
        Self {
            keys_down: HashSet::new(),
            keys_pressed: HashSet::new(),
            keys_released: HashSet::new(),
            pending_keys_pressed: HashSet::new(),
            pending_keys_released: HashSet::new(),
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_down: [false; 8],
            mouse_pressed: [false; 8],
            mouse_released: [false; 8],
            pending_mouse_pressed: [false; 8],
            pending_mouse_released: [false; 8],
        }
    }

    /// Advance per-frame pressed/released flags.
    ///
    /// This snapshots any pending inputs gathered since the last frame boundary.
    pub fn begin_frame(&mut self) {
        self.keys_pressed = std::mem::take(&mut self.pending_keys_pressed);
        self.keys_released = std::mem::take(&mut self.pending_keys_released);
        self.mouse_pressed = self.pending_mouse_pressed;
        self.mouse_released = self.pending_mouse_released;
        self.pending_mouse_pressed.fill(false);
        self.pending_mouse_released.fill(false);
    }

    /// Handle a keyboard input event from winit.
    pub fn handle_key(&mut self, event: &KeyEvent) {
        let PhysicalKey::Code(keycode) = event.physical_key else {
            return;
        };

        match event.state {
            ElementState::Pressed => {
                if !self.keys_down.contains(&keycode) {
                    self.pending_keys_pressed.insert(keycode);
                }
                self.keys_down.insert(keycode);
            }
            ElementState::Released => {
                self.keys_down.remove(&keycode);
                self.pending_keys_released.insert(keycode);
            }
        }
    }

    /// Handle a mouse button input event from winit.
    pub fn handle_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        if let Some(idx) = mouse_button_index(button) {
            match state {
                ElementState::Pressed => {
                    if !self.mouse_down[idx] {
                        self.pending_mouse_pressed[idx] = true;
                    }
                    self.mouse_down[idx] = true;
                }
                ElementState::Released => {
                    self.mouse_down[idx] = false;
                    self.pending_mouse_released[idx] = true;
                }
            }
        }
    }

    /// Handle mouse cursor movement from winit.
    pub fn handle_cursor_moved(&mut self, x: f64, y: f64) {
        self.mouse_x = x as f32;
        self.mouse_y = y as f32;
    }

    /// Returns true if the key is currently held down.
    pub fn is_key_down(&self, key: KeyCode) -> bool {
        self.keys_down.contains(&key)
    }

    /// Returns true if the key was pressed this frame.
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }

    /// Returns true if the key was released this frame.
    pub fn is_key_released(&self, key: KeyCode) -> bool {
        self.keys_released.contains(&key)
    }

    /// Returns true if the mouse button is currently held down.
    pub fn is_mouse_down(&self, button: MouseButton) -> bool {
        mouse_button_index(button)
            .map(|idx| self.mouse_down[idx])
            .unwrap_or(false)
    }

    /// Returns true if the mouse button was pressed this frame.
    pub fn is_mouse_pressed(&self, button: MouseButton) -> bool {
        mouse_button_index(button)
            .map(|idx| self.mouse_pressed[idx])
            .unwrap_or(false)
    }

    /// Returns true if the mouse button was released this frame.
    pub fn is_mouse_released(&self, button: MouseButton) -> bool {
        mouse_button_index(button)
            .map(|idx| self.mouse_released[idx])
            .unwrap_or(false)
    }

    /// Current mouse cursor position in logical pixels.
    pub fn mouse_position(&self) -> (f32, f32) {
        (self.mouse_x, self.mouse_y)
    }

    /// Current mouse cursor position as a Vec2.
    pub fn mouse_position_vec2(&self) -> crate::math::Vec2 {
        crate::math::Vec2::new(self.mouse_x, self.mouse_y)
    }

    /// Current mouse cursor position in screen pixels (surface coordinates).
    pub fn mouse_screen_pixels(&self) -> (f32, f32) {
        // For now, same as logical pixels. Could be enhanced to track DPI scaling separately.
        (self.mouse_x, self.mouse_y)
    }
}

/// A logical input action (e.g. "move_left", "jump").
///
/// This is a lightweight, data-driven layer on top of `InputState`.
/// Game code binds one or more physical inputs (keys/mouse buttons)
/// to each action and then queries the action state instead of
/// referencing key codes directly.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ActionId(pub String);

impl ActionId {
    /// Create a new action identifier from any string-like value.
    pub fn new(name: impl Into<String>) -> Self {
        ActionId(name.into())
    }
}

/// A physical button that can be bound to an action or axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Button {
    Key(KeyCode),
    Mouse(MouseButton),
}

impl Button {
    fn is_down(self, input: &InputState) -> bool {
        match self {
            Button::Key(k) => input.is_key_down(k),
            Button::Mouse(b) => input.is_mouse_down(b),
        }
    }

    fn is_pressed(self, input: &InputState) -> bool {
        match self {
            Button::Key(k) => input.is_key_pressed(k),
            Button::Mouse(b) => input.is_mouse_pressed(b),
        }
    }
}

/// A one-dimensional axis binding (e.g. -1..1 horizontal movement).
#[derive(Clone, Debug)]
pub struct AxisBinding {
    /// Buttons contributing negative direction (e.g. A, Left).
    pub negative: Vec<Button>,
    /// Buttons contributing positive direction (e.g. D, Right).
    pub positive: Vec<Button>,
}

impl AxisBinding {
    /// Create a new axis binding from negative and positive button sets.
    pub fn new(negative: Vec<Button>, positive: Vec<Button>) -> Self {
        Self { negative, positive }
    }
}

/// High-level input mapping from actions/axes to physical inputs.
///
/// This is intentionally simple and game-agnostic. Games are free to
/// store an `InputMap` in their own state, configure bindings in
/// `init()`, and then query actions/axes during `update()`.
#[derive(Clone, Debug)]
pub struct InputMap {
    actions: HashMap<ActionId, Vec<Button>>,
    axes: HashMap<ActionId, AxisBinding>,
}

impl InputMap {
    /// Create an empty input map.
    pub fn new() -> Self {
        Self {
            actions: HashMap::new(),
            axes: HashMap::new(),
        }
    }

    /// Bind a key to an action.
    pub fn bind_key(&mut self, action: ActionId, key: KeyCode) {
        self.actions
            .entry(action)
            .or_default()
            .push(Button::Key(key));
    }

    /// Bind a mouse button to an action.
    pub fn bind_mouse_button(&mut self, action: ActionId, button: MouseButton) {
        self.actions
            .entry(action)
            .or_default()
            .push(Button::Mouse(button));
    }

    /// Define or replace an axis binding.
    pub fn set_axis(&mut self, axis: ActionId, binding: AxisBinding) {
        self.axes.insert(axis, binding);
    }

    /// Check if an action is currently held down.
    pub fn action_down(&self, input: &InputState, action: &ActionId) -> bool {
        self.actions
            .get(action)
            .map(|buttons| buttons.iter().any(|&b| b.is_down(input)))
            .unwrap_or(false)
    }

    /// Check if an action was pressed this frame.
    pub fn action_pressed(&self, input: &InputState, action: &ActionId) -> bool {
        self.actions
            .get(action)
            .map(|buttons| buttons.iter().any(|&b| b.is_pressed(input)))
            .unwrap_or(false)
    }

    /// Get the value of an axis in the range [-1.0, 1.0].
    ///
    /// Negative buttons contribute -1.0, positive buttons +1.0.
    /// If both sides are pressed, they cancel out.
    pub fn axis(&self, input: &InputState, axis: &ActionId) -> f32 {
        if let Some(binding) = self.axes.get(axis) {
            let mut value = 0.0;
            if binding.negative.iter().any(|&b| b.is_down(input)) {
                value -= 1.0;
            }
            if binding.positive.iter().any(|&b| b.is_down(input)) {
                value += 1.0;
            }
            value
        } else {
            0.0
        }
    }
}

fn mouse_button_index(button: MouseButton) -> Option<usize> {
    match button {
        MouseButton::Left => Some(0),
        MouseButton::Right => Some(1),
        MouseButton::Middle => Some(2),
        MouseButton::Back => Some(3),
        MouseButton::Forward => Some(4),
        MouseButton::Other(raw) => {
            let idx = raw as usize;
            let mapped = 5 + idx; // Reserve 0-4 for standard buttons
            (mapped < 8).then_some(mapped)
        }
    }
}
