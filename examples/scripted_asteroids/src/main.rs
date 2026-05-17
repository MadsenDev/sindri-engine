mod components;
mod game;

use anyhow::Result;
use game::ScriptedAsteroids;
use sindri::Engine;

fn main() -> Result<()> {
    Engine::new()
    .with_title("Sindri Engine · Scripted Asteroids")
    .with_size(1280, 720)
    .with_vsync(true)
    .run(ScriptedAsteroids::new()?)
}
