use anyhow::Result;
use sindri::{
    ActionId, AxisBinding, Button, Engine, EngineContext, Frame, InputMap, KeyCode, State,
    StateMachine, StateMachineLike, Vec2,
};

/// Menu state - the initial state
struct MenuState {
    time_entered: f32,
}

impl MenuState {
    fn new() -> Self {
        Self { time_entered: 0.0 }
    }
}

impl State for MenuState {
    fn on_enter(&mut self, _ctx: &mut EngineContext) -> Result<()> {
        println!("Entered MenuState");
        self.time_entered = 0.0;
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext, sm: &mut dyn StateMachineLike) -> Result<()> {
        let dt = ctx.delta_time().as_secs_f32();
        self.time_entered += dt;

        // Press Enter to start game
        if ctx.input().is_key_pressed(KeyCode::Enter) {
            println!("Transitioning to GameplayState");
            sm.push(Box::new(GameplayState::new()));
        }

        // Press Escape to exit
        if ctx.input().is_key_pressed(KeyCode::Escape) {
            ctx.request_exit();
        }

        Ok(())
    }

    fn draw(&mut self, renderer: &mut sindri::Renderer, frame: &mut sindri::Frame) -> Result<()> {
        // Draw menu background (dark blue)
        renderer.clear(frame, [0.05, 0.05, 0.15, 1.0])?;

        // In a real game, you'd draw menu text/buttons here
        // For now, we'll just show a simple background

        Ok(())
    }
}

/// Gameplay state - the main game
struct GameplayState {
    position: Vec2,
    speed: f32,
    input_map: InputMap,
    axis_horizontal: ActionId,
    axis_vertical: ActionId,
}

impl GameplayState {
    fn new() -> Self {
        let mut input_map = InputMap::new();

        let axis_horizontal = ActionId::new("move_horizontal");
        let axis_vertical = ActionId::new("move_vertical");

        // Horizontal: A/Left = -1, D/Right = +1
        input_map.set_axis(
            axis_horizontal.clone(),
            AxisBinding::new(
                vec![Button::Key(KeyCode::KeyA), Button::Key(KeyCode::ArrowLeft)],
                vec![Button::Key(KeyCode::KeyD), Button::Key(KeyCode::ArrowRight)],
            ),
        );

        // Vertical: W/Up = -1 (up), S/Down = +1 (down)
        input_map.set_axis(
            axis_vertical.clone(),
            AxisBinding::new(
                vec![Button::Key(KeyCode::KeyW), Button::Key(KeyCode::ArrowUp)],
                vec![Button::Key(KeyCode::KeyS), Button::Key(KeyCode::ArrowDown)],
            ),
        );

        Self {
            position: Vec2::new(400.0, 300.0),
            speed: 200.0,
            input_map,
            axis_horizontal,
            axis_vertical,
        }
    }
}

impl State for GameplayState {
    fn on_enter(&mut self, _ctx: &mut EngineContext) -> Result<()> {
        println!("Entered GameplayState");
        Ok(())
    }

    fn on_exit(&mut self, _ctx: &mut EngineContext) -> Result<()> {
        println!("Exited GameplayState");
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext, sm: &mut dyn StateMachineLike) -> Result<()> {
        let dt = ctx.delta_time().as_secs_f32();
        let input = ctx.input();

        // Movement
        let horizontal = self.input_map.axis(input, &self.axis_horizontal);
        let vertical = self.input_map.axis(input, &self.axis_vertical);
        let move_dir = Vec2::new(horizontal, vertical);
        if move_dir.length_squared() > 0.0 {
            let dir = move_dir.normalized();
            self.position += dir * self.speed * dt;
        }

        // Press P to pause
        if input.is_key_pressed(KeyCode::KeyP) {
            println!("Pausing game");
            sm.push(Box::new(PauseState));
        }

        // Press Escape to return to menu
        if input.is_key_pressed(KeyCode::Escape) {
            println!("Returning to menu");
            sm.pop(); // Pop this state to return to menu
        }

        Ok(())
    }

    fn draw(&mut self, renderer: &mut sindri::Renderer, frame: &mut sindri::Frame) -> Result<()> {
        // Draw gameplay background (lighter blue)
        renderer.clear(frame, [0.1, 0.1, 0.2, 1.0])?;

        // In a real game, you'd draw game objects here
        // For now, we'll just show a simple background

        Ok(())
    }
}

/// Pause state - overlays on top of gameplay
struct PauseState;

impl State for PauseState {
    fn on_enter(&mut self, _ctx: &mut EngineContext) -> Result<()> {
        println!("Entered PauseState");
        Ok(())
    }

    fn on_exit(&mut self, _ctx: &mut EngineContext) -> Result<()> {
        println!("Exited PauseState");
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext, sm: &mut dyn StateMachineLike) -> Result<()> {
        // Press P again to unpause
        if ctx.input().is_key_pressed(KeyCode::KeyP) {
            println!("Unpausing game");
            sm.pop(); // Pop pause state to return to gameplay
        }

        // Press Escape to return to menu
        if ctx.input().is_key_pressed(KeyCode::Escape) {
            println!("Returning to menu from pause");
            sm.pop(); // Pop pause
            sm.pop(); // Pop gameplay
        }

        Ok(())
    }

    fn draw(&mut self, renderer: &mut sindri::Renderer, frame: &mut sindri::Frame) -> Result<()> {
        // Draw a dark overlay (semi-transparent)
        // Note: clear() with alpha doesn't blend properly, but this demonstrates the concept
        // In a real game, you'd draw a semi-transparent sprite overlay
        renderer.clear(frame, [0.0, 0.0, 0.0, 0.5])?;

        // In a real game, you'd draw "PAUSED" text here

        Ok(())
    }
}

fn main() -> Result<()> {
    // Create state machine with initial menu state
    let state_machine = StateMachine::with_initial_state(Box::new(MenuState::new()));

    Engine::new()
        .with_title("Sindri State Demo")
        .with_size(1024, 768)
        .with_vsync(true)
        .run(state_machine)
}
