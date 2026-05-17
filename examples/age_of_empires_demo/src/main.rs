mod entities;
mod game;
mod resources;
mod systems;
mod ui;

use anyhow::Result;
use game::AgeOfEmpiresDemo;
use sindri::Engine;

fn main() -> Result<()> {
    Engine::new()
        .with_title("Age of Empires Demo - Sindri")
        .with_size(1280, 720)
        .with_vsync(true)
        .run(AgeOfEmpiresDemo::new())
}
