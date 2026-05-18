//! Scene/editor workflow showcase.
//!
//! Loads a `.sindri` scene file, restores entities into `World`, attaches Lua scripts,
//! and renders the resulting scene with hot reload enabled.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use sindri::{
    component::Component, Camera2D, Engine, EngineContext, Game, Name, PhysicsWorld, Scene,
    ScriptComponent, ScriptParams, ScriptRuntime, SpriteComponent, TextureHandle, Transform, Vec2,
    World,
};

struct EditorScene {
    runtime: ScriptRuntime,
    world: World,
    physics: PhysicsWorld,
    camera: Camera2D,
    white_texture: Option<TextureHandle>,
    loaded_scene_name: String,
}

#[derive(Clone, Copy)]
struct SceneSpriteSize(Vec2);

impl EditorScene {
    fn new() -> Result<Self> {
        Ok(Self {
            runtime: ScriptRuntime::new()?.with_hot_reload(true),
            world: World::new(),
            physics: PhysicsWorld::new(),
            camera: Camera2D::default(),
            white_texture: None,
            loaded_scene_name: String::new(),
        })
    }

    fn scene_path() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("scenes")
            .join("editor_scene.sindri")
    }

    fn resolve_script(path: &str) -> String {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(path)
            .to_string_lossy()
            .into_owned()
    }

    fn load_scene_into_world(&mut self) -> Result<()> {
        let scene = Scene::load(&Self::scene_path())?;
        self.loaded_scene_name = scene.name.clone();
        self.world = World::new();
        self.physics = PhysicsWorld::new();

        let white_texture = self
            .white_texture
            .ok_or_else(|| anyhow!("white texture not initialized"))?;

        let mut ids: Vec<_> = scene.entities.keys().copied().collect();
        ids.sort_unstable();

        for raw_id in ids {
            let scene_entity = &scene.entities[&raw_id];
            let entity = sindri::EntityId(raw_id as u32);
            self.world.restore_entity(entity);
            self.world
                .insert(entity, Name::new(scene_entity.name.clone()));

            for component in &scene_entity.components {
                match component {
                    Component::Transform(t) => {
                        self.world.insert(
                            entity,
                            Transform::new(Vec2::new(t.x, t.y))
                                .with_rotation(t.rotation)
                                .with_scale(Vec2::new(t.scale_x, t.scale_y)),
                        );
                    }
                    Component::Sprite(sprite_data) => {
                        let mut sprite = SpriteComponent::new(white_texture);
                        sprite.sprite.set_size_px(
                            Vec2::new(sprite_data.width, sprite_data.height),
                            Vec2::new(1.0, 1.0),
                        );
                        sprite.sprite.tint = sprite_data.color;
                        self.world.insert(entity, sprite);
                        self.world.insert(
                            entity,
                            SceneSpriteSize(Vec2::new(sprite_data.width, sprite_data.height)),
                        );
                    }
                    Component::Script(script) => {
                        self.world.insert(
                            entity,
                            ScriptComponent::default().with_script(
                                Self::resolve_script(&script.path),
                                ScriptParams::default(),
                            ),
                        );
                    }
                    _ => {}
                }
            }
        }

        self.sync_sprite_transforms();
        Ok(())
    }

    fn sync_sprite_transforms(&mut self) {
        let entities = self.world.entities();
        for entity in entities {
            let transform = self.world.get::<Transform>(entity).cloned();
            let base_size = self.world.get::<SceneSpriteSize>(entity).copied();
            if let (Some(transform), Some(base_size), Some(sprite)) = (
                transform,
                base_size,
                self.world.get_mut::<SpriteComponent>(entity),
            ) {
                sprite.sprite.transform.position = transform.position;
                sprite.sprite.transform.rotation = transform.rotation;
                sprite.sprite.transform.scale = Vec2::new(
                    base_size.0.x * transform.scale.x,
                    base_size.0.y * transform.scale.y,
                );
            }
        }
    }
}

impl Game for EditorScene {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        self.camera = ctx.screen_camera();
        self.white_texture = Some(ctx.renderer().load_texture_from_rgba(
            &[255, 255, 255, 255],
            1,
            1,
        )?);
        self.load_scene_into_world()
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let dt = ctx.delta_time().as_secs_f32();
        self.runtime
            .update(&mut self.world, &mut self.physics, ctx.input(), dt)?;
        self.sync_sprite_transforms();
        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let font = ctx.builtin_font(sindri::BuiltinFont::Ui).ok();
        let label = format!("{} · hot-reload Lua scene", self.loaded_scene_name);

        ctx.draw(|renderer, frame| {
            renderer.clear(frame, [0.025, 0.03, 0.04, 1.0])?;
            renderer.draw_world(frame, &self.world, &self.camera)?;

            if let Some(font) = font {
                renderer.draw_text(
                    frame,
                    &label,
                    font,
                    18.0,
                    Vec2::new(24.0, 32.0),
                    [0.82, 0.88, 0.96, 1.0],
                    &self.camera,
                )?;
            }

            Ok(())
        })
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("Sindri · Editor Scene")
        .with_size(1280, 720)
        .with_vsync(true)
        .run(EditorScene::new()?)
}
