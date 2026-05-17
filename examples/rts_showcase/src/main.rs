mod game;

use anyhow::Result;
use game::RtsShowcase;
use sindri::Engine;

fn main() -> Result<()> {
    Engine::new()
        .with_title("Sindri Engine · RTS Showcase")
        .with_size(1280, 720)
        .with_vsync(true)
        .run(RtsShowcase::new())
}