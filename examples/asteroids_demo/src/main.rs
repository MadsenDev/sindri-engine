mod entities;
mod game;

use anyhow::Result;
use game::AsteroidsGame;
use sindri::Engine;

fn main() -> Result<()> {
    Engine::new()
        .with_title("Asteroids - Sindri")
        .with_size(1280, 720)
        .with_vsync(true)
        .run(AsteroidsGame::new())
}
